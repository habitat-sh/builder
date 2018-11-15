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
use bldr_core::privilege::*;

use protocol::originsrv;

use db::models::origin::*;

use server::error::{Error, Result};
use server::AppState;

pub fn authorize_session(
    req: &HttpRequest<AppState>,
    origin_opt: Option<&str>,
) -> Result<originsrv::Session> {
    let session = {
        let extensions = req.extensions();
        match extensions.get::<originsrv::Session>() {
            Some(session) => {
                let flags = FeatureFlags::from_bits(session.get_flags()).unwrap(); // unwrap Ok
                if flags.contains(BUILD_WORKER) {
                    return Ok(session.clone());
                }
                session.clone()
            }
            None => return Err(Error::Authentication),
        }
    };

    if let Some(origin) = origin_opt {
        let mut memcache = req.state().memcache.borrow_mut();

        match memcache.get_origin_member(origin, session.get_id()) {
            Some(val) => {
                trace!(
                    "Origin membership {} {} Cache Hit!",
                    origin,
                    session.get_id()
                );
                if val {
                    return Ok(session);
                } else {
                    return Err(Error::Authorization);
                }
            }
            None => trace!(
                "Origin membership {} {} Cache Miss!",
                origin,
                session.get_id()
            ),
        }

        match check_origin_member(req, origin, session.get_id()) {
            Ok(is_member) => {
                memcache.set_origin_member(origin, session.get_id(), is_member);

                match is_member {
                    true => (),
                    false => return Err(Error::Authorization),
                }
            }
            _ => return Err(Error::Authorization),
        }
    }

    Ok(session)
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

pub fn check_origin_member(
    req: &HttpRequest<AppState>,
    origin: &str,
    account_id: u64,
) -> Result<bool> {
    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    Origin::check_membership(origin, account_id as i64, &*conn).map_err(Error::DieselError)
}
