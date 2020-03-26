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

use actix_web::{http::StatusCode,
                web::{self,
                      Data,
                      Path,
                      ServiceConfig},
                HttpResponse};

use oauth_client::error::Error as OAuthError;

use crate::{protocol::originsrv,
            server::{error::{Error,
                             Result},
                     framework::middleware::{session_create_oauth,
                                             session_create_short_circuit},
                     AppState}};

pub struct Authenticate {}

impl Authenticate {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/authenticate/{code}", web::get().to(authenticate));
    }
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
async fn authenticate(path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let code = path.into_inner();
    debug!("authenticate called, code = {}", code);

    match do_authenticate(&code, &state).await {
        Ok(session) => HttpResponse::Ok().json(session),
        Err(Error::OAuth(OAuthError::HttpResponse(_code, _response))) => {
            HttpResponse::new(StatusCode::UNAUTHORIZED)
        }
        Err(e) => {
            warn!("Oauth client error, {:?}", e);
            e.into()
        }
    }
}

// Internal - these functions should return Result<..>
//
async fn do_authenticate(code: &str, state: &AppState) -> Result<originsrv::Session> {
    if env::var_os("HAB_FUNC_TEST").is_some() {
        return session_create_short_circuit(code, state);
    }

    let oauth = &state.oauth;
    let (token, user) = oauth.authenticate(code).await?;

    session_create_oauth(&token, &user, &oauth.config.provider, state)
}
