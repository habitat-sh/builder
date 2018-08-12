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
use actix_web::{HttpRequest, HttpResponse, Path};

use hab_net::{ErrCode, NetError};
use oauth_client::error::Error as OAuthError;
use server::framework::middleware::{session_create_oauth, session_create_short_circuit};

use server::error::Error;
use server::AppState;

pub fn authenticate(req: &HttpRequest<AppState>) -> HttpResponse {
    let code = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok
    debug!("authenticate called with: code={}", code);

    if env::var_os("HAB_FUNC_TEST").is_some() {
        return match session_create_short_circuit(req, &code) {
            Ok(session) => HttpResponse::Ok().json(session),
            Err(err) => Error::NetError(err).into(),
        };
    }

    let oauth = &req.state().oauth;

    match oauth.authenticate(&code) {
        Ok((token, user)) => {
            let session = match session_create_oauth(req, &token, &user, &oauth.config.provider) {
                Ok(session) => session,
                Err(err) => return Error::NetError(err).into(),
            };

            let id_str = session.get_id().to_string();
            if let Err(e) = req.state().segment.identify(&id_str) {
                warn!("Error identifying a user in segment, {}", e);
            }

            HttpResponse::Ok().json(session)
        }
        Err(OAuthError::HttpResponse(code, response)) => {
            let msg = format!("{}-{}", code, response);
            Error::NetError(NetError::new(ErrCode::ACCESS_DENIED, msg)).into()
        }
        Err(e) => {
            warn!("Oauth client error, {:?}", e);
            Error::NetError(NetError::new(ErrCode::BAD_REMOTE_REPLY, "rg:auth:1")).into()
        }
    }
}

/* OLD:

pub fn authenticate(req: &mut Request) -> IronResult<Response> {
    let code = match get_param(req, "code") {
        Some(c) => c,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if env::var_os("HAB_FUNC_TEST").is_some() {
        let session = { session_create_short_circuit(req, &code)? };
        return Ok(render_json(status::Ok, &session));
    }

    let oauth = req.get::<persistent::Read<OAuthCli>>().unwrap();
    let segment = req.get::<persistent::Read<SegmentCli>>().unwrap();

    match oauth.authenticate(&code) {
        Ok((token, user)) => {
            let session = session_create_oauth(req, &token, &user, &oauth.config.provider)?;
            let id_str = session.get_id().to_string();
            if let Err(e) = segment.identify(&id_str) {
                warn!("Error identifying a user in segment, {}", e);
            }

            Ok(render_json(status::Ok, &session))
        }
        Err(OAuthError::HttpResponse(code, response)) => {
            let msg = format!("{}-{}", code, response);
            let err = NetError::new(ErrCode::ACCESS_DENIED, msg);
            Ok(render_net_error(&err))
        }
        Err(e) => {
            warn!("Oauth client error, {:?}", e);
            let err = NetError::new(ErrCode::BAD_REMOTE_REPLY, "rg:auth:1");
            Ok(render_net_error(&err))
        }
    }
}
*/
