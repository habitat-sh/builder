use crate::{bldr_core::{access_token::{AccessToken,
                                       BUILDER_ACCOUNT_ID,
                                       BUILDER_ACCOUNT_NAME},
                        privilege::FeatureFlags},
            db::models::account::*,
            protocol::{self,
                       originsrv},
            server::{error,
                     AppState}};
use actix_web::{body::BoxBody,
                dev::{Service,
                      ServiceRequest,
                      ServiceResponse},
                http,
                web::Data,
                Error,
                HttpMessage,
                HttpResponse};
use futures::future::{ok,
                      Either,
                      Future};
use oauth_client::types::OAuth2User;
use std::env;

lazy_static! {
    static ref SESSION_DURATION: u32 = 3 * 24 * 60 * 60;
}

// Optional Authentication - this middleware does not enforce authentication,
// but will insert a Session if a valid Bearer token is received
pub fn authentication_middleware<S>(
    req: ServiceRequest,
    srv: &S)
    -> impl Future<Output = Result<ServiceResponse<BoxBody>, Error>>
    where S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>
{
    let hdr = match req.headers().get(http::header::AUTHORIZATION) {
        Some(hdr) => hdr.to_str().unwrap(), // unwrap Ok
        None => return Either::Left(srv.call(req)),
    };

    let hdr_components: Vec<&str> = hdr.split_whitespace().collect();
    if (hdr_components.len() != 2) || (hdr_components[0] != "Bearer") {
        return Either::Right(ok(req.into_response(HttpResponse::Unauthorized().finish())));
    }
    let token = hdr_components[1];

    let session = match authenticate(token,
                                     req.app_data::<Data<AppState>>().expect("request state"))
    {
        Ok(session) => session,
        Err(_) => {
            return Either::Right(ok(req.into_response(HttpResponse::Unauthorized().finish())))
        }
    };

    req.extensions_mut().insert::<originsrv::Session>(session);
    Either::Left(srv.call(req))
}

fn authenticate(token: &str, state: &AppState) -> error::Result<originsrv::Session> {
    // Test hook - always create a valid session
    if env::var_os("HAB_FUNC_TEST").is_some() {
        debug!("HAB_FUNC_TEST: {:?}; calling session_create_short_circuit",
               env::var_os("HAB_FUNC_TEST"));
        return session_create_short_circuit(token, state);
    };

    let mut memcache = state.memcache.borrow_mut();
    match memcache.get_session(token) {
        Some(session) => {
            trace!("Session {} Cache Hit!", token);
            Ok(session)
        }
        None => {
            trace!("Session {} Cache Miss!", token);

            // Pull the session out of the current token provided so we can validate
            // it against the db's tokens
            let mut session = AccessToken::validate_access_token(token, &state.config.api.key_path)
                .map_err(|e| {
                    trace!("Unable to validate access token {}, err={:?}", token, e);
                    error::Error::Authorization
                })?;

            trace!("Found valid session for {} tied to account {}", token, session.get_id());

            if session.get_id() == BUILDER_ACCOUNT_ID {
                trace!("Builder token identified");
                session.set_name(BUILDER_ACCOUNT_NAME.to_owned());
                memcache.set_session(token, &session, None);
                return Ok(session);
            }

            // If we can't find a token in the cache, we need to round-trip to the
            // db to see if we have a valid session token.
            let mut conn = state.db.get_conn().map_err(error::Error::DbError)?;

            match AccountToken::list(session.get_id(), &mut conn).map_err(error::Error::DieselError)
            {
                Ok(access_tokens) => {
                    if access_tokens.len() <= 1 {
                        trace!("Found {} tokens for user {}", access_tokens.len(), session.get_id());
                        return Err(error::Error::Authorization);
                    }
                    match access_tokens.first() {
                        Some(access_token) => {
                            let new_token = access_token.token.clone();
                            if token.trim_end_matches('=') != new_token.trim_end_matches('=') {
                                trace!("Different token {} found for user {}. Token is valid but revoked or otherwise expired", new_token, session.get_id());
                                return Err(error::Error::Authorization);
                            }

                            let account = match Account::get_by_id(session.get_id() as i64, &mut conn)
                                .map_err(error::Error::DieselError) {
                                Ok(account) => account,
                                Err(e) => {
                                    trace!("Failed to find account for id {}: {:?}", session.get_id(), e);
                                    return Err(error::Error::Authorization);
                                }
                            };
                            trace!("Found account for token {} in database", token);
                            session.set_name(account.name);
                            session.set_email(account.email);

                            memcache.set_session(&new_token, &session, None);
                            Ok(session)
                        }
                        None => {
                            // We have no tokens in the database for this user
                            trace!("Failed to find token {} in database", token);
                            Err(error::Error::Authorization)
                        }
                    }
                }
                Err(_) => {
                    // Failed to fetch tokens from the database for this user
                    trace!("Failed to find tokens for {} in database", token);
                    Err(error::Error::Authorization)
                }
            }
        }
    }
}

