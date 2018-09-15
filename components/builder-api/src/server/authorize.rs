// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
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

use actix_web::HttpRequest;
use hab_net::privilege::FeatureFlags;

use bldr_core::access_token::{BUILDER_ACCOUNT_ID, BUILDER_ACCOUNT_NAME};
use protocol::originsrv::*;

use server::error::{Error, Result};
use server::framework::middleware::route_message;
use server::AppState;

pub fn authorize_session(req: &HttpRequest<AppState>, origin_opt: Option<&str>) -> Result<u64> {
    let account_id = {
        let extensions = req.extensions();
        match extensions.get::<Session>() {
            Some(session) => {
                let flags = FeatureFlags::from_bits(session.get_flags()).unwrap(); // unwrap Ok
                if flags.contains(FeatureFlags::BUILD_WORKER) {
                    return Ok(session.get_id());
                }
                session.get_id()
            }
            None => return Err(Error::Authentication),
        }
    };

    if let Some(origin) = origin_opt {
        let mut request = CheckOriginAccessRequest::new();
        request.set_account_id(account_id);
        request.set_origin_name(origin.to_string());

        match route_message::<CheckOriginAccessRequest, CheckOriginAccessResponse>(req, &request) {
            Ok(ref response) if response.get_has_access() => (),
            _ => return Err(Error::Authorization),
        }
    }

    Ok(account_id)
}

// TODO - Merge into authorize_session when we are able to cache the name
pub fn get_session_user_name(req: &HttpRequest<AppState>, account_id: u64) -> String {
    if account_id == BUILDER_ACCOUNT_ID {
        return BUILDER_ACCOUNT_NAME.to_string();
    }

    let mut msg = AccountGetId::new();
    msg.set_id(account_id);

    match route_message::<AccountGetId, Account>(req, &msg) {
        Ok(account) => account.get_name().to_string(),
        Err(err) => {
            warn!("Failed to get account, id={}, err={:?}", account_id, err);
            "".to_string()
        }
    }
}

pub fn check_origin_owner(
    req: &HttpRequest<AppState>,
    account_id: u64,
    origin: &str,
) -> Result<bool> {
    let mut request = CheckOriginOwnerRequest::new();
    request.set_account_id(account_id);
    request.set_origin_name(origin.to_string());
    match route_message::<CheckOriginOwnerRequest, CheckOriginOwnerResponse>(req, &request) {
        Ok(response) => Ok(response.get_is_owner()),
        Err(err) => Err(err),
    }
}
