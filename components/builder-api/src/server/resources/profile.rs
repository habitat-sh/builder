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

use actix_web::http::{self, StatusCode};
use actix_web::{App, FromRequest, HttpRequest, HttpResponse, Json, Path};

use bldr_core;
use hab_net::NetOk;
use protocol::sessionsrv::*;

use server::error::Result;
use server::framework::middleware::{route_message, Authenticated};
use server::helpers;
use server::AppState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserUpdateReq {
    pub email: String,
}

pub struct Profile {}

impl Profile {
    //
    // Internal - these functions should return Result<..>
    //
    fn do_get_profile(req: &HttpRequest<AppState>) -> Result<Account> {
        let account_id = helpers::get_session_id(req);
        let mut request = AccountGetId::new();
        request.set_id(account_id);

        route_message::<AccountGetId, Account>(req, &request)
    }

    fn do_generate_access_token(req: &HttpRequest<AppState>) -> Result<AccountToken> {
        let (session_id, flags) = {
            let extension = req.extensions();
            let session = extension.get::<Session>().unwrap();
            (session.get_id(), session.get_flags())
        };

        let mut request = AccountGetId::new();
        request.set_id(session_id);

        let account = route_message::<AccountGetId, Account>(req, &request)?;

        let mut request = AccountTokenCreate::new();
        let token = bldr_core::access_token::generate_user_token(
            &req.state().config.api.key_path,
            account.get_id(),
            flags,
        ).unwrap();

        request.set_account_id(account.get_id());
        request.set_token(token);

        route_message::<AccountTokenCreate, AccountToken>(req, &request)
    }

    //
    // Route handlers - these functions should return HttpResponse
    //
    fn get_profile(req: &HttpRequest<AppState>) -> HttpResponse {
        match Self::do_get_profile(req) {
            Ok(account) => HttpResponse::Ok().json(account),
            Err(err) => err.into(),
        }
    }

    fn get_access_tokens(req: &HttpRequest<AppState>) -> HttpResponse {
        let account_id = helpers::get_session_id(req);

        let mut request = AccountTokensGet::new();
        request.set_account_id(account_id);

        match route_message::<AccountTokensGet, AccountTokens>(req, &request) {
            Ok(account_tokens) => HttpResponse::Ok().json(account_tokens),
            Err(err) => err.into(),
        }
    }

    fn generate_access_token(req: &HttpRequest<AppState>) -> HttpResponse {
        match Self::do_generate_access_token(req) {
            Ok(account_token) => HttpResponse::Ok().json(account_token),
            Err(err) => err.into(),
        }
    }

    fn revoke_access_token(req: &HttpRequest<AppState>) -> HttpResponse {
        let token_id_str = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok
        let token_id = match token_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
        };

        let mut request = AccountTokenRevoke::new();
        request.set_id(token_id);

        match route_message::<AccountTokenRevoke, NetOk>(req, &request) {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(err) => err.into(),
        }
    }

    fn update_profile((req, body): (HttpRequest<AppState>, Json<UserUpdateReq>)) -> HttpResponse {
        let account_id = helpers::get_session_id(&req);
        let mut request = AccountUpdate::new();
        request.set_id(account_id);
        request.set_email(body.email.to_owned());

        match route_message::<AccountUpdate, NetOk>(&req, &request) {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(err) => err.into(),
        }
    }

    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.resource("/profile", |r| {
            r.middleware(Authenticated);
            r.get().f(Self::get_profile);
            r.method(http::Method::PATCH).with(Profile::update_profile);
        }).resource("/profile/access-tokens", |r| {
                r.middleware(Authenticated);
                r.get().f(Self::get_access_tokens);
                r.post().f(Self::generate_access_token);
            })
            .resource("/profile/access-tokens/{id}", |r| {
                r.middleware(Authenticated);
                r.delete().f(Self::revoke_access_token);
            })
    }
}
