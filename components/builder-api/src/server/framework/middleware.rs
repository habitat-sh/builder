// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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
use std::path::PathBuf;

use actix_web::http;
use actix_web::middleware::{Middleware, Started};
use actix_web::{HttpRequest, Result};
use base64;
use protobuf;

use bldr_core;
use hab_net::conn::RouteClient;
use hab_net::{ErrCode, NetError, NetOk, NetResult};
use oauth_client::types::OAuth2User;
use protocol;
use protocol::sessionsrv::*;
use protocol::Routable;

use server;
use server::services::route_broker::RouteBroker;

/* TO DO: Add custom Cors middleware 

pub struct Cors;

impl AfterMiddleware for Cors {
    fn after(&self, _req: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(headers::AccessControlAllowOrigin::Any);
        res.headers.set(headers::AccessControlAllowHeaders(vec![
            UniCase("authorization".to_string()),
            UniCase("range".to_string()),
        ]));
        res.headers.set(headers::AccessControlAllowMethods(vec![
            Method::Put,
            Method::Delete,
            Method::Patch,
        ]));
        res.headers
            .set(headers::AccessControlExposeHeaders(vec![UniCase(
                "content-disposition".to_string(),
            )]));
        Ok(res)
    }
}

*/

// Router client
pub struct XRouteClient;

impl<S> Middleware<S> for XRouteClient {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let conn = RouteBroker::connect().unwrap();
        req.extensions_mut().insert::<RouteClient>(conn);
        Ok(Started::Done)
    }
}

pub fn route_message<M, R, S>(req: &HttpRequest<S>, msg: &M) -> NetResult<R>
where
    M: Routable,
    R: protobuf::Message,
{
    req.extensions_mut()
        .get_mut::<RouteClient>()
        .expect("no XRouteClient extension in request")
        .route::<M, R>(msg)
}

// Authentication
#[derive(Clone)]
pub struct Authenticated {
    pub key_path: PathBuf,
}

impl<S> Middleware<S> for Authenticated {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let token = req
            .headers()
            .get(http::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap(); // unwrap Ok
        let session = self
            .authenticate(req, &token)
            .map_err(|e| server::error::Error::NetError(e))?;
        req.extensions_mut().insert::<Session>(session);
        Ok(Started::Done)
    }
}

impl Authenticated {
    fn authenticate<S>(&self, req: &HttpRequest<S>, token: &str) -> NetResult<Session> {
        // Test hook - always create a valid session
        if env::var_os("HAB_FUNC_TEST").is_some() {
            debug!(
                "HAB_FUNC_TEST: {:?}; calling session_create_short_circuit",
                env::var_os("HAB_FUNC_TEST")
            );
            return session_create_short_circuit(req, token);
        };

        // Check for a valid personal access token
        if bldr_core::access_token::is_access_token(token) {
            let session = bldr_core::access_token::validate_access_token(&self.key_path, token)
                .map_err(|e| NetError::new(ErrCode::BAD_TOKEN, "net:auth:bad-token"))?;
            revocation_check(req, session.get_id(), token)
                .map_err(|e| NetError::new(ErrCode::BAD_TOKEN, "net:auth:revoked-token"))?;
            return Ok(session);
        };

        // Check for internal sessionsrv token
        let decoded_token = match base64::decode(token) {
            Ok(decoded_token) => decoded_token,
            Err(e) => {
                debug!("Failed to base64 decode token, err={:?}", e);
                return Err(NetError::new(ErrCode::BAD_TOKEN, "net:auth:decode:1"));
            }
        };

        match protocol::message::decode(&decoded_token) {
            Ok(session_token) => session_validate(req, session_token),
            Err(e) => {
                debug!("Failed to decode token, err={:?}", e);
                Err(NetError::new(ErrCode::BAD_TOKEN, "net:auth:decode:2"))
            }
        }
    }
}

pub fn revocation_check<S>(req: &HttpRequest<S>, account_id: u64, token: &str) -> NetResult<()> {
    let mut request = AccountTokenValidate::new();
    request.set_account_id(account_id);
    request.set_token(token.to_owned());
    route_message::<AccountTokenValidate, NetOk, S>(req, &request)?;
    Ok(())
}

pub fn session_validate<S>(req: &HttpRequest<S>, token: SessionToken) -> NetResult<Session> {
    let mut request = SessionGet::new();
    request.set_token(token);
    route_message::<SessionGet, Session, S>(req, &request)
}

pub fn session_create_oauth<S>(
    req: &HttpRequest<S>,
    token: &str,
    user: &OAuth2User,
    provider: &str,
) -> NetResult<Session> {
    let mut request = SessionCreate::new();
    request.set_session_type(SessionType::User);
    request.set_token(token.to_owned());
    request.set_extern_id(user.id.clone());
    request.set_name(user.username.clone());

    match provider.parse::<OAuthProvider>() {
        Ok(p) => request.set_provider(p),
        Err(e) => {
            warn!(
                "Error parsing oauth provider: provider={}, err={:?}",
                provider, e
            );
            return Err(NetError::new(ErrCode::BUG, "session_create_oauth:1"));
        }
    }

    if let Some(ref email) = user.email {
        request.set_email(email.clone());
    }

    route_message::<SessionCreate, Session, S>(req, &request)
}

pub fn session_create_short_circuit<S>(req: &HttpRequest<S>, token: &str) -> NetResult<Session> {
    let request = match token.as_ref() {
        "bobo" => {
            let mut request = SessionCreate::new();
            request.set_session_type(SessionType::User);
            request.set_extern_id("0".to_string());
            request.set_email("bobo@example.com".to_string());
            request.set_name("bobo".to_string());
            request.set_provider(OAuthProvider::GitHub);
            request
        }
        "mystique" => {
            let mut request = SessionCreate::new();
            request.set_session_type(SessionType::User);
            request.set_extern_id("1".to_string());
            request.set_email("mystique@example.com".to_string());
            request.set_name("mystique".to_string());
            request.set_provider(OAuthProvider::GitHub);
            request
        }
        "hank" => {
            let mut request = SessionCreate::new();
            request.set_extern_id("2".to_string());
            request.set_email("hank@example.com".to_string());
            request.set_name("hank".to_string());
            request.set_provider(OAuthProvider::GitHub);
            request
        }
        user => {
            error!("Unexpected short circuit token {:?}", user);
            return Err(NetError::new(
                ErrCode::BUG,
                "net:session-short-circuit:unknown-token",
            ));
        }
    };

    route_message::<SessionCreate, Session, S>(req, &request)
}
