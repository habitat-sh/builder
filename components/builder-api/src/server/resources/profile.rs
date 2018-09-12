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

use actix_web::http::{Method, StatusCode};
use actix_web::{App, FromRequest, HttpRequest, HttpResponse, Json, Path};

use bldr_core;
use hab_net::NetOk;
use protocol::originsrv::*;

use server::authorize::authorize_session;
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
        app.route("/profile", Method::GET, get_profile)
            .route("/profile", Method::PATCH, update_profile)
            .route("/profile/access-tokens", Method::GET, get_access_tokens)
            .route(
                "/profile/access-tokens",
                Method::POST,
                generate_access_token,
            )
            .route(
                "/profile/access-tokens/{id}",
                Method::DELETE,
                revoke_access_token,
            )
    }
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_profile(req: HttpRequest<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let mut request = AccountGetId::new();
    request.set_id(account_id);

    match route_message::<AccountGetId, Account>(&req, &request) {
        Ok(account) => HttpResponse::Ok().json(account),
        Err(err) => err.into(),
    }
}

fn get_access_tokens(req: HttpRequest<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let mut request = AccountTokensGet::new();
    request.set_account_id(account_id);

    match route_message::<AccountTokensGet, AccountTokens>(&req, &request) {
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

    let mut request = AccountTokenCreate::new();
    let token = bldr_core::access_token::generate_user_token(
        &req.state().config.api.key_path,
        account.get_id(),
        flags,
    ).unwrap();

    request.set_account_id(account.get_id());
    request.set_token(token);

    match route_message::<AccountTokenCreate, AccountToken>(&req, &request) {
        Ok(account_token) => HttpResponse::Ok().json(account_token),
        Err(err) => err.into(),
    }
}

fn revoke_access_token(req: HttpRequest<AppState>) -> HttpResponse {
    let token_id_str = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
    let token_id = match token_id_str.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
    };

    let mut request = AccountTokenRevoke::new();
    request.set_id(token_id);

    match route_message::<AccountTokenRevoke, NetOk>(&req, &request) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => err.into(),
    }
}

fn update_profile((req, body): (HttpRequest<AppState>, Json<UserUpdateReq>)) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    if body.email.len() <= 0 {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let mut request = AccountUpdate::new();
    request.set_id(account_id);
    request.set_email(body.email.to_owned());

    match route_message::<AccountUpdate, NetOk>(&req, &request) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => err.into(),
    }
}
