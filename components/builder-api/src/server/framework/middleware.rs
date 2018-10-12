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

use std::ops::Deref;

use bldr_core;
use bldr_core::metrics::CounterMetric;
use hab_net::conn::RouteClient;
use hab_net::{ErrCode, NetError};
use oauth_client::types::OAuth2User;
use protocol;
use protocol::originsrv::*;
use protocol::Routable;

use hab_net::privilege::FeatureFlags;
use server::error;
use server::models::account;
use server::resources::profile::do_get_access_tokens;
use server::services::metrics::Counter;
use server::services::route_broker::RouteBroker;
use server::AppState;

lazy_static! {
    static ref SESSION_DURATION: u32 = 1 * 24 * 60 * 60;
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
    Counter::RouteMessage.increment();

    if M::protocol() == protocol::Protocol::JobSrv {
        // Route via Protobuf over HTTP
        req.state()
            .jobsrv
            .rpc::<M, R>(msg)
            .map_err(error::Error::BuilderCore)
    } else {
        // Route via Protobuf over ZMQ
        req.extensions_mut()
            .get_mut::<RouteClient>()
            .expect("no XRouteClient extension in request")
            .route::<M, R>(msg)
            .map_err(error::Error::NetError)
    }
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

    let mut memcache = req.state().memcache.borrow_mut();
    match memcache.get_session(token) {
        Some(session) => {
            trace!("Session {} Cache Hit!", token);
            return Ok(session);
        }
        None => {
            trace!("Session {} Cache Miss!", token);
            if !bldr_core::access_token::is_access_token(token) {
                // No token in cache and not a PAT - bail
                return Err(error::Error::NetError(NetError::new(
                    ErrCode::BAD_TOKEN,
                    "net:auth:expired-token",
                )));
            }
            // Pull the session out of the current token provided so we can validate
            // it against the db's tokens
            let session = bldr_core::access_token::validate_access_token(
                &req.state().config.api.key_path,
                token,
            ).map_err(|_| NetError::new(ErrCode::BAD_TOKEN, "net:auth:bad-token"))?;

            if session.get_id() == bldr_core::access_token::BUILDER_ACCOUNT_ID {
                trace!("Builder token identified");
                memcache.set_session(token, &session, None);
                return Ok(session);
            }

            // If we can't find a token in the cache, we need to round-trip to the
            // db to see if we have a valid session token.
            match do_get_access_tokens(&req, session.get_id()) {
                Ok(access_tokens) => {
                    assert!(access_tokens.get_tokens().len() <= 1); // Can only have max of 1 for now
                    match access_tokens.get_tokens().first() {
                        Some(access_token) => {
                            let new_token = access_token.get_token();
                            if token.trim_right_matches('=') != new_token.trim_right_matches('=') {
                                // Token is valid but revoked or otherwise expired
                                return Err(error::Error::NetError(NetError::new(
                                    ErrCode::BAD_TOKEN,
                                    "net:auth:revoked-token",
                                )));
                            }
                            memcache.set_session(new_token, &session, None);
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
    }
}

pub fn session_create_oauth(
    req: &HttpRequest<AppState>,
    oauth_token: &str,
    user: &OAuth2User,
    provider: &str,
) -> error::Result<Session> {
    let mut session = Session::new();
    let mut session_token = SessionToken::new();
    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(e) => return Err(e),
    };

    let email = match user.email {
        Some(ref email) => {
            session.set_email(email.clone());
            email
        }
        None => "",
    };

    match account::Account::find_or_create(
        account::FindOrCreateAccount {
            name: user.username.to_string(),
            email: email.to_string(),
        },
        conn.deref(),
    ) {
        Ok(account) => {
            session_token.set_account_id(account.id as u64);
            session_token.set_extern_id(user.id.to_string());
            session_token.set_token(oauth_token.to_string().into_bytes());

            match provider.parse::<OAuthProvider>() {
                Ok(p) => session_token.set_provider(p),
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

            let encoded_token = encode_token(&session_token);
            session.set_id(account.id as u64);
            session.set_name(account.name);
            session.set_token(encoded_token.clone());
            session.set_flags(FeatureFlags::empty().bits());
            session.set_oauth_token(oauth_token.to_owned());

            debug!("issuing session, {:?}", session);
            req.state().memcache.borrow_mut().set_session(
                &session.get_token(),
                &session,
                Some(*SESSION_DURATION),
            );
            Ok(session)
        }
        Err(e) => {
            error!("Failed to create session {}", e);
            Err(e.into())
        }
    }
}

pub fn session_create_short_circuit(
    req: &HttpRequest<AppState>,
    token: &str,
) -> error::Result<Session> {
    let (user, provider) = match token.as_ref() {
        "bobo" => (
            OAuth2User {
                id: "0".to_string(),
                email: Some("bobo@example.com".to_string()),
                username: "bobo".to_string(),
            },
            "GitHub",
        ),
        "mystique" => (
            OAuth2User {
                id: "1".to_string(),
                email: Some("mystique@example.com".to_string()),
                username: "mystique".to_string(),
            },
            "GitHub",
        ),
        "hank" => (
            OAuth2User {
                id: "2".to_string(),
                email: Some("hank@example.com".to_string()),
                username: "hank".to_string(),
            },
            "GitHub",
        ),
        user => {
            error!("Unexpected short circuit token {:?}", user);
            return Err(error::Error::NetError(NetError::new(
                ErrCode::BUG,
                "net:session-short-circuit:unknown-token",
            )));
        }
    };

    session_create_oauth(req, token, &user, provider)
}

fn encode_token(token: &SessionToken) -> String {
    let bytes = protocol::message::encode(token).unwrap(); //Unwrap is safe
    base64::encode(&bytes)
}
