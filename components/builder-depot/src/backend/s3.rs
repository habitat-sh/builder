// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use bldr_core::metrics::CounterMetric;
use futures::{Future, Stream};
use hab_core::package::{Identifiable, PackageArchive, PackageIdent, PackageTarget};
use iron::typemap::Key;
use metrics::Counter;
use rusoto::{credential::StaticProvider, reactor::RequestDispatcher, Region};
use rusoto_s3::{CompleteMultipartUploadRequest, CompletedMultipartUpload, CompletedPart,
                CreateBucketRequest, CreateMultipartUploadRequest, GetObjectRequest,
                PutObjectRequest, S3, S3Client, UploadPartRequest};
use time::PreciseTime;

use config::{S3Backend, S3Cfg};
use Config;
use DepotUtil;

use error::{Error, Result};

// This const is equal to 6MB which is slightly above
// the minimum limit for a multipart upload request
// to s3. Any package over 6MB on upload will use this api
const MINLIMIT: usize = 10240 * 1024;

pub struct S3Handler {
    client: S3Client<StaticProvider, RequestDispatcher>,
    bucket: String,
}

impl S3Handler {
    // The S3 Handler struct contains all of the credential
    // and target information that we should need to perfom
    // any backend operations
    pub fn new(config: S3Cfg) -> Self {
        let region = match config.backend {
            S3Backend::Minio => Region::Custom {
                name: "minio_s3".to_owned(),
                endpoint: config.endpoint.to_string(),
            },
            S3Backend::Aws => Region::from_str(config.endpoint.as_str()).unwrap(),
        };
        let aws_id = config.key_id;
        let aws_secret = config.secret_key;
        let cred_provider = StaticProvider::new_minimal(aws_id, aws_secret);
        let client = S3Client::new(RequestDispatcher::default(), cred_provider, region);
        let bucket = config.bucket_name;

        S3Handler {
            client: client,
            bucket: bucket,
        }
    }

    // Helper function for programmatic creation of
    // the s3 object key
    fn s3_key<T>(&self, ident: &T, target: &PackageTarget) -> PathBuf
    where
        T: Identifiable,
    {
        Path::new(ident.origin())
            .join(format!("{}", ident.name()))
            .join(format!("{}", ident.version().unwrap()))
            .join(format!("{}", ident.release().unwrap()))
            .join(format!("{}", target.architecture))
            .join(format!("{}", target.platform))
            .join(format!(
                "{}-{}-{}-{}-{}-{}.hart",
                ident.origin(),
                ident.name(),
                ident.version().unwrap(),
                ident.release().unwrap(),
                target.architecture,
                target.platform
            ))
    }

    // This function checks whether or not the
    // configured bucket exists in the configured
    // backend.
    fn bucket_exists(&self) -> Result<bool> {
        let artifactbucket = self.bucket.to_owned();
        match self.client.list_buckets().sync() {
            Ok(bucket_list) => match bucket_list.buckets {
                Some(buckets) => {
                    return Ok(buckets
                        .iter()
                        .any(|ref x| x.name.clone().unwrap() == artifactbucket))
                }
                None => Ok(false),
            },
            Err(e) => {
                debug!("{:?}", e);
                return Err(Error::ListBuckets(e));
            }
        }
    }

    pub fn create_bucket(&self) -> Result<()> {
        let mut request = CreateBucketRequest::default();
        request.bucket = self.bucket.clone();

        match self.bucket_exists() {
            Ok(_) => Ok(()),
            Err(_) => match self.client.create_bucket(&request).sync() {
                Ok(_response) => Ok(()),
                Err(e) => {
                    debug!("{:?}", e);
                    Err(Error::CreateBucketError(e))
                }
            },
        }
    }

    pub fn upload(
        &self,
        hart: &PathBuf,
        ident: &PackageIdent,
        target: &PackageTarget,
    ) -> Result<()> {
        Counter::UploadRequests.increment();
        let key = self.s3_key(ident, target).to_string_lossy().into_owned();
        let file = File::open(hart)?;
        let path_attr = &hart.clone().into_os_string().into_string().unwrap();
        info!("S3Handler::upload request started for s3_key: {:?}", key);
        let size = file.metadata().unwrap().len() as usize;
        if size < MINLIMIT {
            self.single_upload(key, file, &path_attr)
        } else {
            self.multipart_upload(key, file, &path_attr)
        }
    }

