// Copyright (c) 2016-2022 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Pkg storage backend variant which uses S3 (or an API compatible clone) for
//! hart storage.
//!
//! Has been tested against AWS S3.
//!
//! All packages are stored in a single bucket, using the fully qualified
//! package ident followed by the harfile name.hart as the key
//!
//! # Configuration
//!
//! Currently the S3Handler must be configured with both an access key
//! ID and a secret access key.
use std::{fmt::Display,
          fs::File,
          io::{self,
               BufRead,
               BufReader,
               Read,
               Write},
          path::{Path,
                 PathBuf},
          pin::Pin,
          str::FromStr,
          time::Instant};

use aws_sdk_s3::{config::{Credentials,
                          Region},
                 primitives::ByteStream,
                 types::{CompletedMultipartUpload,
                         CompletedPart},
                 Client as S3Client};

use bytes::Bytes;
use futures::stream::Stream;

use super::metrics::Counter;
use crate::{bldr_core::metrics::CounterMetric,
            config::{S3Backend,
                     S3Cfg},
            hab_core::package::{PackageArchive,
                                PackageIdent,
                                PackageTarget},
            server::error::{Error,
                            Result}};

// This const is equal to 6MB which is slightly above
// the minimum limit for a multipart upload request
// to s3. Any package over 6MB on upload will use this api
const MINLIMIT: usize = 10240 * 1024;

pub struct S3Handler {
    client: S3Client,
    bucket: String,
}

impl S3Handler {
    // The S3 Handler struct contains all of the credential
    // and target information that we should need to perfom
    // any backend operations
    pub fn new(config: S3Cfg) -> Self {
        let (region_name, endpoint_url, force_path_style) = match config.backend {
            S3Backend::Minio => ("minio_s3".to_owned(), Some(config.endpoint.to_string()), true),
            S3Backend::Aws => (String::from_str(config.endpoint.as_str()).unwrap(), None, false),
        };

        let creds = Credentials::new(config.key_id.clone(),
                                     config.secret_key.clone(),
                                     None,
                                     None,
                                     "static");

        // Create a specific Region instance instead of using RegionProviderChain
        let region = Region::new(region_name);

        // Create configuration synchronously
        let mut s3_conf_builder = aws_sdk_s3::config::Builder::new().behavior_version_latest()
                                                                    .region(region)
                                                                    .credentials_provider(creds);

        // Apply endpoint URL and path style settings if needed
        if let Some(url) = endpoint_url {
            s3_conf_builder = s3_conf_builder.endpoint_url(url);
        }
        if force_path_style {
            s3_conf_builder = s3_conf_builder.force_path_style(true);
        }

        let s3_conf = s3_conf_builder.build();
        let client = S3Client::from_conf(s3_conf);
        let bucket = config.bucket_name;

        S3Handler { client, bucket }
    }

    // This function checks whether or not the
    // configured bucket exists in the configured
    // backend.
    #[allow(dead_code)]
    async fn bucket_exists(&self) -> Result<bool> {
        let artifactbucket = self.bucket.to_owned();
        match self.client.list_buckets().send().await {
            Ok(bucket_list) => {
                match bucket_list.buckets {
                    Some(buckets) => {
                        Ok(buckets.iter()
                                  .any(|x| x.name.clone().unwrap() == artifactbucket))
                    }
                    None => Ok(false),
                }
            }
            Err(e) => {
                debug!("{:?}", e);
                Err(e.into())
            }
        }
    }

