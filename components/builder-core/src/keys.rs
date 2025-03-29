// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
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

use std::env;
use habitat_core::{crypto::keys::{Key, BuilderSecretEncryptionKey, KeyCache, NamedRevision}, Error};

pub const BUILDER_KEY_NAME: &str = "bldr";

fn get_bldr_secret_key_from_env() -> Option<Result<BuilderSecretEncryptionKey, Error>> {
    match env::var("BLDR_SECRET_KEY") {
        Ok(val) => {
            // Update the value before parsing: replace "\\n" with actual newline characters
            let val = val.replace("\\n", "\n");
            Some(val.parse::<BuilderSecretEncryptionKey>().map_err(|e| {
                Error::CryptoError(format!("Failed to parse `BLDR_SECRET_KEY`: {}", e))
            }))
        }
        Err(_) => None, // If not set, return None
    }
}

/// Retrieves the latest builder secret encryption key.
/// 
/// First, it attempts to retrieve the key from the environment variable `BLDR_SECRET_KEY`.
/// If the key is not set in the environment, it falls back to fetching the key from the `key_cache`.
pub fn get_latest_builder_key(key_cache: &KeyCache) -> Result<BuilderSecretEncryptionKey, Error> {
    if let Some(result) = get_bldr_secret_key_from_env() {
        return result;
    }

    key_cache.latest_builder_key()
}

/// Retrieves the builder secret encryption key for a specific revision.
/// 
/// First, it attempts to retrieve the key from the environment variable `BLDR_SECRET_KEY`.
/// If the key is found and matches the revision, it is returned. If there is a revision mismatch,
/// an error is returned with details about the mismatch.
/// If the key is not found in the environment, the function falls back to fetching the key from 
/// the `key_cache` based on the provided `named_revision`.
pub fn get_builder_key_for_revision(key_cache: &KeyCache, named_revision: &NamedRevision) -> Result<BuilderSecretEncryptionKey, Error> {
    if let Some(result) = get_bldr_secret_key_from_env() {
        let key = result?;

        // Generally, this should not happen if the BLDR_SECRET_KEY is used to configure the key. 
        // This is an extra safeguard to inform that there is a mismatch in the key for some reason.
        if key.named_revision() != named_revision {
            return Err(Error::CryptoError(format!(
                "Revision mismatch from env BLDR_SECRET_KEY. Expected revision: {}, Found: {}",
                named_revision, key.named_revision()
            )));
        }

        return Ok(key);
    }

    key_cache.builder_secret_encryption_key(named_revision)
}

/// This module contains tests that interact with the process environment (via environment variables).
/// 
/// These tests cannot be run in parallel because they modify the `BLDR_SECRET_KEY` environment variable, 
/// and concurrent access may cause interference or inconsistent results. 
/// 
/// Some tests are ignored for manual inspection when issues occur. 
/// In the future, we may use a crate like 'temp-env' to handle environment-based tests if needed.
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper function to set the env variable
    fn set_bldr_secret_key(key_value: &str) {
        env::set_var("BLDR_SECRET_KEY", key_value);
    }

    #[test]
    #[ignore]
    fn test_bldr_secret_key_not_set() {
        let result = get_bldr_secret_key_from_env();

        assert!(result.is_none(), "Expected None when BLDR_SECRET_KEY is not set");
    }

    // Test case for BLDR_SECRET_KEY with escaped newlines
    #[test]
    fn test_bldr_secret_key_with_escaped_newlines() {
        let key_with_escaped_newlines = r#"BOX-SEC-1\nbldr-20200825205529\n\nM9u8wuJmZMsmVG4tNgngYJDapjIJE1RnxJAFVN97Bxs="#;

        set_bldr_secret_key(key_with_escaped_newlines);

        let result = get_bldr_secret_key_from_env();

        assert!(result.is_some(), "The result should be Some.");

        match result.unwrap() {
            Ok(key) => {
                assert_eq!(format!("{}", key.named_revision()), "bldr-20200825205529");
            }
            Err(e) => {
                panic!("Failed to parse key: {}", e);
            }
        }
    }

    // Test case for BLDR_SECRET_KEY with actual newlines
    #[test]
    #[ignore]
    fn test_bldr_secret_key_with_actual_newlines() {
        let key_with_newlines = r#"BOX-SEC-1
bldr-20200825205529

M9u8wuJmZMsmVG4tNgngYJDapjIJE1RnxJAFVN97Bxs="#;

        set_bldr_secret_key(key_with_newlines);

        let result = get_bldr_secret_key_from_env();

        assert!(result.is_some(), "The result should be Some.");

        match result.unwrap() {
            Ok(key) => {
                assert_eq!(format!("{}", key.named_revision()), "bldr-20200825205529");
            }
            Err(e) => {
                panic!("Failed to parse key: {}", e);
            }
        }
    }
}
