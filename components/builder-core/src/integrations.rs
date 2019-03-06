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

// This module holds code that's common to dealing with the integrations for builder-api and
// builder-depot

use std::path::Path;

use base64;

use crate::{error::{Error,
                    Result},
            hab_core::crypto::{keys::box_key_pair::WrappedSealedBox,
                               BoxKeyPair},
            keys};

// TBD - these functions should take keys directly instead of key directory.

pub fn encrypt<A>(key_dir: A, bytes: &[u8]) -> Result<String>
    where A: AsRef<Path>
{
    let display_path = key_dir.as_ref().display();

    let kp = match BoxKeyPair::get_latest_pair_for(keys::BUILDER_KEY_NAME, &key_dir.as_ref()) {
        Ok(p) => p,
        Err(err) => {
            let e = format!("Can't find bldr key pair at {}, err={}", &display_path, err);
            error!("Can't find bldr key pair at {}", &display_path);
            return Err(Error::EncryptError(e));
        }
    };

    let wsb = match kp.encrypt(bytes, Some(&kp)) {
        Ok(s) => s,
        Err(err) => {
            let e = format!("Unable to encrypt with bldr key pair, err={:?}", &err);
            error!("Unable to encrypt with bldr key pair, err={:?}", err);
            return Err(Error::EncryptError(e));
        }
    };

    // kp.encrypt returns a WrappedSealedBox which contains readable metadata and base64
    // ciphertext. We base64 encode the WrappedSealedBox again, so that the returned string
    // is consistently base64 and does not have random text interspersed with readable text.
    // This makes it easier to pass around, eg, for access tokens, and is by design.
    // The downside is that there is double base64 happening.
    Ok(base64::encode(wsb.as_bytes()))
}

// This function takes in a double base64 encoded string
pub fn decrypt<A>(key_dir: A, b64text: &str) -> Result<Vec<u8>>
    where A: AsRef<Path>
{
    let decoded = base64::decode(b64text).map_err(Error::Base64Error)?;
    let wsb = &WrappedSealedBox::from(String::from_utf8(decoded).unwrap());
    let plaintext = match BoxKeyPair::decrypt_with_path(wsb, &key_dir.as_ref()) {
        Ok(bytes) => bytes,
        Err(err) => {
            let e = format!("Unable to decrypt with bldr key pair, err={:?}", &err);
            debug!("Unable to decrypt with bldr key pair, err={:?}", err);
            return Err(Error::DecryptError(e));
        }
    };

    Ok(plaintext)
}

// This function takes in a double base64 encoded string
pub fn validate<A>(key_dir: A, b64text: &str) -> Result<()>
    where A: AsRef<Path>
{
    let decoded = base64::decode(b64text).map_err(Error::Base64Error)?;
    let wsb = &WrappedSealedBox::from(String::from_utf8(decoded).unwrap());
    let box_secret = BoxKeyPair::secret_metadata(wsb)?;

    match BoxKeyPair::get_pair_for(box_secret.sender, &key_dir.as_ref()) {
        Ok(_) => (),
        Err(err) => {
            let e = format!("Unable to find sender key pair, err={:?}", &err);
            error!("Unable to find sender key pair, err={:?}", err);
            return Err(Error::DecryptError(e));
        }
    }

    match box_secret.receiver {
        Some(recv) => {
            match BoxKeyPair::get_pair_for(recv, &key_dir.as_ref()) {
                Ok(_) => (),
                Err(err) => {
                    let e = format!("Unable to find receiver key pair, err={:?}", &err);
                    error!("Unable to find receiver key pair, err={:?}", err);
                    return Err(Error::DecryptError(e));
                }
            }
        }
        None => {
            let e = "No receiver key pair specified".to_string();
            error!("No receiver key pair specified");
            return Err(Error::DecryptError(e));
        }
    };

    Ok(())
}