    // This function checks whether an uploaded file
    // exists in the configured s3 bucket. It should
    // only get called from within an upload future.
    async fn object_exists(&self, object_key: &str) -> Result<()> {
        let request = self.client
                          .head_object()
                          .bucket(self.bucket.clone())
                          .key(object_key.to_string());

        match request.send().await {
            Ok(object) => {
                info!("Verified {} was written to minio!", object_key);
                debug!("Head Object check returned: {:?}", object);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    #[allow(dead_code)]
    pub async fn create_bucket(&self) -> Result<()> {
        let request = self.client.create_bucket().bucket(self.bucket.clone());

        match self.bucket_exists().await {
            Ok(_) => Ok(()),
            Err(_) => {
                match request.send().await {
                    Ok(_response) => Ok(()),
                    Err(e) => {
                        debug!("{:?}", e);
                        Err(e.into())
                    }
                }
            }
        }
    }

    pub async fn upload(&self,
                        hart_path: &Path,
                        ident: &PackageIdent,
                        target: PackageTarget)
                        -> Result<()> {
        Counter::UploadRequests.increment();
        let key = s3_key(ident, target)?;
        let file = File::open(hart_path).map_err(Error::IO)?;

        info!("S3Handler::upload request started for s3_key: {}", key);

        let size = file.metadata().unwrap().len() as usize;
        let fqpi = hart_path.to_str().unwrap();

        if size < MINLIMIT {
            self.single_upload(&key, file, &fqpi).await?;
        } else {
            self.multipart_upload(&key, file, &fqpi).await?;
        }
        self.object_exists(&key).await
    }

    pub async fn delete(&self, ident: &PackageIdent, target: PackageTarget) -> Result<()> {
        Counter::DeleteRequests.increment();
        let key = s3_key(ident, target)?;

        info!("S3Handler::delete request started for s3_key: {}", key);

        let request = self.client
                          .delete_object()
                          .bucket(self.bucket.clone())
                          .key(key.clone());

        match request.send().await {
            Ok(_) => {
                info!("S3Handler::delete succeeded for s3_key: {}", key);
                Ok(())
            }
            Err(e) => {
                Counter::DeleteFailures.increment();
                error!("S3Handler::delete failed for s3_key {}: ({:?})", key, e);
                Err(e.into())
            }
        }
    }

    /// Streams a package's bytes directly from S3, rather than buffering the whole object
    /// in memory or on local disk first. Returns the object's content length (when S3 reports
    /// one) along with a stream of chunks as they arrive, so that callers (e.g. the HTTP
    /// download handler) can forward each chunk to their own client as soon as it's received,
    /// instead of waiting for the entire, potentially very large, artifact to be fetched.
    pub async fn download_stream(
        &self,
        ident: &PackageIdent,
        target: PackageTarget)
        -> Result<(Option<i64>,
                   Pin<Box<dyn Stream<Item = std::result::Result<Bytes, io::Error>> + Send>>)> {
        Counter::DownloadRequests.increment();
        let key = s3_key(ident, target)?;
        let request = self.client
                          .get_object()
                          .bucket(self.bucket.clone())
                          .key(key);

        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to retrieve object from S3, ident={}: {:?}",
                      ident, e);
                return Err(e.into());
            }
        };

        let content_length = response.content_length;

        // `aws_smithy_types::byte_stream::ByteStream` does not itself implement
        // `futures::Stream` -- it only exposes an async `next()` method (plus an unrelated
        // inherent `map()` that transforms the underlying `SdkBody`, not stream items). Adapt it
        // into a real `futures::Stream` by driving that `next()` method through
        // `futures::stream::unfold`, converting each streaming error into an `io::Error` the
        // same way the rest of this module does.
        let stream = futures::stream::unfold(response.body, |mut body| {
            async move {
                match body.next().await {
                    Some(Ok(bytes)) => Some((Ok(bytes), body)),
                    Some(Err(e)) => Some((Err(io::Error::other(e.to_string())), body)),
                    None => None,
                }
            }
        });
        Ok((content_length, Box::pin(stream)))
    }