pub fn session_create_oauth(oauth_token: &str,
                            user: &OAuth2User,
                            provider: &str,
                            state: &AppState)
                            -> error::Result<originsrv::Session> {
    let mut session = originsrv::Session::new();
    let mut session_token = originsrv::SessionToken::new();
    let mut conn = state.db.get_conn().map_err(error::Error::DbError)?;

    let email = match user.email {
        Some(ref email) => {
            session.set_email(email.clone());
            email
        }
        None => "",
    };

    match Account::find_or_create(&NewAccount { name: &user.username,
                                                email },
                                  &mut conn)
    {
        Ok(account) => {
            session_token.set_account_id(account.id as u64);
            session_token.set_extern_id(user.id.to_string());
            session_token.set_token(oauth_token.to_string().into_bytes());

            match provider.parse::<originsrv::OAuthProvider>() {
                Ok(p) => session_token.set_provider(p),
                Err(e) => {
                    warn!("Error parsing oauth provider: provider={}, err={:?}",
                          provider, e);
                    return Err(error::Error::System);
                }
            }

            let encoded_token = encode_token(&session_token);
            session.set_id(account.id as u64);
            session.set_name(account.name);
            session.set_token(encoded_token);
            session.set_flags(FeatureFlags::empty().bits());
            session.set_oauth_token(oauth_token.to_owned());

            debug!("issuing session, {:?}", session);
            state.memcache
                 .borrow_mut()
                 .set_session(session.get_token(), &session, Some(*SESSION_DURATION));
            Ok(session)
        }
        Err(e) => {
            error!("Failed to create session {}", e);
            Err(e.into())
        }
    }
}

pub fn session_create_short_circuit(token: &str,
                                    state: &AppState)
                                    -> error::Result<originsrv::Session> {
    let (user, provider) = match token {
        "bobo" => {
            (OAuth2User { id:       "0".to_string(),
                          email:    Some("bobo@example.com".to_string()),
                          username: "bobo".to_string(), },
             "GitHub")
        }
        "mystique" => {
            (OAuth2User { id:       "1".to_string(),
                          email:    Some("mystique@example.com".to_string()),
                          username: "mystique".to_string(), },
             "GitHub")
        }
        "hank" => {
            (OAuth2User { id:       "2".to_string(),
                          email:    Some("hank@example.com".to_string()),
                          username: "hank".to_string(), },
             "GitHub")
        }
        "wesker" => {
            (OAuth2User { id:       "3".to_string(),
                          email:    Some("awesker@umbrella.corp".to_string()),
                          username: "wesker".to_string(), },
             "GitHub")
        }
        "lkennedy" => {
            (OAuth2User { id:       "4".to_string(),
                          email:    Some("lkennedy@rcpd.gov".to_string()),
                          username: "lkennedy".to_string(), },
             "GitHub")
        }
        user => {
            error!("Unexpected short circuit token {:?}", user);
            return Err(error::Error::System);
        }
    };

    session_create_oauth(token, &user, provider, state)
}

fn encode_token(token: &originsrv::SessionToken) -> String {
    let bytes = protocol::message::encode(token).unwrap(); // Unwrap is safe
    habitat_core::base64::encode(bytes)
}
