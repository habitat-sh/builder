use crate::{bldr_core::{access_token::{AccessToken,
                                       BUILDER_ACCOUNT_ID,
                                       BUILDER_ACCOUNT_NAME},
                        privilege::FeatureFlags},
            config::ProvisionCfg,
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
use std::{convert::TryFrom,
          env};

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
    if let Some(cached_session) = get_cached_session(token, state)? {
        return Ok(cached_session);
    }

    // Validate token and create new session
    validate_token_and_create_session(token, state)
}

fn handle_test_environment(token: &str,
                           state: &AppState)
                           -> error::Result<Option<originsrv::Session>> {
    if env::var_os("HAB_FUNC_TEST").is_some()
       && matches!(token,
                   "bobo"
                   | "mystique"
                   | "hank"
                   | "wesker"
                   | "lkennedy"
                   | "oauth-identity-original"
                   | "oauth-identity-renamed"
                   | "oauth-identity-reused"
                   | "oauth-identity-legacy-first-login"
                   | "oauth-identity-gitlab"
                   | "oauth-identity-okta"
                   | "oauth-identity-collision-owner"
                   | "oauth-identity-collision-holder"
                   | "oauth-identity-collision-rename"
                   | "oauth-identity-collision-legacy"
                   | "oauth-identity-backfilled"
                   | "oauth-identity-inviter"
                   | "oauth-identity-service")
    {
        debug!("HAB_FUNC_TEST: {:?}; calling session_create_short_circuit",
               env::var_os("HAB_FUNC_TEST"));
        return Ok(Some(session_create_short_circuit(token, state)?));
    }
    Ok(None)
}

fn get_cached_session(token: &str, state: &AppState) -> error::Result<Option<originsrv::Session>> {
    // Release the cache borrow before any further cache or database access.
    let cached = state.memcache.borrow_mut().get_session(token);
    match cached {
        Some(mut session) => {
            trace!("Session {} Cache Hit!", token);
            if session.id() != BUILDER_ACCOUNT_ID {
                refresh_cached_session_account(&mut session, state)?;
            }
            Ok(Some(session))
        }
        None => {
            trace!("Session {} Cache Miss!", token);
            Ok(None)
        }
    }
}

/// Apply the account's current name and email to a cached session. They come
/// from the account cache, which login refreshes whenever a name changes. The
/// database is consulted only when that entry has expired or been evicted.
fn refresh_cached_session_account(session: &mut originsrv::Session,
                                  state: &AppState)
                                  -> error::Result<()> {
    let cached = state.memcache.borrow_mut().get_account(session.id());
    if let Some((name, email)) = cached {
        session.set_name(name);
        session.set_email(email);
        return Ok(());
    }

    let account_id = i64::try_from(session.id()).map_err(|_| error::Error::Authorization)?;
    // Keep authenticating from the cached session while the database is unavailable.
    let mut conn = match state.db.get_conn() {
        Ok(conn) => conn,
        Err(e) => {
            warn!("Unable to refresh cached account {}: {}", account_id, e);
            return Ok(());
        }
    };
    let account = match Account::get_by_id(account_id, &mut conn) {
        Ok(account) => account,
        Err(diesel::result::Error::NotFound) => return Err(error::Error::Authorization),
        Err(e) => {
            warn!("Unable to refresh cached account {}: {}", account_id, e);
            return Ok(());
        }
    };
    state.memcache
         .borrow_mut()
         .set_account(session.id(), &account.name, &account.email);
    session.set_name(account.name);
    session.set_email(account.email);
    Ok(())
}

