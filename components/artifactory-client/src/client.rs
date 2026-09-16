// Copyright (c) 2019 Chef Software Inc. and/or applicable contributors
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

use std::{collections::HashMap,
          path::Path};

use crate::{config::ArtifactoryCfg,
            error::{ArtifactoryError,
                    ArtifactoryResult}};
use futures::stream::StreamExt;
use reqwest::{header::{HeaderMap,
                       HeaderName,
                       HeaderValue},
              Body,
              Response};

use crate::hab_core::package::{PackageArchive,
                               PackageIdent,
                               PackageTarget};

use builder_core::http_client::{HttpClient,
                                USER_AGENT_BLDR};
use tokio::io::AsyncWriteExt;
const X_JFROG_ART_API: &str = "x-jfrog-art-api";

#[derive(Clone)]
pub struct ArtifactoryClient {
    inner:       HttpClient,
    pub api_url: String,
    pub api_key: String,
    pub repo:    String,
}

impl ArtifactoryClient {
    pub fn new(config: ArtifactoryCfg) -> ArtifactoryResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT_BLDR.0.clone(), USER_AGENT_BLDR.1.clone());
        headers.insert(HeaderName::from_static(X_JFROG_ART_API),
                       HeaderValue::from_str(&config.api_key).expect("Invalid API key value"));

        Ok(ArtifactoryClient { inner:   HttpClient::new(&config.api_url, headers)?,
                               api_url: config.api_url,
                               api_key: config.api_key,
                               repo:    config.repo, })
    }

    pub async fn upload(&self,
                        source_path: &Path,
                        ident: &PackageIdent,
                        target: PackageTarget)
                        -> ArtifactoryResult<Response> {
        debug!("ArtifactoryClient upload request for file path: {:?}",
               source_path);

        let url = self.url_path_for(ident, target);
        debug!("ArtifactoryClient upload url = {}", url);

        let body: Body = tokio::fs::read(source_path).await
                                                     .map_err(ArtifactoryError::IO)?
                                                     .into();

        let resp = match self.inner
                             .put(&url)
                             .body(body)
                             .send()
                             .await
                             .map_err(ArtifactoryError::HttpClient)
        {
            Ok(resp) => resp,
            Err(err) => {
                error!("ArtifactoryClient upload failed, err={}", err);
                return Err(err);
            }
        };

        debug!("Artifactory response status: {:?}", resp.status());

        if resp.status().is_success() {
            Ok(resp)
        } else {
            error!("Artifactory upload non-success status: {:?}", resp.status());
            Err(ArtifactoryError::ApiError(resp.status(), HashMap::new()))
        }
    }

    /// Performs the initial GET request for a package artifact and returns the raw response
    /// without reading its body, so callers can stream the bytes directly to their own
    /// destination (e.g. an HTTP client) instead of buffering the whole artifact to local disk
    /// first. This avoids adding a full backend-fetch delay before any bytes reach the caller,
    /// which matters a great deal for very large packages.
    pub async fn download_response(&self,
                                   ident: &PackageIdent,
                                   target: PackageTarget)
                                   -> ArtifactoryResult<Response> {
        debug!("ArtifactoryClient streaming download request for {} ({})",
               ident, target);

        let url = self.url_path_for(ident, target);
        debug!("ArtifactoryClient download url = {}", url);

        let resp = match self.inner
                             .get(&url)
                             .send()
                             .await
                             .map_err(ArtifactoryError::HttpClient)
        {
            Ok(resp) => resp,
            Err(err) => {
                error!("ArtifactoryClient download failed, err={}", err);
                return Err(err);
            }
        };

        debug!("Artifactory response status: {:?}", resp.status());

        if resp.status().is_success() {
            Ok(resp)
        } else {
            error!("Artifactory download non-success status: {:?}",
                   resp.status());
            Err(ArtifactoryError::ApiError(resp.status(), HashMap::new()))
        }
    }

    pub async fn download(&self,
                          destination_path: &Path,
                          ident: &PackageIdent,
                          target: PackageTarget)
                          -> ArtifactoryResult<PackageArchive> {
        debug!("ArtifactoryClient download request for {} ({}) to destination path: {:?}",
               ident, target, destination_path);

        let url = self.url_path_for(ident, target);
        debug!("ArtifactoryClient download url = {}", url);

        let resp = match self.inner
                             .get(&url)
                             .send()
                             .await
                             .map_err(ArtifactoryError::HttpClient)
        {
            Ok(resp) => resp,
            Err(err) => {
                error!("ArtifactoryClient download failed, err={}", err);
                return Err(err);
            }
        };

        debug!("Artifactory response status: {:?}", resp.status());

        if resp.status().is_success() {
            let mut file = tokio::fs::File::create(destination_path).await
                                                                    .map_err(ArtifactoryError::IO)?;
            let mut stream = resp.bytes_stream();
            while let Some(chunk) = stream.next().await {
                file.write_all(&chunk?).await?;
            }
            Ok(PackageArchive::new(destination_path)?)
        } else {
            error!("Artifactory download non-success status: {:?}",
                   resp.status());
            Err(ArtifactoryError::ApiError(resp.status(), HashMap::new()))
        }
    }

    pub async fn delete(&self,
                        ident: &PackageIdent,
                        target: PackageTarget)
                        -> ArtifactoryResult<()> {
        let url = self.url_path_for(ident, target);
        debug!("ArtifactoryClient delete url = {}", url);

        let resp = match self.inner
                             .delete(&url)
                             .send()
                             .await
                             .map_err(ArtifactoryError::HttpClient)
        {
            Ok(resp) => resp,
            Err(err) => {
                error!("ArtifactoryClient delete failed, err={}", err);
                return Err(err);
            }
        };

        debug!("Artifactory delete response status: {:?}", resp.status());

        if resp.status().is_success() {
            Ok(())
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND
                  || resp.status() == reqwest::StatusCode::GONE
        {
            warn!("Artifactory delete returned {} for {} ({}); artifact may have already been \
                   removed",
                  resp.status(),
                  ident,
                  target);
            Ok(())
        } else {
            error!("Artifactory delete non-success status: {:?}", resp.status());
            Err(ArtifactoryError::ApiError(resp.status(), HashMap::new()))
        }
    }

    fn url_path_for(&self, ident: &PackageIdent, target: PackageTarget) -> String {
        let hart_name = ident.archive_name_with_target(target)
                             .expect("ident is fully qualified");

        let url = format!("{}/artifactory/{}/{}/{}/{}",
                          self.api_url,
                          self.repo,
                          ident.iter().collect::<Vec<&str>>().join("/"),
                          target.iter().collect::<Vec<&str>>().join("/"),
                          hart_name);

        url
    }
}

