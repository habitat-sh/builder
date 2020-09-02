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

pub const BUILDER_ACCOUNT_ID: u64 = 0;
pub const BUILDER_ACCOUNT_NAME: &str = "BUILDER";

// Access token prefix rules:
// MUST CONTAIN AN *INVALID* base-64 character
// MUST NOT CONTAIN shell special characters (eg, !)
// SHOULD be URL-safe (just in case)
const ACCESS_TOKEN_PREFIX: &str = "_";

const BUILDER_TOKEN_LIFETIME_HOURS: i64 = 2;

pub fn generate_bldr_token(key_cache: &KeyCache) -> Result<String> {
    generate_access_token(key_cache,
                          BUILDER_ACCOUNT_ID,
                          FeatureFlags::all().bits(),
                          Duration::hours(BUILDER_TOKEN_LIFETIME_HOURS))
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
    Ok(format!("{}{}", ACCESS_TOKEN_PREFIX, token_value))
}

pub fn is_access_token(token: &str) -> bool { token.starts_with(ACCESS_TOKEN_PREFIX) }

/// Decrypts a token to get a valid `Session`.
pub fn validate_access_token(key_cache: &KeyCache, token: &str) -> Result<originsrv::Session> {
    let bytes = crypto::decrypt(&key_cache, &token[ACCESS_TOKEN_PREFIX.len()..])?;

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
