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

//! Archiver variant which uses S3 (or an API compatible clone) for
//! log storage.
//!
//! Has been tested against both AWS S3 and [Minio](https://minio.io).
//!
//! All job logs are stored in a single bucket, using the job's ID
//! (with a `.log` extension) as the key.
//!
//! # Configuration
//!
//! Currently the archiver must be configured with both an access key
//! ID and a secret access key.

use async_trait::async_trait;
use futures::stream::TryStreamExt;
use rusoto_s3::{GetObjectRequest,
                PutObjectRequest,
                S3Client,
                S3};
use std::{fs::OpenOptions,
          io::Read,
          path::PathBuf,
          str::FromStr};

use rusoto_core::HttpClient;

use crate::rusoto::{credential::StaticProvider,
                    Region};

use super::LogArchiver;
use crate::{config::ArchiveCfg,
            error::{Error,
                    Result}};

pub struct S3Archiver {
    client: S3Client,
    bucket: String,
}

impl S3Archiver {
    pub fn new(config: &ArchiveCfg) -> Self {
        let key = config.key
                        .as_ref()
                        .cloned()
                        .expect("S3 key must be configured");

        let secret = config.secret
                           .as_ref()
                           .cloned()
                           .expect("S3 secret must be configured");

        let bucket = config.bucket
                           .as_ref()
                           .cloned()
                           .expect("S3 bucket must be configured");

        let region = Region::from_str(config.region.as_str()).unwrap();

        let cred_provider = StaticProvider::new_minimal(key, secret);
        let http_client = HttpClient::new().expect("Rusoto http client must be availalbe");
        let client = S3Client::new_with(http_client, cred_provider, region);

        S3Archiver { client, bucket }
    }

    /// Generates the bucket key under which the job log will be
    /// stored.
    fn key(job_id: u64) -> String { format!("{}.log", job_id) }
}

#[async_trait]
impl LogArchiver for S3Archiver {
    async fn archive(&self, job_id: u64, file_path: &PathBuf) -> Result<()> {
        let mut buffer = Vec::new();
        let mut request = PutObjectRequest::default();
        request.bucket = self.bucket.clone();
        request.key = Self::key(job_id);

        let mut file = OpenOptions::new().read(true).open(file_path)?;
        file.read_to_end(&mut buffer)?;
        request.body = Some(buffer.into());

        match self.client.put_object(request).await {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("Job log upload failed for {}: ({:?})", job_id, e);
                Err(Error::JobLogArchive(job_id, e))
            }
        }
    }

    async fn retrieve(&self, job_id: u64) -> Result<Vec<String>> {
        let mut request = GetObjectRequest::default();
        request.bucket = self.bucket.clone();
        request.key = Self::key(job_id);

        let payload = self.client.get_object(request).await;
        let stream = match payload {
            Ok(response) => response.body.expect("Downloaded object is not empty"),
            Err(e) => {
                warn!("Failed to retrieve job log for {} ({:?})", job_id, e);
                return Err(Error::JobLogRetrieval(job_id, e));
            }
        };

        let bytes = stream.map_ok(|b| bytes::BytesMut::from(&b[..]))
                          .try_concat()
                          .await
                          .expect("Unable to retrieve byte stream");

        let lines = String::from_utf8_lossy(&bytes).lines()
                                                   .map(|l| l.to_string())
                                                   .collect();

        Ok(lines)
    }
}
