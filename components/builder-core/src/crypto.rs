//! This module holds code that's common to dealing with the
//! encrypting and decrytping data in Builder.

use crate::error::Result;
use habitat_core::crypto::keys::{BuilderSecretEncryptionKey,
                                 Key,
                                 KeyCache,
                                 KeyRevision,
                                 SignedBox};

/// Encrypts bytes using the latest Builder encryption key in the
/// `KeyCache`, returning the encrypted content as a string.
///
/// It is *not* returning the `NamedRevision` because all Builder keys
/// have the same name, by definition (there's no inherent reason to
/// *just* return the `KeyRevision`, it's just what other parts of
/// Builder take right now).
///
/// Returning a String, and not a SignedBox, because at some point
/// we'd like to transition to symmetric key encryption, the output of
/// which would be represented by a different type. Also, it's not
/// really important outside of this module what kind of encryption is
/// being used, just that there _is_ encryption.
pub fn encrypt<B>(key_cache: &KeyCache, bytes: B) -> Result<(String, KeyRevision)>
    where B: AsRef<[u8]>
{
    let key = key_cache.latest_builder_key()?;
    Ok(encrypt_with_key(&key, bytes))
}

/// Same as `encrypt`, but using a specific key.
pub fn encrypt_with_key<B>(key: &BuilderSecretEncryptionKey, bytes: B) -> (String, KeyRevision)
    where B: AsRef<[u8]>
{
    let encrypted = key.encrypt(bytes);
    (encrypted.to_string(), key.named_revision().revision().clone())
}

/// Decrypts a given string rendering of encrypted content using the
/// appropriate Builder encryption key. We pass the `KeyCache` because
/// the encoded message tells us which key revision to use, so we
/// don't know which key we're going to use until we dig into the
/// message itself.
///
/// Returns a byte vector because not everything we encrypt is a
/// string. At some point it may be worthwhile to provide a typed
/// decryption interface to consolidate incidental logic here, thus
/// cleaning up callsites. But for now, you get bytes!
pub fn decrypt(key_cache: &KeyCache, encrypted_message: &str) -> Result<Vec<u8>> {
    let encrypted_message = encrypted_message.parse::<SignedBox>()?;
    let builder_key = key_cache.builder_secret_encryption_key(encrypted_message.decryptor())?;
    Ok(builder_key.decrypt(&encrypted_message)?)
}
