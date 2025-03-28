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