// These tests exercise `ArtifactoryClient::download_response` (the real, production streaming
// download entry point used by builder-api's `download_package` handler) against a real -- if
// minimal -- local HTTP server, rather than mocking `reqwest` itself. This is what actually
// catches regressions in how the client adapts a live backend HTTP response into a byte stream
// (e.g. a change that accidentally buffers the body, drops errors, or otherwise breaks partway
// through consuming `bytes_stream()`), which purely synthetic, in-memory stream tests elsewhere
// cannot verify.
#[cfg(test)]
mod download_response_streaming_tests {
    use super::*;
    use std::{io::{Read,
                   Write},
              net::TcpListener,
              str::FromStr,
              time::Duration};

    // Spawn a minimal, one-shot raw HTTP/1.1 server on an OS-assigned loopback port. It accepts
    // a single connection, drains (and discards) the request, then writes back the given raw
    // response bytes verbatim before closing the connection. This lets us simulate both a
    // complete response and a backend connection that closes before delivering as many bytes as
    // it originally promised via `Content-Length` -- without needing a real Artifactory/S3
    // instance or a network mocking crate.
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

    fn test_client(api_url: String) -> ArtifactoryClient {
        ArtifactoryClient::new(ArtifactoryCfg { api_url,
                                               api_key: "test-api-key".to_string(),
                                               repo: "test-repo".to_string() })
            .expect("client should construct")
    }

    fn test_ident() -> PackageIdent {
        PackageIdent::from_str("acme/streamtest/1.0.0/20200101000000").expect("valid ident")
    }

    #[tokio::test]
    async fn download_response_streams_full_body_from_a_real_backend_connection() {
        let payload = b"artifact bytes delivered over a real (loopback) HTTP connection, in \
                        full."
            .to_vec();
        let response = http_ok_response(&payload, payload.len());
        let api_url = spawn_mock_server(response);

        let client = test_client(api_url);
        let target = PackageTarget::from_str("x86_64-linux").expect("valid target");

        let resp = client.download_response(&test_ident(), target)
                         .await
                         .expect("download_response should succeed for a complete backend \
                                 response");

        let mut received = Vec::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            received.extend_from_slice(&chunk.expect("chunk should not error for a complete \
                                                       response"));
        }

        assert_eq!(received, payload);
    }

    #[tokio::test]
    async fn download_response_surfaces_a_stream_error_for_a_truncated_backend_connection() {
        let full_payload = b"artifact bytes that will be cut off before fully delivered to the \
                             caller, simulating a dropped backend connection!!"
            .to_vec();
        // Declare a Content-Length matching the *full* payload, but only actually send half of
        // it before the mock server closes the connection -- simulating a truncated/dropped
        // backend transfer partway through.
        let truncated_payload = &full_payload[..full_payload.len() / 2];
        let response = http_ok_response(truncated_payload, full_payload.len());
        let api_url = spawn_mock_server(response);

        let client = test_client(api_url);
        let target = PackageTarget::from_str("x86_64-linux").expect("valid target");

        // The initial response (status + headers) is still received successfully -- the
        // truncation is only observable once the body stream itself is consumed, which is
        // exactly why `download_package` cannot rely on a successful `download_response` call
        // alone to guarantee the artifact is intact.
        let resp = client.download_response(&test_ident(), target)
                         .await
                         .expect("headers should still be received even though the body will \
                                 be truncated");

        let mut stream = resp.bytes_stream();
        let mut saw_error = false;
        while let Some(chunk) = stream.next().await {
            if chunk.is_err() {
                saw_error = true;
                break;
            }
        }

        assert!(saw_error,
                "expected the truncated backend connection to surface as a stream error \
                 instead of silently yielding a short/incomplete body as if it were complete");
    }
}
