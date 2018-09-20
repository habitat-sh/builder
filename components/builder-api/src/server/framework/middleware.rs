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
use actix_web::middleware::{Middleware, Started};
use actix_web::{HttpRequest, HttpResponse, Result};
use base64;
use protobuf;

use bldr_core;
use bldr_core::metrics::CounterMetric;
use hab_net::conn::RouteClient;
use hab_net::{ErrCode, NetError};
use oauth_client::types::OAuth2User;
use protocol;
use protocol::originsrv::*;
use protocol::Routable;

use server::error;
use server::resources::profile::do_get_access_tokens;
use server::services::metrics::Counter;
use server::services::route_broker::RouteBroker;
use server::AppState;

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
    Counter::RouteMessage.increment();

    req.extensions_mut()
        .get_mut::<RouteClient>()
        .expect("no XRouteClient extension in request")
        .route::<M, R>(msg)
        .map_err(|e| error::Error::NetError(e))
}

// Optional Authentication - this middleware does not enforce authentication,
// but will insert a Session if a valid Bearer token is received
pub struct Authentication;

impl Middleware<AppState> for Authentication {
    fn start(&self, req: &HttpRequest<AppState>) -> Result<Started> {
        let hdr = match req.headers().get(http::header::AUTHORIZATION) {
            Some(hdr) => hdr.to_str().unwrap(), // unwrap Ok
            None => return Ok(Started::Done),
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
        let mut memcache = req.state().memcache.borrow_mut();
        match memcache.get_session(token) {
            Some(session) => {
                trace!("Session {} Cache Hit!", token);
                return Ok(session);
            }
            None => {
                trace!("Session {} Cache Miss!", token);
                // Pull the session out of the current token provided so we can validate
                // it against the db's tokens
                let session = bldr_core::access_token::validate_access_token(
                    &req.state().config.api.key_path,
                    token,
                ).map_err(|_| NetError::new(ErrCode::BAD_TOKEN, "net:auth:bad-token"))?;

                // If we can't find a token in the cache, we need to round-trip to the
                // db to see if we have a valid session token.
                match do_get_access_tokens(&req, session.get_id()) {
                    Ok(access_tokens) => {
                        assert!(access_tokens.get_tokens().len() <= 1); // Can only have max of 1 for now
                        match access_tokens.get_tokens().first() {
                            Some(access_token) => {
                                let new_token = access_token.get_token();
                                if token.trim_right_matches('=')
                                    != new_token.trim_right_matches('=')
                                {
                                    // Token is valid but revoked or otherwise expired
                                    return Err(error::Error::NetError(NetError::new(
                                        ErrCode::BAD_TOKEN,
                                        "net:auth:revoked-token",
                                    )));
                                }
                                memcache.set_session(new_token, &session);
                                return Ok(session);
                            }
                            None => {
                                // We have no tokens in the database for this user
                                return Err(error::Error::NetError(NetError::new(
                                    ErrCode::BAD_TOKEN,
                                    "net:auth:revoked-token",
                                )));
                            }
                        }
                    }
                    Err(_) => {
                        // Failed to fetch tokens from the database for this user
                        return Err(error::Error::NetError(NetError::new(
                            ErrCode::BAD_TOKEN,
                            "net:auth:revoked-token",
                        )));
                    }
                }
            }
        };
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
