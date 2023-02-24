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

//! Archiver variant which stores job logs in the local filesystem.
//!
//! To avoid the problems of large numbers of files stored within a
//! single directory, the archiver stores logs in a nested directory
//! structure, based on the SHA256 checksum of a job's ID. For
//! example, job `722477594578067456` would be stored in
//! `/archive/97/6e/48/3c/722477594578067456.log`, where `/archive` is
//! the root of the archive on the filesystem. This is the same
//! approach taken by Chef's `bookshelf` cookbook storage engine.

use crate::{config::ArchiveCfg,
            error::Result,
            server::log_directory::LogDirectory};
use async_trait::async_trait;

use sha2::{Digest,
           Sha256};
use std::{fs::{self,
               OpenOptions},
          io::Read,
          path::{Path,
                 PathBuf}};

use super::LogArchiver;

/// Wraps a `PathBuf` representing the root of a local job log archive.
pub struct LocalArchiver(PathBuf);

impl LocalArchiver {
    // CM TODO: Implement an error type for bad configuration
    pub fn new(config: &ArchiveCfg) -> Self {
        let archive_dir = config.local_dir
                                .as_ref()
                                .expect("Missing local archive directory!");
        LogDirectory::validate(archive_dir).unwrap();
        LocalArchiver(archive_dir.clone())
    }

    /// Generate the path that a given job's logs will be stored
    /// at. Uses the first 4 bytes of the SHA256 checksums of the ID
    /// to generate a filesystem path that should distribute files so
    /// as not to run afoul of directory limits.
    pub fn archive_path(&self, job_id: u64) -> PathBuf {
        let mut hasher = Sha256::default();
        hasher.update(job_id.to_string().as_bytes());
        let checksum = hasher.finalize();

        let mut new_path = self.0.clone();
        for byte in checksum.iter().take(4) {
            // 0-pad the representation, e.g. "0a", not "a"
            new_path.push(format!("{:02x}", byte));
        }
        new_path.push(format!("{}.log", job_id));

        new_path
    }
}

#[async_trait]
impl LogArchiver for LocalArchiver {
    async fn archive(&self, job_id: u64, file_path: &Path) -> Result<()> {
        let archive_path = self.archive_path(job_id);
        let parent_dir = &archive_path.parent().unwrap();
        fs::create_dir_all(parent_dir)?;
        fs::copy(file_path, &archive_path)?;
        Ok(())
    }

    async fn retrieve(&self, job_id: u64) -> Result<Vec<String>> {
        let log_file = self.archive_path(job_id);
        let mut buffer = Vec::new();
        let mut file = OpenOptions::new().read(true).open(log_file)?;
        file.read_to_end(&mut buffer)?;
        let lines = String::from_utf8_lossy(buffer.as_slice()).lines()
                                                              .map(|l| l.to_string())
                                                              .collect();
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_archive_path() {
        let archiver = LocalArchiver(PathBuf::from("/archive"));
        let job_id: u64 = 722_543_779_847_979_008;
        let expected_path = PathBuf::from(format!("/archive/0a/6b/ef/ac/{}.log", job_id));
        let actual_path = archiver.archive_path(job_id);
        assert_eq!(actual_path, expected_path);
    }
}