    pub async fn download(&self,
                          loc: &Path,
                          ident: &PackageIdent,
                          target: PackageTarget)
                          -> Result<PackageArchive> {
        Counter::DownloadRequests.increment();
        let key = s3_key(ident, target)?;
        let request = self.client
                          .get_object()
                          .bucket(self.bucket.clone())
                          .key(key);

        let payload = request.send().await;
        let body = match payload {
            Ok(response) => response.body,
            Err(e) => {
                warn!("Failed to retrieve object from S3, ident={}: {:?}",
                      ident, e);
                return Err(e.into());
            }
        };

        match write_archive(loc, body).await {
            Ok(result) => Ok(result),
            Err(e) => {
                warn!("Unable to write file {:?} to archive, err={:?}", loc, e);
                Err(e)
            }
        }
    }

    pub async fn size_of(&self, ident: &PackageIdent, target: PackageTarget) -> Result<i64> {
        Counter::SizeRequests.increment();
        let key = s3_key(ident, target)?;
        let request = self.client
                          .head_object()
                          .bucket(self.bucket.clone())
                          .key(key);

        let payload = request.send().await;
        match payload {
            Ok(response) => {
                match response.content_length {
                    Some(len) => Ok(len),
                    None => {
                        warn!("S3 object missing content length for ident={}", ident);
                        Err(Error::IO(std::io::Error::other("S3 object missing content length")))
                    }
                }
            }
            Err(e) => {
                warn!("Failed to retrieve object metadata from S3, ident={}: {:?}",
                      ident, e);
                Err(e.into())
            }
        }
    }

    async fn single_upload<P: Into<PathBuf> + Display>(&self,
                                                       key: &str,
                                                       hart: File,
                                                       path_attr: &P)
                                                       -> Result<()> {
        Counter::SingleUploadRequests.increment();
        let start_time = Instant::now();
        let mut reader = BufReader::new(hart);
        let mut object: Vec<u8> = Vec::new();
        let bucket = self.bucket.clone();
        let _complete = reader.read_to_end(&mut object).map_err(Error::IO);

        let request = self.client
                          .put_object()
                          .key(key.to_string())
                          .bucket(bucket)
                          .body(ByteStream::from(object));

        match request.send().await {
            Ok(_) => {
                info!("Upload completed for {} (in {} sec):",
                      path_attr,
                      start_time.elapsed().as_secs_f64());
                Ok(())
            }
            Err(e) => {
                Counter::UploadFailures.increment();
                warn!("Upload failed for {}: ({:?})", path_attr, e);
                Err(e.into())
            }
        }
    }

    async fn multipart_upload<P: Into<PathBuf> + Display>(&self,
                                                          key: &str,
                                                          hart: File,
                                                          path_attr: &P)
                                                          -> Result<()> {
        Counter::MultipartUploadRequests.increment();
        let start_time = Instant::now();
        let mut p: Vec<CompletedPart> = Vec::new();
        let mprequest = self.client
                            .create_multipart_upload()
                            .key(key.to_string())
                            .bucket(self.bucket.clone());

        match mprequest.send().await {
            Ok(output) => {
                let mut reader = BufReader::with_capacity(MINLIMIT, hart);
                let mut part_num: i64 = 0;
                let mut should_break = false;
                loop {
                    let length;
                    {
                        let buffer = reader.fill_buf().map_err(Error::IO)?;
                        length = buffer.len();
                        if length < MINLIMIT {
                            should_break = true;
                        }
                        part_num += 1;

                        let request = self.client
                                          .upload_part()
                                          .key(key.to_string())
                                          .bucket(self.bucket.clone())
                                          .upload_id(output.upload_id.clone().unwrap())
                                          .body(ByteStream::from(buffer.to_vec()))
                                          .part_number(part_num as i32);

                        match request.send().await {
                            Ok(upo) => {
                                p.push(CompletedPart::builder().set_e_tag(upo.e_tag)
                                                               .part_number(part_num as i32)
                                                               .build());
                            }
                            Err(e) => {
                                debug!("{:?}", e);
                                return Err(e.into());
                            }
                        }
                    }
                    reader.consume(length);
                    if should_break {
                        break;
                    }
                }

                let completion =
                    self.client
                        .complete_multipart_upload()
                        .key(key.to_string())
                        .bucket(self.bucket.clone())
                        .upload_id(output.upload_id.unwrap())
                        .multipart_upload(CompletedMultipartUpload::builder().set_parts(Some(p))
                                                                             .build());

                match completion.send().await {
                    Ok(_) => {
                        info!("Upload completed for {} (in {} sec):",
                              path_attr,
                              start_time.elapsed().as_secs_f64());
                        Ok(())
                    }
                    Err(e) => {
                        warn!("Upload failed for {}: ({:?})", path_attr, e);
                        Err(e.into())
                    }
                }
            }
            Err(e) => {
                debug!("{:?}", e);
                Err(e.into())
            }
        }
    }
}

