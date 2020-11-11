//! Defines the structure and operations of access tokens that users need to
//! interact with restricted portions of the Builder API.

use super::privilege::FeatureFlags;
use crate::{crypto,
            error::{Error,
                    Result},
            protocol::{message,
                       originsrv}};
use chrono::{self,
             Duration,
             LocalResult::Single,
             TimeZone,
             Utc};
use habitat_core::crypto::keys::{KeyCache,
                                 SignedBox};
use std::{fmt,
          str::FromStr};

pub const BUILDER_ACCOUNT_ID: u64 = 0;
pub const BUILDER_ACCOUNT_NAME: &str = "BUILDER";
const BUILDER_TOKEN_LIFETIME_HOURS: i64 = 2;

/// Access token prefix rules:
///
/// * MUST contain an *invalid* base64 character
/// * MUST NOT contain special shell characters (eg, !)
/// * SHOULD be URL-safe (just in case)
const ACCESS_TOKEN_PREFIX: &str = "_";

/// Encapsulates the string encoding of the encrypted OriginSrv::AccessToken
/// type, as well as logic for creating, serializing, and validating access
/// tokens for the API.
///
/// The flow of logic is actually a little surprising, as we have several
/// representations of access tokens in the system.
///
/// *This* `AccessToken` is ultimately responsible for generating what a user
/// will see as their token, i.e., a string like
///
///    _Qk9YLTEKYmxkci0yMDIwMDkxNDE0NTYwNwpibGRyLTIwMjAwOTE0MTQ1NjA3CkgvS21jOTczbm4xNWpySHhRVVI0TmU4Z3U5Mm5DWEZpCmFJcVlDRnl4aohLOLyouthoughtthiswasrealNOPEKSThIUXluNUxLaVZFbVFvcA==
///
/// This is, in reality, the base64-encoding of the string form of a the
/// Builder-encrypted protobuf-encoded bytes of an `originsrv::AccessToken`
/// (with a "_" added to the front).
///
/// WHEW!
///
/// For this type, we will simply wrap the string rendering of the encrypted
/// protobuf payload; the base64-encoding and "_"-prefixing will be handled by
/// the `Display` trait implementation.
///
/// But wait, it doesn't end there! That's just how we *create* a token; it says
/// nothing about we *consume* a token.
///
/// Once a user sends us a token (as part of an API request), we have to parse
/// it and decrypt its contents. At that point, we're back to the
/// `originsrv::AccessToken` we started with. This, in turn, ends up being
/// converted into an `originsrv::Session` struct for use within the API.
///
/// Thus, the overall flow of types is:
///
///    originsrv::AccessToken -> AccessToken (i.e., this type) -> originsrv::Session
///
/// Due to the current setup of these types, I would have preferred that some of
/// the functions and methods in the implementation block below be attached to
/// other types, but for the time being, they're all localized here.
///
/// Hopefully with future refactorings, we can consolidate some of these types
/// and logic more.
#[derive(Clone, Debug)]
pub struct AccessToken(String);

impl AccessToken {
    /// Constructor for creating a short-lived access token to be used by
    /// Builder workers when running builds.
    pub fn bldr_token(key_cache: &KeyCache) -> Result<Self> {
        Self::generate_access_token(key_cache,
                                    BUILDER_ACCOUNT_ID,
                                    FeatureFlags::all().bits(),
                                    Duration::hours(BUILDER_TOKEN_LIFETIME_HOURS))
    }

    /// Constructor used for creating never-expiring access tokens for "normal"
    /// user accounts.
    ///
    /// Currently , user tokens never expire, and can only be revoked.
    pub fn user_token(key_cache: &KeyCache, account_id: u64, privileges: u32) -> Result<Self> {
        Self::generate_access_token(key_cache, account_id, privileges, Duration::max_value())
    }

