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

use std::env;

use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Path};

use hab_net::{ErrCode, NetError};
use oauth_client::error::Error as OAuthError;

use protocol::sessionsrv::*;

use server::error::{Error, Result};
use server::framework::middleware::{session_create_oauth, session_create_short_circuit};
use server::AppState;

pub struct Authenticate {}

impl Authenticate {
    //
    // Internal - these functions should return Result<..>
    //
    fn do_authenticate(req: &HttpRequest<AppState>, code: String) -> Result<Session> {
        if env::var_os("HAB_FUNC_TEST").is_some() {
            return session_create_short_circuit(req, &code);
        }

        let oauth = &req.state().oauth;
        let (token, user) = oauth.authenticate(&code)?;
        let session = session_create_oauth(req, &token, &user, &oauth.config.provider)?;

        let id_str = session.get_id().to_string();
        if let Err(e) = req.state().segment.identify(&id_str) {
            warn!("Error identifying a user in segment, {}", e);
        }

        Ok(session)
    }

    //
    // Route handlers - these functions should return HttpResponse
    //
    fn authenticate(req: &HttpRequest<AppState>) -> HttpResponse {
        let code = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok
        debug!("authenticate called, code = {}", code);

        match Self::do_authenticate(req, code) {
            Ok(session) => HttpResponse::Ok().json(session),
            Err(Error::OAuth(OAuthError::HttpResponse(code, response))) => {
                let msg = format!("{}-{}", code, response);
                Error::NetError(NetError::new(ErrCode::ACCESS_DENIED, msg)).into()
            }
            Err(e) => {
                warn!("Oauth client error, {:?}", e);
                Error::NetError(NetError::new(ErrCode::BAD_REMOTE_REPLY, "rg:auth:1")).into()
            }
        }
    }

    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.resource("/authenticate/{code}", |r| r.get().f(Self::authenticate))
    }
}
