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

use actix_web::{HttpRequest, HttpResponse};
use protocol::sessionsrv::*;

use server::error::Error;
use server::framework::middleware::route_message;
use server::AppState;

pub fn user_origins(req: &HttpRequest<AppState>) -> HttpResponse {
    debug!("user_origins called");
    let mut request = AccountOriginListRequest::new();

    let account_id = {
        let extensions = req.extensions();
        let session = extensions.get::<Session>().unwrap(); // Unwrap Ok
        session.get_id()
    };
    debug!("Got session, account id = {}", account_id);
    request.set_account_id(account_id);

    match route_message::<AccountOriginListRequest, AccountOriginListResponse>(req, &request) {
        Ok(origins) => HttpResponse::Ok().json(origins),
        Err(err) => Error::NetError(err).into(),
    }
}
