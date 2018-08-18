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

use actix_web::{App, HttpRequest, HttpResponse};
use protocol::sessionsrv::*;

use server::error::Result;
use server::framework::middleware::{route_message, Authenticated};
use server::helpers;
use server::AppState;

pub struct User {}

impl User {
    //
    // Internal - these functions should return Result<..>
    //
    fn do_get_invitations(req: &HttpRequest<AppState>) -> Result<AccountInvitationListResponse> {
        let account_id = helpers::get_session_id(req);

        let mut request = AccountInvitationListRequest::new();
        request.set_account_id(account_id);

        route_message::<AccountInvitationListRequest, AccountInvitationListResponse>(req, &request)
    }

    fn do_get_origins(req: &HttpRequest<AppState>) -> Result<AccountOriginListResponse> {
        let account_id = helpers::get_session_id(req);

        let mut request = AccountOriginListRequest::new();
        request.set_account_id(account_id);

        route_message::<AccountOriginListRequest, AccountOriginListResponse>(req, &request)
    }

    //
    // Route handlers - these functions should return HttpResponse
    //
    fn get_invitations(req: &HttpRequest<AppState>) -> HttpResponse {
        match Self::do_get_invitations(req) {
            Ok(invites) => HttpResponse::Ok().json(invites),
            Err(err) => err.into(),
        }
    }

    fn get_origins(req: &HttpRequest<AppState>) -> HttpResponse {
        match Self::do_get_origins(req) {
            Ok(origins) => HttpResponse::Ok().json(origins),
            Err(err) => err.into(),
        }
    }

    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.resource("/user/invitations", |r| {
            r.middleware(Authenticated);
            r.get().f(Self::get_invitations);
        }).resource("/user/origins", |r| {
            r.middleware(Authenticated);
            r.get().f(Self::get_origins);
        })
    }
}