pub(crate) fn validate_token_and_create_session(token: &str,
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
           session.id());

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
    if session.id() == BUILDER_ACCOUNT_ID {
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
        match AccountToken::list(session.id(), &mut conn).map_err(error::Error::DieselError) {
            Ok(tokens) => tokens,
            Err(e) => {
                trace!("Failed to list access tokens for user {}: {:?}",
                       session.id(),
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
        error!("Found {} tokens for user {} but there should only be one",
               access_tokens.len(),
               session.id());
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
               session.id());
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
    let account = Account::get_by_id(session.id() as i64, conn).map_err(|e| {
                                                                   trace!("Failed to find \
                                                                           account for id {}: \
                                                                           {:?}",
                                                                          session.id(),
                                                                          e);
                                                                   error::Error::Authorization
                                                               })?;

    trace!("Found account for token {} in database", new_token);
    session.set_name(account.name.clone());
    session.set_email(account.email.clone());

    {
        let mut memcache = state.memcache.borrow_mut();
        memcache.set_account(session.id(), &account.name, &account.email);
        memcache.set_session(new_token, session, None);
    }
    Ok(session.clone())
}

/// The auto-provisioned account is a service account for startup API calls. It
/// is never bound to, or renamed for, an OAuth identity.
fn is_provisioned_service_account(config: &ProvisionCfg, username: &str) -> bool {
    config.auto_provision_account && config.username == username
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

    let parsed_provider = provider.parse::<originsrv::OAuthProvider>()
                                  .map_err(|_| error::Error::BadRequest)?;

    if is_provisioned_service_account(&state.config.provision, &user.username) {
        warn!("Rejected OAuth login using the provisioned service account name");
        return Err(error::Error::Conflict);
    }

    match Account::find_or_create_oauth(&NewAccount { name: &user.username,
                                                      email },
                                        provider,
                                        &user.id,
                                        &mut conn)
    {
        Ok((account, displaced)) => {
            {
                // Share the new names with every cached session of these accounts.
                let mut memcache = state.memcache.borrow_mut();
                memcache.set_account(account.id as u64, &account.name, &account.email);
                if let Some(displaced) = displaced {
                    memcache.set_account(displaced.id as u64, &displaced.name, &displaced.email);
                }
            }
            session_token.set_account_id(account.id as u64);
            session_token.set_extern_id(user.id.to_string());
            session_token.set_token(oauth_token.to_string().into_bytes());

            session_token.set_provider(parsed_provider);

            let encoded_token = encode_token(&session_token);
            session.set_id(account.id as u64);
            session.set_name(account.name);
            session.set_token(encoded_token);
            session.set_flags(FeatureFlags::empty().bits());
            session.set_oauth_token(oauth_token.to_owned());

            debug!("issuing session, {:?}", session);
            state.memcache
                 .borrow_mut()
                 .set_session(session.token(), &session, Some(*SESSION_DURATION));
            Ok(session)
        }
        Err(e) => {
            error!("Failed to create session {}", e);
            if matches!(e, diesel::result::Error::NotFound) {
                Err(error::Error::Conflict)
            } else {
                Err(e.into())
            }
        }
    }
}

pub fn session_create_short_circuit(token: &str,
                                    state: &AppState)
                                    -> error::Result<originsrv::Session> {
    let (mut user, provider) = match token {
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
        "oauth-identity-original" => {
            (OAuth2User { id:       "1000001".to_string(),
                          email:    None,
                          username: "oauth-identity-original".to_string(), },
             "github")
        }
        "oauth-identity-renamed" => {
            (OAuth2User { id:       "1000001".to_string(),
                          email:    None,
                          username: "oauth-identity-renamed".to_string(), },
             "github")
        }
        "oauth-identity-reused" => {
            (OAuth2User { id:       "1000002".to_string(),
                          email:    None,
                          username: "oauth-identity-original".to_string(), },
             "github")
        }
        "oauth-identity-legacy-first-login" => {
            (OAuth2User { id:       "1000005".to_string(),
                          email:    None,
                          username: "oauth-identity-legacy".to_string(), },
             "github")
        }
        "oauth-identity-backfilled" => {
            (OAuth2User { id:       "1000006".to_string(),
                          email:    None,
                          username: "oauth-identity-backfilled-renamed".to_string(), },
             "github")
        }
        "oauth-identity-inviter" => {
            (OAuth2User { id:       "1000007".to_string(),
                          email:    None,
                          username: "oauth-identity-inviter".to_string(), },
             "github")
        }
        "oauth-identity-service" => {
            (OAuth2User { id:       "1000008".to_string(),
                          email:    None,
                          username: "oauth-identity-service".to_string(), },
             "github")
        }
        "oauth-identity-gitlab" => {
            (OAuth2User { id:       "1000001".to_string(),
                          email:    None,
                          username: "oauth-identity-gitlab".to_string(), },
             "gitlab")
        }
        "oauth-identity-okta" => {
            (OAuth2User { id:       "1000001".to_string(),
                          email:    None,
                          username: "oauth-identity-okta".to_string(), },
             "okta")
        }
        "oauth-identity-collision-owner" => {
            (OAuth2User { id:       "1000003".to_string(),
                          email:    None,
                          username: "oauth-identity-collision-old".to_string(), },
             "github")
        }
        "oauth-identity-collision-holder" => {
            (OAuth2User { id:       "1000004".to_string(),
                          email:    None,
                          username: "oauth-identity-collision-new".to_string(), },
             "github")
        }
        "oauth-identity-collision-rename" => {
            (OAuth2User { id:       "1000003".to_string(),
                          email:    None,
                          username: "oauth-identity-collision-new".to_string(), },
             "github")
        }
        "oauth-identity-collision-legacy" => {
            (OAuth2User { id:       "1000003".to_string(),
                          email:    None,
                          username: "oauth-identity-collision-legacy".to_string(), },
             "github")
        }
        user => {
            error!("Unexpected short circuit token {:?}", user);
            return Err(error::Error::System);
        }
    };

    let mut provider = provider.to_owned();
    if matches!(token, "bobo" | "mystique" | "hank" | "wesker" | "lkennedy") {
        let mut conn = state.db.get_conn().map_err(error::Error::DbError)?;
        resolve_test_fixture_identity(&mut user, &mut provider, &mut conn)?;
    }

    session_create_oauth(token, &user, &provider, state)
}

fn resolve_test_fixture_identity(user: &mut OAuth2User,
                                 provider: &mut String,
                                 conn: &mut diesel::PgConnection)
                                 -> error::Result<()> {
    match Account::get(&user.username, conn) {
        Ok(account) => {
            if let (Some(stored_provider), Some(stored_id)) = (account.provider, account.extern_id)
            {
                *provider = stored_provider;
                user.id = stored_id;
            }
            Ok(())
        }
        Err(diesel::result::Error::NotFound) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn encode_token(token: &originsrv::SessionToken) -> String {
    let bytes = protocol::message::encode(token).unwrap(); // Unwrap is safe
    habitat_core::base64::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provisioned_service_account_name_is_reserved_only_when_provisioning() {
        let mut config = ProvisionCfg { username: "service".to_string(),
                                        ..ProvisionCfg::default() };
        assert!(!is_provisioned_service_account(&config, "service"));

        config.auto_provision_account = true;
        assert!(is_provisioned_service_account(&config, "service"));
        assert!(!is_provisioned_service_account(&config, "someone-else"));
    }
}