// Helper function for programmatic creation of
// the s3 object key
fn s3_key(ident: &PackageIdent, target: PackageTarget) -> Result<String> {
    // Calling this method first ensures that the ident is fully qualified and the correct errors
    // are returned in case of failure
    let hart_name = ident.archive_name_with_target(target)
                         .map_err(Error::HabitatCore)?;

    Ok(format!("{}/{}/{}",
               ident.iter().collect::<Vec<&str>>().join("/"),
               target.iter().collect::<Vec<&str>>().join("/"),
               hart_name))
}

async fn write_archive(filename: &Path, body: ByteStream) -> Result<PackageArchive> {
    // TODO This is a blocking call, used in async functions
    let mut file = match File::create(filename) {
        Ok(f) => f,
        Err(e) => {
            warn!("Unable to create archive file for {:?}, err={:?}",
                  filename, e);
            return Err(Error::IO(e));
        }
    };

    match body.collect().await {
        Ok(aggregated) => {
            let bytes = aggregated.into_bytes();
            if let Err(e) = file.write_all(&bytes) {
                warn!("Unable to write archive for {:?}, err={:?}", filename, e);
                return Err(Error::IO(e));
            }
        }
        Err(e) => {
            warn!("Failed to read S3 body stream, err={:?}", e);
            return Err(Error::IO(std::io::Error::other(e.to_string())));
        }
    }

    Ok(PackageArchive::new(filename)?)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::hab_core;

    #[test]
    fn s3_key_fully_qualified_ident() {
        let ident =
            PackageIdent::from_str("bend-sinister/the-other-way/1.0.0/20180701122201").unwrap();
        let target = PackageTarget::from_str("x86_64-linux").unwrap();

        assert_eq!(format!("{}/{}",
                           "bend-sinister/the-other-way/1.0.0/20180701122201/x86_64/linux",
                           "bend-sinister-the-other-way-1.0.0-20180701122201-x86_64-linux.hart"),
                   s3_key(&ident, target).unwrap());
    }

    #[test]
    fn s3_key_fuzzy_ident() {
        let ident = PackageIdent::from_str("acme/not-enough").unwrap();
        let target = PackageTarget::from_str("x86_64-linux").unwrap();

        match s3_key(&ident, target) {
            Err(Error::HabitatCore(hab_core::Error::FullyQualifiedPackageIdentRequired(i))) => {
                assert_eq!("acme/not-enough", i)
            }
            Err(e) => panic!("Wrong expected error, found={:?}", e),
            Ok(s) => panic!("Should not have computed a result, returned={}", s),
        }
    }
}

// These tests exercise `S3Handler::download_stream` (the real, production streaming download
// entry point used by builder-api's `download_package` handler) against a real -- if minimal --
// local HTTP server standing in for S3/Minio, rather than mocking the AWS SDK itself. This is
// what actually catches regressions in how the handler adapts a live `GetObject` response into a
// byte stream, which purely synthetic, in-memory stream tests elsewhere cannot verify.
#[cfg(test)]
mod download_stream_integration_tests {
    use super::*;
    use std::{net::TcpListener,
              time::Duration};

    use futures::stream::StreamExt;

