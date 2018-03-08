// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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

use std::env;

use hab_net::app::prelude::*;
use hab_net::privilege::FeatureFlags;
use bldr_core;

use protocol::net;
use protocol::sessionsrv as proto;

use super::{encode_token, ServerState, Session};
use error::SrvResult;

pub fn account_get_id(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountGetId>()?;
    match state.datastore.get_account_by_id(&msg) {
        Ok(Some(account)) => conn.route_reply(req, &account)?,
        Ok(None) => {
            let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "ss:account-get-id:0");
            conn.route_reply(req, &*err)?;
        }
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-get-id:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_get(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountGet>()?;
    match state.datastore.get_account(&msg) {
        Ok(Some(account)) => conn.route_reply(req, &account)?,
        Ok(None) => {
            let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "ss:account-get:0");
            conn.route_reply(req, &*err)?;
        }
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-get:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_update(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountUpdate>()?;
    match state.datastore.update_account(&msg) {
        Ok(()) => conn.route_reply(req, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-update:0");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_create(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountCreate>()?;
    match state.datastore.create_account(&msg) {
        Ok(account) => conn.route_reply(req, &account)?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-create:0");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_find_or_create(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountFindOrCreate>()?;
    match state.datastore.account_find_or_create(&msg) {
        Ok(account) => conn.route_reply(req, &account)?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-foc:0");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_token_create(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountTokenCreate>()?;
    let account_id = msg.get_account_id();

    {
        state.tokens.write().unwrap().remove(&account_id);
    }

    match state.datastore.create_account_token(&msg) {
        Ok(account_token) => conn.route_reply(req, &account_token)?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-token-create:0");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_token_revoke(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountTokenRevoke>()?;

    let mut msg_get = proto::AccountTokenGet::new();
    msg_get.set_id(msg.get_id());

    let account_id = match state.datastore.get_account_token(&msg_get) {
        Ok(account_token) => account_token.get_account_id(),
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-token-revoke:0");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
            return Ok(());
        }
    };

    {
        state.tokens.write().unwrap().remove(&account_id);
    }

    match state.datastore.revoke_account_token(&msg) {
        Ok(_) => conn.route_reply(req, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-token-revoke:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_tokens_get(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountTokensGet>()?;
    match state.datastore.get_account_tokens(&msg) {
        Ok(account_tokens) => conn.route_reply(req, &account_tokens)?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-tokens-get:0");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_token_validate(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountTokenValidate>()?;

    let account_id = msg.get_account_id();

    // Builder tokens are never in the DB and are not revocable
    if account_id == bldr_core::access_token::BUILDER_ACCOUNT_ID {
        conn.route_reply(req, &NetOk::new())?;
        return Ok(());
    }

    cache_tokens_for_account(state, account_id)?; // Pre-emptively populate cache if needed

    let is_valid = match state.tokens.read().unwrap().get(&account_id) {
        Some(&Some(ref token)) => token == msg.get_token(),
        Some(&None) => false,
        None => panic!("Not reachable!"),
    };

    if is_valid {
        conn.route_reply(req, &NetOk::new())?
    } else {
        let err = NetError::new(ErrCode::DATA_STORE, "ss:account-tokens-validate:0");
        conn.route_reply(req, &*err)?;
    }

    Ok(())
}

pub fn cache_tokens_for_account(state: &mut ServerState, account_id: u64) -> SrvResult<()> {
    let needs_cache_entry = {
        state.tokens.read().unwrap().get(&account_id).is_none()
    };

    if needs_cache_entry {
        let mut tokens = state.tokens.write().unwrap();
        let mut msg = proto::AccountTokensGet::new();
        msg.set_account_id(account_id);

        match state.datastore.get_account_tokens(&msg) {
            Ok(account_tokens) => {
                assert!(account_tokens.get_tokens().len() <= 1); // Can only have max of 1 for now
                if account_tokens.get_tokens().is_empty() {
                    tokens.insert(account_id, None);
                } else {
                    tokens.insert(
                        account_id,
                        Some(
                            account_tokens
                                .get_tokens()
                                .first()
                                .unwrap()
                                .get_token()
                                .to_owned(),
                        ),
                    );
                }
            }
            Err(err) => {
                warn!("Unable to fetch account tokens: {:?}", err);
                return Err(err);
            }
        }
    }

    Ok(())
}

pub fn session_create(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let mut msg = req.parse::<proto::SessionCreate>()?;
    debug!("session-create, {:?}", msg);
    let mut flags = FeatureFlags::default();
    if env::var_os("HAB_FUNC_TEST").is_some() {
        flags = FeatureFlags::empty();
    } else if msg.get_session_type() == proto::SessionType::Builder {
        flags = FeatureFlags::all();
    } else if msg.get_provider() == proto::OAuthProvider::GitHub {
        assign_permissions(msg.get_name(), &mut flags, state)
    }

    let account = if msg.get_session_type() == proto::SessionType::Builder {
        let mut account = proto::Account::new();
        account.set_id(0);
        account.set_email(msg.take_email());
        account.set_name(msg.take_name());
        account
    } else {
        let mut account_req = proto::AccountFindOrCreate::default();
        account_req.set_name(msg.take_name());
        account_req.set_email(msg.take_email());

        match conn.route::<proto::AccountFindOrCreate, proto::Account>(&account_req) {
            Ok(account) => account,
            Err(e) => {
                let err = NetError::new(ErrCode::DATA_STORE, "ss:session-create:5");
                error!("{}, {}", e, err);
                conn.route_reply(req, &*err)?;
                return Ok(());
            }
        }
    };

    let session = Session::build(msg, account, flags)?;
    {
        debug!("issuing session, {:?}", session);
        state.sessions.write().unwrap().insert(session.clone());
    }
    conn.route_reply(req, &*session)?;
    Ok(())
}

pub fn session_get(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::SessionGet>()?;
    let token = encode_token(msg.get_token())?;
    let expire_session = {
        match state.sessions.read().unwrap().get(token.as_str()) {
            Some(session) => {
                if session.expired() {
                    true
                } else {
                    conn.route_reply(req, &**session)?;
                    false
                }
            }
            None => {
                let err = NetError::new(ErrCode::SESSION_EXPIRED, "ss:session-get:0");
                conn.route_reply(req, &*err)?;
                false
            }
        }
    };
    // JW TODO: We should renew the session if it's within X time of expiring since the
    // user just confirmed they're still using this session.
    if expire_session {
        state.sessions.write().unwrap().remove(token.as_str());
    }
    Ok(())
}

pub fn account_origin_invitation_create(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountOriginInvitationCreate>()?;
    match state.datastore.create_account_origin_invitation(&msg) {
        Ok(()) => conn.route_reply(req, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-origin-invitation-create:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_invitation_accept(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountOriginInvitationAcceptRequest>()?;
    match state.datastore.accept_origin_invitation(&msg) {
        Ok(()) => conn.route_reply(req, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-origin-invitation-accept:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_invitation_ignore(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountOriginInvitationIgnoreRequest>()?;
    match state.datastore.ignore_origin_invitation(&msg) {
        Ok(()) => conn.route_reply(req, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-origin-invitation-ignore:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_invitation_rescind(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountOriginInvitationRescindRequest>()?;
    match state.datastore.rescind_origin_invitation(&msg) {
        Ok(()) => conn.route_reply(req, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(
                ErrCode::DATA_STORE,
                "ss:account-origin-invitation-rescind:1",
            );
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_create(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountOriginCreate>()?;
    match state.datastore.create_origin(&msg) {
        Ok(()) => conn.route_reply(req, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-origin-create:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_remove(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountOriginRemove>()?;
    match state.datastore.delete_origin(&msg) {
        Ok(()) => conn.route_reply(req, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-origin-remove:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_list_request(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountOriginListRequest>()?;
    match state.datastore.get_origins_by_account(&msg) {
        Ok(reply) => conn.route_reply(req, &reply)?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-origin-list-request:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

pub fn account_invitation_list(
    req: &mut Message,
    conn: &mut RouteConn,
    state: &mut ServerState,
) -> SrvResult<()> {
    let msg = req.parse::<proto::AccountInvitationListRequest>()?;
    match state.datastore.list_invitations(&msg) {
        Ok(response) => conn.route_reply(req, &response)?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-invitation-list:1");
            error!("{}, {}", e, err);
            conn.route_reply(req, &*err)?;
        }
    }
    Ok(())
}

fn assign_permissions(name: &str, flags: &mut FeatureFlags, state: &ServerState) {
    let installation_id = state.permissions.app_install_id;
    debug!(
        "GITHUB-CALL builder_sessionsrv::server::handlers::assign_permissions: Getting app_installation_token; installation_id={}",
        installation_id
    );
    match state.github.app_installation_token(
        state.permissions.app_install_id,
    ) {
        Ok(token) => {
            debug!(
                "GITHUB-CALL builder_sessionsrv::server::handlers::assign_permissions: Checking team membership for {}",
                name
            );
            match state.github.check_team_membership(
                &token,
                state.permissions.admin_team,
                name,
            ) {
                Ok(Some(membership)) => {
                    if membership.active() {
                        debug!("Granting feature flag={:?}", FeatureFlags::ADMIN);
                        flags.set(FeatureFlags::ADMIN, true);
                    }
                }
                Ok(None) => (),
                Err(err) => warn!("Failed to check github team membership, {}", err),
            }
            for team in state.permissions.early_access_teams.iter() {
                debug!(
                    "GITHUB-CALL builder_sessionsrv::server::handlers::assign_permissions: Checking team membership for {}",
                    name
                );
                match state.github.check_team_membership(&token, *team, name) {
                    Ok(Some(membership)) => {
                        if membership.active() {
                            debug!("Granting feature flag={:?}", FeatureFlags::EARLY_ACCESS);
                            flags.set(FeatureFlags::EARLY_ACCESS, true);
                            break;
                        }
                    }
                    Ok(None) => (),
                    Err(err) => warn!("Failed to check github team membership, {}", err),
                }
            }
            for team in state.permissions.build_worker_teams.iter() {
                debug!(
                    "GITHUB-CALL builder_sessionsrv::server::handlers::assign_permissions: Checking team membership for {}",
                    name
                );
                match state.github.check_team_membership(&token, *team, name) {
                    Ok(Some(membership)) => {
                        if membership.active() {
                            debug!("Granting feature flag={:?}", FeatureFlags::BUILD_WORKER);
                            flags.set(FeatureFlags::BUILD_WORKER, true);
                            break;
                        }
                    }
                    Ok(None) => (),
                    Err(err) => warn!("Failed to check github team membership, {}", err),
                }
            }
        }
        Err(err) => warn!("Failed to obtain installation token, {}", err),
    }
}
