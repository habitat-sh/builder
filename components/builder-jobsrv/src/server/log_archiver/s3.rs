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

use std::fs::OpenOptions;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;

use futures::{Future, Stream};
use rusoto::{credential::StaticProvider, reactor::RequestDispatcher, Region};
use rusoto_s3::{GetObjectRequest, PutObjectRequest, S3Client, S3};

use super::LogArchiver;
use config::ArchiveCfg;
use error::{Error, Result};

pub struct S3Archiver {
    client: S3Client<StaticProvider, RequestDispatcher>,
    bucket: String,
}

impl S3Archiver {
    pub fn new(config: &ArchiveCfg) -> Result<S3Archiver> {
        let key = config
            .key
            .as_ref()
            .cloned()
            .expect("S3 key must be configured");

        let secret = config
            .secret
            .as_ref()
            .cloned()
            .expect("S3 secret must be configured");

        let bucket = config
            .bucket
            .as_ref()
            .cloned()
            .expect("S3 bucket must be configured");

        let region = Region::from_str(config.region.as_str()).unwrap();

        let cred_provider = StaticProvider::new_minimal(key, secret);
        let client = S3Client::new(RequestDispatcher::default(), cred_provider, region);

        Ok(S3Archiver {
            client: client,
            bucket: bucket,
        })
    }

    /// Generates the bucket key under which the job log will be
    /// stored.
    fn key(job_id: u64) -> String {
        format!("{}.log", job_id)
    }
}

impl LogArchiver for S3Archiver {
    fn archive(&self, job_id: u64, file_path: &PathBuf) -> Result<()> {
        let mut buffer = Vec::new();
        let mut request = PutObjectRequest::default();
        request.bucket = self.bucket.clone();
        request.key = Self::key(job_id);

        let mut file = OpenOptions::new().read(true).open(file_path)?;
        file.read_to_end(&mut buffer)?;
        request.body = Some(buffer.as_slice().to_vec());

        match self.client.put_object(&request).sync() {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("Job log upload failed for {}: ({:?})", job_id, e);
                Err(Error::JobLogArchive(job_id, e))
            }
        }
    }

    fn retrieve(&self, job_id: u64) -> Result<Vec<String>> {
        let mut request = GetObjectRequest::default();
        request.bucket = self.bucket.clone();
        request.key = Self::key(job_id);

        let payload = self.client.get_object(&request).sync();
        let stream = match payload {
            Ok(response) => response.body.expect("Downloaded object is not empty"),
            Err(e) => {
                warn!("Failed to retrieve job log for {} ({:?})", job_id, e);
                return Err(Error::JobLogRetrieval(job_id, e));
            }
        };

        let body = stream.concat2().wait().unwrap();

        let lines = String::from_utf8_lossy(body.as_slice())
            .lines()
            .map(|l| l.to_string())
            .collect();

        Ok(lines)
    }
}