    /// Given the string form of an `AccessToken`, fully process it to yield an
    /// `originsrv::Session` struct.
    ///
    /// See the type-level documentation for additional details.
    pub fn validate_access_token(token: &str, key_cache: &KeyCache) -> Result<originsrv::Session> {
        // Parse the input as an AccessToken.
        let token: Self = token.parse()?;

        // Decrypt the contents to get the `originsrv::AccessToken`
        // protobuf inside.
        let payload = token.decrypt(key_cache)?;

        // Ensure that the token has not expired yet.
        //
        // TODO (CM - 2020-09-15): This logic would be better as an `expired`
        // method on the `originsrv::AccessToken` type itself.
        match Utc.timestamp_opt(payload.get_expires(), 0 /* nanoseconds */) {
            Single(expires) => {
                if expires < Utc::now() {
                    return Err(Error::TokenExpired);
                }
            }
            _ => return Err(Error::TokenInvalid),
        }

        // If all is OK, finally convert into an `originsrv::Session`.
        Ok(payload.into())
    }

    ////////////////////////////////////////////////////////////////////////

    /// Helper function with common logic creating an `AccessToken` from all the
    /// necessary inputs.
    fn generate_access_token(key_cache: &KeyCache,
                             account_id: u64,
                             flags: u32,
                             lifetime: Duration)
                             -> Result<Self> {
        // Create originsrv::AccessToken protobuf struct
        let token = AccessToken::new_proto(account_id, flags, lifetime);

        // Encrypt that protobuf struct to a String.
        let token = AccessToken::encrypt(&token, key_cache)?;

        // Turn it into our general AccessToken domain object
        Ok(Self(token))
    }

    /// Create a fully-initialized `originsrv::AccessToken`. This will be
    /// encrypted to form the core payload of `AccessToken`.
    ///
    /// Ideally, this would be a function on the originsrv::AccessToken struct.
    ///
    /// Would call this function `new`, but that's already taken by the
    /// protobuf-generated code :/
    fn new_proto(account_id: u64, flags: u32, lifetime: Duration) -> originsrv::AccessToken {
        let expires = Utc::now().checked_add_signed(lifetime)
                                .unwrap_or_else(|| chrono::MAX_DATE.and_hms(0, 0, 0))
                                .timestamp();

        let mut token = originsrv::AccessToken::new();
        token.set_account_id(account_id);
        token.set_flags(flags);
        token.set_expires(expires);

        token
    }

    /// Given an `originsrv::AccessToken`, encrypt it to form the payload of
    /// `AccessToken`.
    ///
    /// Ideally, this would be a function on the originsrv::AccessToken struct.
    fn encrypt(proto_token: &originsrv::AccessToken, key_cache: &KeyCache) -> Result<String> {
        let bytes = message::encode(proto_token).map_err(Error::Protocol)?;
        let (token_value, _) = crypto::encrypt(&key_cache, bytes)?;
        Ok(token_value)
    }

    /// Given an `AccessToken`, decrypt the contents to yield the
    /// original `originsrv::AccessToken`.
    fn decrypt(&self, key_cache: &KeyCache) -> Result<originsrv::AccessToken> {
        let bytes = crypto::decrypt(&key_cache, &self.0)?;
        let payload: originsrv::AccessToken =
            message::decode(&bytes).map_err(|e| {
                                       warn!("Unable to deserialize access token, err={:?}", e);
                                       Error::TokenInvalid
                                   })?;
        Ok(payload)
    }
}

impl fmt::Display for AccessToken {
    /// Ultimately responsible for rendering an access token as
    /// something like:
    ///
    ///    _Qk9YLTFuYmxkci0yMAo=
    ///
    /// (but longer)
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", ACCESS_TOKEN_PREFIX, base64::encode(&self.0))
    }
}

impl FromStr for AccessToken {
    type Err = Error;

    /// Parses an `AccessToken` from a string. Does *not* attempt to do any
    /// decrypting of its inner payload, nor any checking to see if the token
    /// has already expired.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if let Some(payload) = s.strip_prefix(ACCESS_TOKEN_PREFIX) {
            let encrypted = base64::decode(payload).map(String::from_utf8)??;

