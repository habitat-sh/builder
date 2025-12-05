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
    if let Some(test_env) = handle_test_environment(token, state)? {
        return Ok(test_env);
    }

    // Try to get session from cache first
    if let Some(cached_session) = get_cached_session(token, state) {
        return Ok(cached_session);
    }

    // Validate token and create new session
    validate_token_and_create_session(token, state)
}

fn handle_test_environment(token: &str,
                           state: &AppState)
                           -> error::Result<Option<originsrv::Session>> {
    if env::var_os("HAB_FUNC_TEST").is_some() {
        debug!("HAB_FUNC_TEST: {:?}; calling session_create_short_circuit",
               env::var_os("HAB_FUNC_TEST"));
        return Ok(Some(session_create_short_circuit(token, state)?));
    }
    Ok(None)
}

fn get_cached_session(token: &str, state: &AppState) -> Option<originsrv::Session> {
    let mut memcache = state.memcache.borrow_mut();
    match memcache.get_session(token) {
        Some(session) => {
            trace!("Session {} Cache Hit!", token);
            Some(session)
        }
        None => {
            trace!("Session {} Cache Miss!", token);
            None
        }
    }
}

fn validate_token_and_create_session(token: &str,
                                     state: &AppState)
                                     -> error::Result<originsrv::Session> {
    // Pull the session out of the current token provided so we can validate it against the db's
    // tokens
    let mut session =
        AccessToken::validate_access_token(token, &state.config.api.key_path).map_err(|e| {
            trace!("Unable to validate access token {}, err={:?}", token, e);
            error::Error::Authorization
        })?;

    trace!("Found valid session for {} tied to account {}",
           token,
           session.get_id());

    // Handle special builder account case
    if let Some(builder_session) = handle_builder_account(&mut session, token, state) {
        return Ok(builder_session);
    }

    // Validate against database tokens
    validate_database_token(token, &mut session, state)
}

fn handle_builder_account(session: &mut originsrv::Session,
                          token: &str,
                          state: &AppState)
                          -> Option<originsrv::Session> {
    if session.get_id() == BUILDER_ACCOUNT_ID {
        trace!("Builder token identified");
        session.set_name(BUILDER_ACCOUNT_NAME.to_owned());
        state.memcache
             .borrow_mut()
             .set_session(token, session, None);
        return Some(session.clone());
    }
    None
}

fn validate_database_token(token: &str,
                           session: &mut originsrv::Session,
                           state: &AppState)
                           -> error::Result<originsrv::Session> {
    // If we can't find a token in the cache, we need to round-trip to the db to see if we have a
    // valid session token.
    let mut conn = state.db.get_conn().map_err(error::Error::DbError)?;

    let access_tokens =
        match AccountToken::list(session.get_id(), &mut conn).map_err(error::Error::DieselError) {
            Ok(tokens) => tokens,
            Err(e) => {
                trace!("Failed to list access tokens for user {}: {:?}",
                       session.get_id(),
                       e);
                return Err(error::Error::Authorization);
            }
        };

    validate_token_count_and_match(token, session, &access_tokens, &mut conn, state)
}

fn validate_token_count_and_match(token: &str,
                                  session: &mut originsrv::Session,
                                  access_tokens: &[AccountToken],
                                  conn: &mut diesel::PgConnection,
                                  state: &AppState)
                                  -> error::Result<originsrv::Session> {
    if access_tokens.len() > 1 {
        trace!("Found {} tokens for user {}",
               access_tokens.len(),
               session.get_id());
        return Err(error::Error::Authorization);
    }

    let access_token = access_tokens.first().ok_or_else(|| {
                                                 trace!("Failed to find token {} in database",
                                                        token);
                                                 error::Error::Authorization
                                             })?;

    let new_token = &access_token.token;
    if token.trim_end_matches('=') != new_token.trim_end_matches('=') {
        trace!("Different token {} found for user {}. Token is valid but revoked or otherwise \
                expired",
               new_token,
               session.get_id());
        return Err(error::Error::Authorization);
    }

    finalize_session_with_account(token, session, new_token, conn, state)
}

fn finalize_session_with_account(_token: &str,
                                 session: &mut originsrv::Session,
                                 new_token: &str,
                                 conn: &mut diesel::PgConnection,
                                 state: &AppState)
                                 -> error::Result<originsrv::Session> {
    let account = Account::get_by_id(session.get_id() as i64, conn).map_err(|e| {
                                                                       trace!("Failed to find \
                                                                               account for id \
                                                                               {}: {:?}",
                                                                              session.get_id(),
                                                                              e);
                                                                       error::Error::Authorization
                                                                   })?;

    trace!("Found account for token {} in database", new_token);
    session.set_name(account.name);
    session.set_email(account.email);

    state.memcache
         .borrow_mut()
         .set_session(new_token, session, None);
    Ok(session.clone())
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
