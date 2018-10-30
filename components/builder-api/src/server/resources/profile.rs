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

use bldr_core;
use protocol::originsrv;

use db::models::account::*;

use server::authorize::authorize_session;
use server::error::{Error, Result};
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
pub fn do_get_access_tokens(
    req: &HttpRequest<AppState>,
    account_id: u64,
) -> Result<Vec<AccountToken>> {
    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    AccountToken::list(account_id, &*conn).map_err(Error::DieselError)
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_account(req: HttpRequest<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(_err) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Account::get_by_id(account_id, &*conn).map_err(Error::DieselError) {
        Ok(account) => HttpResponse::Ok().json(account),
        Err(err) => err.into(),
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

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Memcache supports multiple tokens but to preserve legacy behavior
    // we must purge any existing tokens AFTER generating new ones
    let access_tokens = match AccountToken::list(account_id, &*conn).map_err(Error::DieselError) {
        Ok(access_tokens) => access_tokens,
        Err(err) => return err.into(),
    };

    // TODO: Provide an API for this
    let flags = {
        let extension = req.extensions();
        let session = extension.get::<originsrv::Session>().unwrap();
        session.get_flags()
    };

    let token = bldr_core::access_token::generate_user_token(
        &req.state().config.api.key_path,
        account_id,
        flags,
    ).unwrap();

    let new_token = NewAccountToken {
        account_id: account_id as i64,
        token: &token,
    };

    match AccountToken::create(&new_token, &*conn).map_err(Error::DieselError) {
        Ok(account_token) => {
            let mut memcache = req.state().memcache.borrow_mut();
            for token in access_tokens {
                memcache.delete_session_key(&token.token)
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

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let access_tokens = match AccountToken::list(account_id, &*conn).map_err(Error::DieselError) {
        Ok(access_tokens) => access_tokens,
        Err(err) => return err.into(),
    };

    match AccountToken::delete(token_id, &*conn).map_err(Error::DieselError) {
        Ok(_) => {
            let mut memcache = req.state().memcache.borrow_mut();
            for token in access_tokens {
                memcache.delete_session_key(&token.token)
            }
            HttpResponse::Ok().finish()
        }
        Err(err) => err.into(),
    }
}

fn update_account((req, body): (HttpRequest<AppState>, Json<UserUpdateReq>)) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(_err) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    if body.email.len() <= 0 {
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    match Account::update(account_id, &body.email, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => err.into(),
    }
}