            // Though the fact that we're encrypting as a `SignedBox` for this
            // application is not terribly important, the fact that the string
            // value does represent encrypted content of *some* kind *is*
            // important. We don't want bogus tokens from getting any farther
            // into the system than necessary.
            //
            // At the *very* least, the string content should be parsable as a
            // SignedBox. (You could additionally go further and assert that the
            // encryptor and decryptor are the same, and that the name of the
            // keys was "bldr", but we can hold on that for the moment.)
            //
            // See documentation comments on builder_core::crypto::encrypt as
            // well.
            if encrypted.parse::<SignedBox>().is_err() {
                Err(Error::TokenInvalid)
            } else {
                Ok(Self(encrypted))
            }
        } else {
            Err(Error::TokenInvalid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use habitat_core::crypto::keys::KeyCache;
    use tempfile::{Builder,
                   TempDir};

    /// Create a new KeyCache *with a new Builder encryption key* for use in
    /// tests.
    ///
    /// Returns the `TempDir` that backs the cache to prevent it from getting
    /// `Drop`ped too early; feel free to ignore it.
    ///
    /// (Identical to the function from the `habitat_core` crate, but adds the
    /// Builder key for convenience here.)
    fn new_cache() -> (KeyCache, TempDir) {
        let dir = Builder::new().prefix("key_cache").tempdir().unwrap();
        let cache = KeyCache::new(dir.path());
        // Not strictly required, of course, since we know we just
        // created the directory.
        cache.setup().unwrap();

        // Create a new bldr encryption key, since tests are going to need one.
        let bldr_key = habitat_core::crypto::keys::generate_builder_encryption_key();
        cache.write_key(&bldr_key).unwrap();
        (cache, dir)
    }

    /// While we won't necessarily always do signed-box encryption for tokens,
    /// we currently do. Thus, this test is useful for confirming what *kind* of
    /// `String` the `AccessToken` type currently wraps.
    ///
    /// Consider it another form of documentation.
    #[test]
    fn token_struct_contains_encrypted_string() {
        let (cache, _dir) = new_cache();
        let token = AccessToken::bldr_token(&cache).unwrap();

        let encrypted = token.0;
        let parsed = encrypted.parse::<SignedBox>();
        assert!(parsed.is_ok(),
                "Expected '{}' to parse as a SignedBox, but it didn't!",
                encrypted);
    }

    #[test]
    fn creates_builder_token() {
        let (cache, _dir) = new_cache();
        let token = AccessToken::bldr_token(&cache).unwrap();

        let inner = token.decrypt(&cache).unwrap();

        assert_eq!(inner.get_account_id(),
                   BUILDER_ACCOUNT_ID,
                   "Builder tokens should be for the Builder account only");
        assert_eq!(inner.get_flags(),
                   FeatureFlags::all().bits(),
                   "Builder tokens should have all flags enabled");

        // Expiration times are given as seconds-past-the-epoch. Here, we figure
        // out what that will be 2 hours from now. Generally, the token's
        // expiration should exactly be what we compute here, but we'll give 1
        // second of wiggle room to account for slow machines, as well as cases
        // where we happen to generate a token on the last nanosecond of a
        // second.
        let expected_expiration =
            Utc::now().checked_add_signed(Duration::hours(BUILDER_TOKEN_LIFETIME_HOURS))
                      .unwrap()
                      .timestamp();
        let upper_bound = expected_expiration;
        let lower_bound = expected_expiration - 1;
        let acceptable_range = lower_bound..=upper_bound;

        assert!(acceptable_range.contains(&inner.get_expires()),
                "Builder tokens should expire in 2 hours (expected {}, got {})",
                expected_expiration,
                inner.get_expires());
    }

    #[test]
    fn creates_user_token() {
        let (cache, _dir) = new_cache();
        let account_id = 2112;
        let privileges = FeatureFlags::default().bits();

        let token = AccessToken::user_token(&cache, account_id, privileges).unwrap();

        let inner = token.decrypt(&cache).unwrap();

        assert_eq!(inner.get_account_id(), account_id);
        assert_eq!(inner.get_flags(), privileges);

        // December 31, 262143 CE... see `chrono::naive::date::MAX_DATE`
        // User tokens essentially never expire.
        let maximum_time = 8_210_298_326_400;
        assert_eq!(inner.get_expires(), maximum_time);
    }

    mod validate_access_token {
        use super::*;

        #[test]
        fn new_token_validates() {
            let (cache, _dir) = new_cache();
            let token = AccessToken::bldr_token(&cache).unwrap();

            assert!(AccessToken::validate_access_token(&token.to_string(), &cache).is_ok());
        }

        #[test]
        fn expired_token_does_not_validate() {
            let (cache, _dir) = new_cache();
            let account_id = 2001;
            let flags = FeatureFlags::all().bits();

            // This token is valid only for the second in which it is created.
            let lifetime = Duration::seconds(0);

            // Using private `generate_access_token` function here to gain control
            // of the token duration; the public constructors hide this.
            let token =
                AccessToken::generate_access_token(&cache, account_id, flags, lifetime).unwrap();

            // Sleep to ensure enough time has passed for the token to definitely be
            // marked as expired.
            std::thread::sleep(std::time::Duration::from_secs(2));

            assert!(AccessToken::validate_access_token(&token.to_string(), &cache).is_err(),
                    "Expired tokens can never validate!");
        }
    }

    mod display {
        use super::*;

        #[test]
        fn token_string_structure_is_correct() {
            let (cache, _dir) = new_cache();
            let token = AccessToken::bldr_token(&cache).unwrap();
            let token = token.to_string();

            assert!(token.starts_with('_'), "Token must start with a '_'");

            let rest_of_token = token.trim_start_matches('_');
            assert!(base64::decode(rest_of_token).is_ok(),
                    "Token after '_' must be base64-encoded")
        }
    }

    mod fromstr {
        use super::*;

        // This token was created during an ephemeral test; the key used to
        // encrypt it was also created solely for that ephemeral test, and no
        // longer exists. This is a valid token, but can no longer be decrypted.
        const SAMPLE_TOKEN: &str = "_Qk9YLTEKYmxkci0yMDIwMDkxNTE2MDkyMQpibGRyLTIwMjAwOTE1MTYwOTIxCkc1amVzMVVsdm1xWE0xZW9rWXYyM202ODB0aVVxWjRiCnp4ZlFoZE9FTEVUOXViQ2VWRVBaVnJVR0p0YWhTL2JtYmxDcmxpdz0=";

        #[test]
        fn parse() {
            let parsed = SAMPLE_TOKEN.parse::<AccessToken>();
            assert!(parsed.is_ok(), "Should be able to parse a user token");
        }

        #[test]
        fn token_must_start_with_appropriate_prefix_to_parse() {
            let truncated = SAMPLE_TOKEN.trim_start_matches('_');
            let parsed = truncated.parse::<AccessToken>();
            assert!(parsed.is_err(), "Token must start with '_'");
        }

        #[test]
        fn token_must_be_base64() {
            let bad_token = "_itcouldbeatokenbuta space characterisnotvalidbase64";
            let parsed = bad_token.parse::<AccessToken>();
            println!("parsed: {:?}", parsed);
            match parsed {
                Err(Error::Base64Error(base64::DecodeError::InvalidByte(..))) => { /* Ok! */ }
                _ => panic!("expected base64 decoding error"),
            }
        }

        #[test]
        fn cannot_take_just_any_base64_content_as_a_token() {
            let bad_token = "_DEADBEEFDEADBEEFDEADBEEF";
            let parsed = bad_token.parse::<AccessToken>();
            assert!(parsed.is_err(),
                    "Not just any base64 string can parse as a token");
        }

        #[test]
        fn base64_content_must_be_a_signed_box() {
            let (cache, _dir) = new_cache();
            let key = cache.latest_builder_key().unwrap();

            // This should generate a structurally valid token, in that it is a
            // base64-encoded SignedBox. Without decrypting it, however, we
            // can't say whether it is actually an encrypted
            // `originsrv::AuthToken`.
            //
            // This is the best we can do with `std::str::FromStr`, though.
            let encrypted: SignedBox = key.encrypt("supersecretstuff");
            let b64 = base64::encode(encrypted.to_string());
            let token = format!("_{}", b64);
            let parsed = token.parse::<AccessToken>();
            assert!(parsed.is_ok(), "It should parse because it's encrypted");

            assert!(AccessToken::validate_access_token(&token, &cache).is_err(),
                    "There is no way this token could ever validate, because the right data \
                     wasn't encrypted to begin with");
        }
    }
}
