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

use std::path::PathBuf;

use chrono::LocalResult::Single;
use chrono::{self, Duration, TimeZone, Utc};

use super::privilege::FeatureFlags;
use crate::error::{Error, Result};
use crate::integrations::{decrypt, encrypt, validate};
use crate::protocol::{message, originsrv};

pub const BUILDER_ACCOUNT_ID: u64 = 0;
pub const BUILDER_ACCOUNT_NAME: &str = "BUILDER";

// Access token prefix rules:
// MUST CONTAIN AN *INVALID* base-64 character
// MUST NOT CONTAIN shell special characters (eg, !)
// SHOULD be URL-safe (just in case)
const ACCESS_TOKEN_PREFIX: &str = "_";

const BUILDER_TOKEN_LIFETIME_HOURS: i64 = 2;

pub fn generate_bldr_token(key_dir: &PathBuf) -> Result<String> {
    generate_access_token(
        key_dir,
        BUILDER_ACCOUNT_ID,
        FeatureFlags::all().bits(),
        Duration::hours(BUILDER_TOKEN_LIFETIME_HOURS),
    )
}

pub fn generate_user_token(key_dir: &PathBuf, account_id: u64, privileges: u32) -> Result<String> {
    generate_access_token(
        key_dir,
        account_id,
        privileges,
        Duration::max_value(), // User tokens never expire, can only be revoked
    )
}

pub fn generate_access_token(
    key_dir: &PathBuf,
    account_id: u64,
    flags: u32,
    lifetime: Duration,
) -> Result<String> {
    let expires = Utc::now()
        .checked_add_signed(lifetime)
        .unwrap_or_else(|| chrono::MAX_DATE.and_hms(0, 0, 0))
        .timestamp();

    let mut token = originsrv::AccessToken::new();
    token.set_account_id(account_id);
    token.set_flags(flags);
    token.set_expires(expires);

    let bytes = message::encode(&token).map_err(Error::Protocol)?;
    let ciphertext = encrypt(key_dir, &bytes)?;

    Ok(format!("{}{}", ACCESS_TOKEN_PREFIX, ciphertext))
}

pub fn is_access_token(token: &str) -> bool {
    token.starts_with(ACCESS_TOKEN_PREFIX)
}

pub fn validate_access_token(key_dir: &PathBuf, token: &str) -> Result<originsrv::Session> {
    assert!(is_access_token(token));

    let bytes = decrypt(key_dir, &token[ACCESS_TOKEN_PREFIX.len()..])?;

    let payload: originsrv::AccessToken = match message::decode(&bytes) {
        Ok(p) => p,
        Err(e) => {
            warn!("Unable to deserialize access token, err={:?}", e);
            return Err(Error::TokenInvalid);
        }
    };

    if payload.get_account_id() == BUILDER_ACCOUNT_ID {
        validate(key_dir, &token[ACCESS_TOKEN_PREFIX.len()..])?
    }

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
