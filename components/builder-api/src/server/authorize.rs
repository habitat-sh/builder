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

use std::str::FromStr;

use actix_web::HttpRequest;

use crate::{bldr_core::{access_token::BUILDER_ACCOUNT_ID,
                        metrics::CounterMetric,
                        privilege::*},
            db::models::origin::*,
            protocol::originsrv};

use crate::server::{error::{Error,
                            Result},
                    helpers::req_state,
                    services::metrics::Counter};

pub fn authorize_session(req: &HttpRequest,
                         origin_opt: Option<&str>,
                         min_role: Option<OriginMemberRole>)
                         -> Result<originsrv::Session> {
    let session = {
        let extensions = req.extensions();
        match extensions.get::<originsrv::Session>() {
            Some(session) => {
                let flags = FeatureFlags::from_bits(session.get_flags()).unwrap(); // unwrap Ok
                if flags.contains(FeatureFlags::BUILD_WORKER) {
                    debug!("authorize_session: detected allowed BUILD_WORKER");
                    return Ok(session.clone());
                }
                debug!("authorize_session: found session {}", session.get_id());
                session.clone()
            }
            None => {
                debug!("authorize_session: unable to get session!");
                return Err(Error::Authentication);
            }
        }
    };

    if let Some(origin) = origin_opt {
        let minimum_req_role = match min_role {
            Some(r) => r,
            None => {
                let r = OriginMemberRole::Maintainer;
                // TODO: When we have finalized implementation of the various member roles,
                // we should turn this into a warn level message.
                // see: https://github.com/habitat-sh/builder/issues/1403
                debug!("authorize_session: minimum role parameter not set! Assuming {}",
                       r);
                r
            }
        };
        match check_origin_member_role(req, origin, session.get_id()) {
            Some(member_role) => {
                if member_role >= minimum_req_role {
                    debug!("authorize_session: account {} has {} permissions in origin {}",
                           session.get_id(),
                           minimum_req_role,
                           origin);
                    return Ok(session);
                } else {
                    debug!("authorize_session: account {} does not have {} permissions in origin \
                            {}. Current role: {}",
                           session.get_id(),
                           minimum_req_role,
                           origin,
                           member_role);
                    return Err(Error::Authorization);
                }
            }
            None => {
                debug!("authorize_session: account {} is not a member of the origin {}",
                       session.get_id(),
                       origin);
                return Err(Error::Authorization);
            }
        }
    }
    Ok(session)
}

pub fn check_origin_owner(req: &HttpRequest, account_id: u64, origin: &str) -> Result<bool> {
    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    match Origin::get(origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => Ok(origin.owner_id == account_id as i64),
        Err(err) => Err(err),
    }
}

pub fn check_origin_member(req: &HttpRequest, origin: &str, account_id: u64) -> Result<bool> {
    if account_id == BUILDER_ACCOUNT_ID {
        Ok(true)
    } else {
        let mut memcache = req_state(req).memcache.borrow_mut();
        match memcache.get_origin_member(origin, account_id) {
            Some(val) => {
                debug!("Origin membership {} {} Cache Hit!", origin, account_id);
                return Ok(val);
            }
            None => debug!("Origin membership {} {} Cache Miss!", origin, account_id),
        }
        let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;
        match Origin::check_membership(origin, account_id as i64, &*conn).map_err(Error::DieselError) {
            Ok(is_member) => {
                memcache.set_origin_member(origin, account_id, is_member);
                debug!("Found member {} in origin {}", account_id, origin);
                Ok(is_member)
            }
            Err(err) => {
                warn!("Check membership error {}", err);
                Err(err)
            }
        }
    }
}

fn check_origin_member_role(req: &HttpRequest,
                            origin: &str,
                            account_id: u64)
                            -> Option<OriginMemberRole> {
    if account_id == BUILDER_ACCOUNT_ID {
        Some(OriginMemberRole::Owner)
    } else {
        let mut memcache = req_state(req).memcache.borrow_mut();
        match memcache.get_origin_member_role(origin, account_id) {
            Some(val) => {
                Counter::MemcacheMemberRoleHit.increment();
                debug!("Origin role membership {} {} Cache Hit!",
                       origin, account_id);
                match OriginMemberRole::from_str(&val) {
                    Ok(role) => return Some(role),
                    Err(_) => debug!("Unable to unwrap role from memcache!"),
                }
            }
            None => {
                Counter::MemcacheMemberRoleMiss.increment();
                debug!("Origin role membership {} {} Cache Miss!",
                       origin, account_id);
            }
        }
        match req_state(req).db.get_conn() {
            Ok(conn) => {
                match OriginMember::member_role(origin, account_id as i64, &*conn) {
                    Ok(member_role) => {
                        memcache.set_origin_member_role(origin,
                                                        account_id,
                                                        &member_role.to_string());
                        debug!("Found account {} has member type {}",
                               account_id, member_role);
                        Some(member_role)
                    }
                    Err(err) => {
                        debug!("Unable to determine member type for account {} in origin {}. \
                                More than likely they are simply not a member: {}",
                               account_id, origin, err);
                        None
                    }
                }
            }
            Err(err) => {
                warn!("Unable to retrieve request state: {}", err);
                None
            }
        }
    }
}
