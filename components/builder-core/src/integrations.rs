// This module holds code that's common to dealing with the integrations for builder-api and
// builder-depot

use crate::{error::{Error,
                    Result},
            keys};
use habitat_core::crypto::{keys::box_key_pair::WrappedSealedBox,
                           BoxKeyPair};
use std::path::Path;

// TBD - these functions should take keys directly instead of key directory.

pub fn encrypt_with_keypair(key_pair: &BoxKeyPair, bytes: &[u8]) -> Result<String> {
    let wsb = match key_pair.encrypt(bytes, Some(&key_pair)) {
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

pub fn get_keypair_helper<A>(key_dir: A) -> Result<BoxKeyPair>
    where A: AsRef<Path>
{
    // This probably could be rewritten as a map_err
    let display_path = key_dir.as_ref().display();
    match BoxKeyPair::get_latest_pair_for(keys::BUILDER_KEY_NAME, &key_dir.as_ref()) {
        Ok(p) => Ok(p),
        Err(err) => {
            let e = format!("Can't find bldr key pair at {}, err={}", &display_path, err);
            error!("Can't find bldr key pair at {}", &display_path);
            Err(Error::EncryptError(e))
        }
    }
}

pub fn encrypt<A>(key_dir: A, bytes: &[u8]) -> Result<(String, String)>
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

    encrypt_with_keypair(&kp, bytes).map(|x| (x, kp.rev))
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