    pub fn download(
        &self,
        loc: &PathBuf,
        ident: &PackageIdent,
        target: &PackageTarget,
    ) -> Result<PackageArchive> {
        Counter::DownloadRequests.increment();
        let mut request = GetObjectRequest::default();
        let key = self.s3_key(ident, target).to_string_lossy().into_owned();
        request.bucket = self.bucket.to_owned();
        request.key = key;

        let payload = self.client.get_object(&request).sync();
        let body = match payload {
            Ok(response) => response.body,
            Err(e) => {
                warn!("Failed to retrieve object from S3: {:?}", e);
                return Err(Error::PackageDownload(e));
            }
        };

        let file = body.expect("Downloaded pkg archive empty!").concat2();
        match Config::write_archive(&loc, file.wait().unwrap()) {
            Ok(result) => return Ok(result),
            Err(e) => {
                warn!("Unable to write file {:?} to archive, err={:?}", loc, e);
                return Err(e);
            }
        }
    }

    fn single_upload(&self, key: String, hart: File, path_attr: &str) -> Result<()> {
        Counter::SingleUploadRequests.increment();
        let start_time = PreciseTime::now();
        let mut reader = BufReader::new(hart);
        let mut object: Vec<u8> = Vec::new();
        let bucket = self.bucket.clone();
        let _complete = reader.read_to_end(&mut object)?;

        let mut request = PutObjectRequest::default();
        request.key = key;
        request.bucket = bucket;
        request.body = Some(object);

        match self.client.put_object(&request).sync() {
            Ok(_) => {
                let end_time = PreciseTime::now();
                info!(
                    "Upload completed for {} (in {} sec):",
                    &path_attr,
                    start_time.to(end_time)
                );
                Ok(())
            }
            Err(e) => {
                Counter::UploadFailures.increment();
                warn!("Upload failed for {}: ({:?})", &path_attr, e);
                Err(Error::PackageUpload(e))
            }
        }
    }

    fn multipart_upload(&self, key: String, hart: File, path_attr: &str) -> Result<()> {
        Counter::MultipartUploadRequests.increment();
        let start_time = PreciseTime::now();
        let mut p: Vec<CompletedPart> = Vec::new();
        let mut mprequest = CreateMultipartUploadRequest::default();
        mprequest.key = key.clone();
        mprequest.bucket = self.bucket.clone();

        match self.client.create_multipart_upload(&mprequest).sync() {
            Ok(output) => {
                let mut request = UploadPartRequest::default();
                request.key = mprequest.key;
                request.bucket = mprequest.bucket;
                request.upload_id = output.upload_id.clone().unwrap(); //unwrap safe

                let mut reader = BufReader::with_capacity(MINLIMIT, hart);
                let mut part_num: i64 = 0;
                let mut should_break = false;
                loop {
                    let mut length;
                    {
                        let buffer = reader.fill_buf()?;
                        length = buffer.len();
                        if length < MINLIMIT {
                            should_break = true;
                        }
                        part_num += 1;
                        request.body = Some(buffer.to_vec());
                        request.part_number = part_num;

                        match self.client.upload_part(&request).sync() {
                            Ok(upo) => {
                                p.push(CompletedPart {
                                    e_tag: upo.e_tag,
                                    part_number: Some(part_num),
                                });
                            }
                            Err(e) => {
                                debug!("{:?}", e);
                                return Err(Error::PartialUpload(e));
                            }
                        }
                    }
                    reader.consume(length);
                    if should_break {
                        break;
                    }
                }

                let mut completion = CompleteMultipartUploadRequest {
                    key: key,
                    bucket: self.bucket.clone(),
                    multipart_upload: Some(CompletedMultipartUpload { parts: Some(p) }),
                    upload_id: output.upload_id.unwrap(),
                    request_payer: None,
                };

                match self.client.complete_multipart_upload(&completion).sync() {
                    Ok(_) => {
                        let end_time = PreciseTime::now();
                        info!(
                            "Upload completed for {} (in {} sec):",
                            &path_attr,
                            start_time.to(end_time)
                        );
                        Ok(())
                    }
                    Err(e) => {
                        warn!("Upload failed for {}", &path_attr);
                        debug!("{:?}", e);
                        Err(Error::MultipartCompletion(e))
                    }
                }
            }
            Err(e) => {
                debug!("{:?}", e);
                Err(Error::MultipartUploadReq(e))
            }
        }
    }
}

pub struct S3Cli;

impl Key for S3Cli {
    type Value = S3Handler;
}
