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

use actix_web::http::Method;
use actix_web::{App, HttpRequest, HttpResponse};
use protocol::sessionsrv::*;

use server::authorize::authorize_session;
use server::framework::middleware::route_message;
use server::AppState;

pub struct User {}

impl User {
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route("/user/invitations", Method::GET, get_invitations)
            .route("/user/origins", Method::GET, get_origins)
    }
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_invitations(req: HttpRequest<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let mut request = AccountInvitationListRequest::new();
    request.set_account_id(account_id);

    match route_message::<AccountInvitationListRequest, AccountInvitationListResponse>(
        &req, &request,
    ) {
        Ok(invites) => HttpResponse::Ok().json(invites),
        Err(err) => err.into(),
    }
}

fn get_origins(req: HttpRequest<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let mut request = AccountOriginListRequest::new();
    request.set_account_id(account_id);

    match route_message::<AccountOriginListRequest, AccountOriginListResponse>(&req, &request) {
        Ok(origins) => HttpResponse::Ok().json(origins),
        Err(err) => err.into(),
    }
}