    // Spawn a minimal, one-shot raw HTTP/1.1 server on an OS-assigned loopback port. It accepts
    // a single connection, drains (and discards) the request (including whatever SigV4 auth
    // headers the AWS SDK sends -- this server doesn't validate them, it just needs to look like
    // a plain HTTP endpoint), then writes back the given raw response bytes verbatim before
    // closing the connection. This lets us simulate both a complete `GetObject` response and a
    // backend connection that closes before delivering as many bytes as it originally promised
    // via `Content-Length`, without needing a real S3/Minio instance.
    fn spawn_mock_server(response: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let addr = listener.local_addr().expect("mock server local addr");

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_read_timeout(Some(Duration::from_secs(2))).ok();

                let mut request_buf = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            request_buf.extend_from_slice(&buf[..n]);
                            if request_buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }

                let _ = stream.write_all(&response);
                let _ = stream.flush();
                // Dropping `stream` here closes the connection.
            }
        });

        format!("http://{}", addr)
    }

    fn http_ok_response(body: &[u8], declared_content_length: usize) -> Vec<u8> {
        let mut resp = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: \
                                application/octet-stream\r\nConnection: close\r\n\r\n",
                               declared_content_length).into_bytes();
        resp.extend_from_slice(body);
        resp
    }

    fn test_handler(endpoint: String) -> S3Handler {
        S3Handler::new(S3Cfg { key_id: "test-key-id".to_string(),
                               secret_key: "test-secret-key".to_string(),
                               bucket_name: "test-bucket".to_string(),
                               backend: S3Backend::Minio,
                               endpoint })
    }

    fn test_ident() -> PackageIdent {
        PackageIdent::from_str("acme/streamtest/1.0.0/20200101000000").expect("valid ident")
    }

    #[tokio::test]
    async fn download_stream_forwards_full_body_from_a_real_backend_connection() {
        let payload = b"artifact bytes delivered over a real (loopback) HTTP connection, in \
                        full."
                              .to_vec();
        let response = http_ok_response(&payload, payload.len());
        let endpoint = spawn_mock_server(response);

        let handler = test_handler(endpoint);
        let target = PackageTarget::from_str("x86_64-linux").expect("valid target");

        let (_content_length, mut stream) =
            handler.download_stream(&test_ident(), target)
                   .await
                   .expect("download_stream should succeed for a complete backend response");

        let mut received = Vec::new();
        while let Some(chunk) = stream.next().await {
            received.extend_from_slice(&chunk.expect("chunk should not error for a complete \
                                                      response"));
        }

        assert_eq!(received, payload);
    }

    #[tokio::test]
    async fn download_stream_surfaces_a_stream_error_for_a_truncated_backend_connection() {
        let full_payload = b"artifact bytes that will be cut off before fully delivered to the \
                             caller, simulating a dropped backend connection!!"
                                                                               .to_vec();
        // Declare a Content-Length matching the *full* payload, but only actually send half of
        // it before the mock server closes the connection -- simulating a truncated/dropped
        // backend transfer partway through.
        let truncated_payload = &full_payload[..full_payload.len() / 2];
        let response = http_ok_response(truncated_payload, full_payload.len());
        let endpoint = spawn_mock_server(response);

        let handler = test_handler(endpoint);
        let target = PackageTarget::from_str("x86_64-linux").expect("valid target");

        // The initial response (status + headers, including the declared Content-Length) is
        // still received successfully -- the truncation is only observable once the body stream
        // itself is consumed, which is exactly why `download_package` cannot rely on a
        // successful `download_stream` call alone to guarantee the artifact is intact.
        let (_content_length, mut stream) =
            handler.download_stream(&test_ident(), target)
                   .await
                   .expect("headers should still be received even though the body will be \
                            truncated");

        let mut saw_error = false;
        while let Some(chunk) = stream.next().await {
            if chunk.is_err() {
                saw_error = true;
                break;
            }
        }

        assert!(saw_error,
                "expected the truncated backend connection to surface as a stream error instead \
                 of silently yielding a short/incomplete body as if it were complete");
    }
}
