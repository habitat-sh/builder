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

use protocol::originsrv;

use db::models::account::*;
use db::models::origin::*;

use server::error::{Error, Result};
use server::AppState;

pub fn authorize_session(req: &HttpRequest<AppState>, origin_opt: Option<&str>) -> Result<u64> {
    let account_id = {
        let extensions = req.extensions();
        match extensions.get::<originsrv::Session>() {
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

    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    if let Some(origin) = origin_opt {
        match Origin::check_membership(origin, account_id, &*conn).map_err(Error::DieselError) {
            Ok(is_member) if is_member => (),
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

    let conn = match req.state().db.get_conn() {
        Ok(conn) => conn,
        Err(err) => {
            warn!("Failed to get account, id={}, err={:?}", account_id, err);
            return "".to_string();
        }
    };

    match Account::get_by_id(account_id, &*conn) {
        Ok(account) => account.name.to_string(),
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
    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    match Origin::get(origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => Ok(origin.owner_id == account_id as i64),
        Err(err) => Err(err),
    }
}
