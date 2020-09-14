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
use habitat_core::crypto::keys::KeyCache;
use std::str::FromStr;

pub const BUILDER_ACCOUNT_ID: u64 = 0;
pub const BUILDER_ACCOUNT_NAME: &str = "BUILDER";

// Access token prefix rules:
// MUST CONTAIN AN *INVALID* base-64 character
// MUST NOT CONTAIN shell special characters (eg, !)
// SHOULD be URL-safe (just in case)
const ACCESS_TOKEN_PREFIX: &str = "_";

const BUILDER_TOKEN_LIFETIME_HOURS: i64 = 2;

/// The string form of the encrypted OriginSrv::AccessToken type
#[derive(Clone, Debug)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn bldr_token(key_cache: &KeyCache) -> Result<Self> {
        Self::generate_access_token(key_cache,
                                    BUILDER_ACCOUNT_ID,
                                    FeatureFlags::all().bits(),
                                    Duration::hours(BUILDER_TOKEN_LIFETIME_HOURS))
    }

    ////////////////////////////////////////////////////////////////////////

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

    // Ideally, this would be a function on the originsrv::AccessToken
    // struct.
    //
    // Would call this function `new`, but that's taken by the
    // protobuf-generated code :/
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

    // Ideally, this would be a function on the originsrv::AccessToken
    // struct.
    fn encrypt(proto_token: &originsrv::AccessToken, key_cache: &KeyCache) -> Result<String> {
        let bytes = message::encode(proto_token).map_err(Error::Protocol)?;
        let (token_value, _) = crypto::encrypt(&key_cache, bytes)?;
        Ok(token_value)
    }

impl fmt::Display for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", ACCESS_TOKEN_PREFIX, base64::encode(&self.0))
    }
}

}

pub fn generate_user_token(key_cache: &KeyCache,
                           account_id: u64,
                           privileges: u32)
                           -> Result<String> {
    generate_access_token(key_cache,
                          account_id,
                          privileges,
                          Duration::max_value() /* User tokens never expire, can only be revoked */)
}

fn generate_access_token(key_cache: &KeyCache,
                         account_id: u64,
                         flags: u32,
                         lifetime: Duration)
                         -> Result<String> {
    let expires = Utc::now().checked_add_signed(lifetime)
                            .unwrap_or_else(|| chrono::MAX_DATE.and_hms(0, 0, 0))
                            .timestamp();

    let mut token = originsrv::AccessToken::new();
    token.set_account_id(account_id);
    token.set_flags(flags);
    token.set_expires(expires);

    let bytes = message::encode(&token).map_err(Error::Protocol)?;
    let (token_value, _) = crypto::encrypt(&key_cache, bytes)?;
    let token_value = base64::encode(token_value);

    Ok(format!("{}{}", ACCESS_TOKEN_PREFIX, token_value))
}

pub fn is_access_token(token: &str) -> bool { token.starts_with(ACCESS_TOKEN_PREFIX) }

/// Decrypts a token to get a valid `Session`.
pub fn validate_access_token(key_cache: &KeyCache, token: &str) -> Result<originsrv::Session> {
    let encrypted = base64::decode(&token[ACCESS_TOKEN_PREFIX.len()..]).map(String::from_utf8)??;
    let bytes = crypto::decrypt(&key_cache, &encrypted)?;

    let payload: originsrv::AccessToken = match message::decode(&bytes) {
        Ok(p) => p,
        Err(e) => {
            warn!("Unable to deserialize access token, err={:?}", e);
            return Err(Error::TokenInvalid);
        }
    };

    match Utc.timestamp_opt(payload.get_expires(), 0 /* nanoseconds */) {
        Single(expires) => {
            if expires < Utc::now() {
                Err(Error::TokenExpired)
            } else {
                Ok(payload.into())
            }
        }
        _ => Err(Error::TokenInvalid),
    }
}
