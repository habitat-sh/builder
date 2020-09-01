//! This module holds code that's common to dealing with the
//! integrations for builder-api and builder-depot

use crate::{error::{Error,
                    Result},
            keys};
use habitat_core::crypto::keys::{BuilderSecretEncryptionKey,
                                 Key,
                                 KeyCache,
                                 KeyRevision,
                                 SignedBox};
use std::path::Path;

/// Encrypts bytes using the latest Builder encryption key in the
/// `KeyCache`, returning the base64-encoded encrypted `SignedBox`,
/// along with the revision of the Builder key that was used. The
/// bytes are encrypted such that only Builder could have encrypted
/// it, and only Builder can decrypt it.
///
/// It is *not* returning the `NamedRevision` because all Builder keys
/// have the same name, by definition (there's no inherent reason to
/// *just* return the `KeyRevision`, it's just what other parts of
/// Builder take right now).
///
/// This function is what all encryption in Builder uses. While the
/// base64 encoding used here is only strictly necessary for access
/// tokens, but it is also used for storing encrypted data in the
/// database, even though the actual cryptographic bits of a
/// `SignedBox` are already serialized as base64.
// TODO (CM): If we were to ever change this, we'd need to migrate
// data stored in the database to decode it first. Furthermore, the
// paired `decrypt` method below assumes that its input will be base64
// encoded. Interestingly, we store *public* keys (i.e., unencrypted
// key strings, which themselves are a mix of plaintext and base64
// encoded binary information, just like our SignedBoxes are).
//
// TODO (CM): If we were to just return a SignedBox, we would be able
// to get the KeyRevision directly from it, since all SignedBoxes know
// who the encryptor is. However, that would require knowing that for
// these SignedBoxes, the encryptor and decryptor are always the
// same. If we were to do that, it would be useful to create a new
// type to represent that fact, but that could require database
// migrations if not done properly.
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
    let encrypted: SignedBox = key.encrypt(bytes);
    let b64 = base64::encode(encrypted.to_string());
    (b64, key.named_revision().revision().clone())
}

/// Decrypts a given base64 `SignedBox` using the appropriate Builder
/// encryption key. We pass the `KeyCache` because the encoded message
/// tells us which key revision to use, so we don't know which key
/// we're going to use until we dig into the message itself.
pub fn decrypt(key_cache: &KeyCache, b64text: &str) -> Result<Vec<u8>> {
    let signed_box = base64::decode(b64text).map(String::from_utf8)??
                                            .parse::<SignedBox>()?;
    let builder_key = key_cache.builder_secret_encryption_key(signed_box.decryptor())?;
    Ok(builder_key.decrypt(&signed_box)?)
}
