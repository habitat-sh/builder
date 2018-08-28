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

use actix_web::http;
use actix_web::http::header;
use actix_web::middleware::{Middleware, Response, Started};
use actix_web::{HttpRequest, HttpResponse, Result};
use base64;
use protobuf;

use bldr_core;
use hab_net::conn::RouteClient;
use hab_net::{ErrCode, NetError, NetOk};
use oauth_client::types::OAuth2User;
use protocol;
use protocol::sessionsrv::*;
use protocol::Routable;

use server::error;
use server::services::route_broker::RouteBroker;
use server::AppState;

// Cors
pub struct Cors;

impl<S> Middleware<S> for Cors {
    fn response(&self, _: &HttpRequest<S>, mut resp: HttpResponse) -> Result<Response> {
        {
            let h = resp.headers_mut();
            h.insert(
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                header::HeaderValue::from_static("*"),
            );
            h.insert(
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                header::HeaderValue::from_static("Authorization"),
            );
            h.insert(
                header::ACCESS_CONTROL_ALLOW_METHODS,
                header::HeaderValue::from_static("DELETE, PATCH, POST, PUT"),
            );
            h.insert(
                header::ACCESS_CONTROL_EXPOSE_HEADERS,
                header::HeaderValue::from_static("Content-Disposition"),
            );
        }
        Ok(Response::Done(resp))
    }
}

// Router client
pub struct XRouteClient;

impl<S> Middleware<S> for XRouteClient {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let conn = RouteBroker::connect().unwrap();
        req.extensions_mut().insert::<RouteClient>(conn);
        Ok(Started::Done)
    }
}

pub fn route_message<M, R>(req: &HttpRequest<AppState>, msg: &M) -> error::Result<R>
where
    M: Routable,
    R: protobuf::Message,
{
    req.extensions_mut()
        .get_mut::<RouteClient>()
        .expect("no XRouteClient extension in request")
        .route::<M, R>(msg)
        .map_err(|e| error::Error::NetError(e))
}

/*
// Authentication
pub struct Authenticated;

impl Middleware<AppState> for Authenticated {
    fn start(&self, req: &HttpRequest<AppState>) -> Result<Started> {
        auth_wrapper(req, false)
    }
}
*/

// Optional Authentication
pub struct Optional;

impl Middleware<AppState> for Optional {
    fn start(&self, req: &HttpRequest<AppState>) -> Result<Started> {
        auth_wrapper(req, true)
    }
}

fn auth_wrapper(req: &HttpRequest<AppState>, optional: bool) -> Result<Started> {
    let hdr = match req.headers().get(http::header::AUTHORIZATION) {
        Some(hdr) => hdr.to_str().unwrap(), // unwrap Ok
        None => if optional {
            return Ok(Started::Done);
        } else {
            return Ok(Started::Response(HttpResponse::Unauthorized().finish()));
        },
    };

    let hdr_components: Vec<&str> = hdr.split_whitespace().collect();
    if (hdr_components.len() != 2) || (hdr_components[0] != "Bearer") {
        return Ok(Started::Response(HttpResponse::Unauthorized().finish()));
    }
    let token = hdr_components[1];

    let session = match authenticate(req, &token) {
        Ok(session) => session,
        Err(_) => return Ok(Started::Response(HttpResponse::Unauthorized().finish())),
    };

    req.extensions_mut().insert::<Session>(session);
    Ok(Started::Done)
}

fn authenticate(req: &HttpRequest<AppState>, token: &str) -> error::Result<Session> {
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
        let session =
            bldr_core::access_token::validate_access_token(&req.state().config.api.key_path, token)
                .map_err(|_| NetError::new(ErrCode::BAD_TOKEN, "net:auth:bad-token"))?;
        revocation_check(req, session.get_id(), token)
            .map_err(|_| NetError::new(ErrCode::BAD_TOKEN, "net:auth:revoked-token"))?;
        return Ok(session);
    };

    // Check for internal sessionsrv token
    let decoded_token = match base64::decode(token) {
        Ok(decoded_token) => decoded_token,
        Err(e) => {
            debug!("Failed to base64 decode token, err={:?}", e);
            return Err(error::Error::NetError(NetError::new(
                ErrCode::BAD_TOKEN,
                "net:auth:decode:1",
            )));
        }
    };

    match protocol::message::decode(&decoded_token) {
        Ok(session_token) => session_validate(req, session_token),
        Err(e) => {
            debug!("Failed to decode token, err={:?}", e);
            Err(error::Error::NetError(NetError::new(
                ErrCode::BAD_TOKEN,
                "net:auth:decode:2",
            )))
        }
    }
}

fn revocation_check(
    req: &HttpRequest<AppState>,
    account_id: u64,
    token: &str,
) -> error::Result<()> {
    let mut request = AccountTokenValidate::new();
    request.set_account_id(account_id);
    request.set_token(token.to_owned());
    route_message::<AccountTokenValidate, NetOk>(req, &request)?;
    Ok(())
}

fn session_validate(req: &HttpRequest<AppState>, token: SessionToken) -> error::Result<Session> {
    let mut request = SessionGet::new();
    request.set_token(token);
    route_message::<SessionGet, Session>(req, &request)
}

pub fn session_create_oauth(
    req: &HttpRequest<AppState>,
    token: &str,
    user: &OAuth2User,
    provider: &str,
) -> error::Result<Session> {
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
            return Err(error::Error::NetError(NetError::new(
                ErrCode::BUG,
                "session_create_oauth:1",
            )));
        }
    }

    if let Some(ref email) = user.email {
        request.set_email(email.clone());
    }

    route_message::<SessionCreate, Session>(req, &request)
}

pub fn session_create_short_circuit(
    req: &HttpRequest<AppState>,
    token: &str,
) -> error::Result<Session> {
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
            return Err(error::Error::NetError(NetError::new(
                ErrCode::BUG,
                "net:session-short-circuit:unknown-token",
            )));
        }
    };

    route_message::<SessionCreate, Session>(req, &request)
}
