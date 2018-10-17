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
//
use actix_web::http::{Method, StatusCode};
use actix_web::{App, FromRequest, HttpRequest, HttpResponse, Json, Path};

use std::ops::Deref;

use bldr_core;
use hab_net::NetOk;
use protocol::originsrv::*;

use db::models::account::{Account as AccountModel, GetAccountById, UpdateAccount};
use server::authorize::authorize_session;
use server::error::Result;
use server::framework::middleware::route_message;
use server::AppState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserUpdateReq {
    #[serde(default)]
    pub email: String,
}

pub struct Profile {}

impl Profile {
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route("/profile", Method::GET, get_account)
            .route("/profile", Method::PATCH, update_account)
            .route("/profile/access-tokens", Method::GET, get_access_tokens)
            .route(
                "/profile/access-tokens",
                Method::POST,
                generate_access_token,
            ).route(
                "/profile/access-tokens/{id}",
                Method::DELETE,
                revoke_access_token,
            )
    }
}

// do_get_access_tokens is used in the framework middleware so it has to be public
pub fn do_get_access_tokens(req: &HttpRequest<AppState>, account_id: u64) -> Result<AccountTokens> {
    let mut request = AccountTokensGet::new();
    request.set_account_id(account_id);

    route_message::<AccountTokensGet, AccountTokens>(&req, &request)
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_account(req: HttpRequest<AppState>) -> HttpResponse {
    let session_id = match authorize_session(&req, None) {
        Ok(session_id) => session_id as i64,
        Err(_err) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    match AccountModel::get_by_id(
        GetAccountById {
            id: session_id.clone(),
        },
        conn.deref(),
    ) {
        Ok(account) => HttpResponse::Ok().json(account),
        Err(_e) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn get_access_tokens(req: HttpRequest<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    match do_get_access_tokens(&req, account_id) {
        Ok(account_tokens) => HttpResponse::Ok().json(account_tokens),
        Err(err) => err.into(),
    }
}

fn generate_access_token(req: HttpRequest<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    // TODO: Provide an API for this
    let flags = {
        let extension = req.extensions();
        let session = extension.get::<Session>().unwrap();
        session.get_flags()
    };

    let mut request = AccountGetId::new();
    request.set_id(account_id);

    let account = match route_message::<AccountGetId, Account>(&req, &request) {
        Ok(account) => account,
        Err(err) => return err.into(),
    };

    // Memcache supports multiple tokens but to preserve legacy behavior
    // we must purge any existing tokens AFTER generating new ones
    let access_tokens = match do_get_access_tokens(&req, account_id) {
        Ok(access_tokens) => access_tokens,
        Err(err) => return err.into(),
    };

    let mut request = AccountTokenCreate::new();
    let token = bldr_core::access_token::generate_user_token(
        &req.state().config.api.key_path,
        account.get_id(),
        flags,
    ).unwrap();

    request.set_account_id(account.get_id());
    request.set_token(token);

    match route_message::<AccountTokenCreate, AccountToken>(&req, &request) {
        Ok(account_token) => {
            let mut memcache = req.state().memcache.borrow_mut();
            for token in access_tokens.get_tokens() {
                memcache.delete_key(token.get_token())
            }
            HttpResponse::Ok().json(account_token)
        }
        Err(err) => err.into(),
    }
}

fn revoke_access_token(req: HttpRequest<AppState>) -> HttpResponse {
    let token_id_str = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
    let token_id = match token_id_str.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
    };

    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let access_tokens = match do_get_access_tokens(&req, account_id) {
        Ok(access_tokens) => access_tokens,
        Err(err) => return err.into(),
    };

    let mut request = AccountTokenRevoke::new();
    request.set_id(token_id);

    match route_message::<AccountTokenRevoke, NetOk>(&req, &request) {
        Ok(_) => {
            let mut memcache = req.state().memcache.borrow_mut();
            for token in access_tokens.get_tokens() {
                memcache.delete_key(token.get_token())
            }
            HttpResponse::Ok().finish()
        }
        Err(err) => err.into(),
    }
}

fn update_account((req, body): (HttpRequest<AppState>, Json<UserUpdateReq>)) -> HttpResponse {
    let session_id = match authorize_session(&req, None) {
        Ok(session_id) => session_id as i64,
        Err(_err) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    if body.email.len() <= 0 {
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    match AccountModel::update(
        UpdateAccount {
            id: session_id.clone(),
            email: body.email.to_owned(),
        },
        conn.deref(),
    ) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(_e) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
