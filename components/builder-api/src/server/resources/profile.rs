use crate::{bldr_core,
            db::models::{account::*,
                         license_keys::*},
            protocol::originsrv,
            server::{authorize::authorize_session,
                     error::{Error,
                             Result},
                     framework::headers,
                     helpers::req_state,
                     AppState}};
use actix_web::{body::BoxBody,
                http::{self,
                       StatusCode},
                web::{self,
                      Data,
                      Json,
                      Path,
                      ServiceConfig},
                HttpMessage,
                HttpRequest,
                HttpResponse};
use bldr_core::access_token::AccessToken as CoreAccessToken;
use bytes::Bytes;
use chrono::NaiveDate;
use reqwest;
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserUpdateReq {
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct LicensePayload {
    pub account_id:  String,
    pub license_key: String,
}

pub struct Profile {}

impl Profile {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/profile", web::get().to(get_account))
           .route("/profile", web::patch().to(update_account))
           .route("/profile/access-tokens", web::get().to(get_access_tokens))
           .route("/profile/access-tokens",
                  web::post().to(generate_access_token))
           .route("/profile/access-tokens/{id}",
                  web::delete().to(revoke_access_token))
           .route("/profile/license", web::put().to(set_license))
           .route("/profile/license", web::delete().to(delete_license))
           .route("/profile/license", web::get().to(get_license));
    }
}

// do_get_access_tokens is used in the framework middleware so it has to be public
pub fn do_get_access_tokens(req: &HttpRequest, account_id: u64) -> Result<Vec<AccountToken>> {
    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    AccountToken::list(account_id, &conn).map_err(Error::DieselError)
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
async fn get_account(req: HttpRequest, state: Data<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id() as i64,
        Err(_err) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Account::get_by_id(account_id, &conn).map_err(Error::DieselError) {
        Ok(account) => HttpResponse::Ok().json(account),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_access_tokens(req: HttpRequest) -> HttpResponse {
    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    match do_get_access_tokens(&req, account_id) {
        Ok(tokens) => {
            let json = json!({
                "tokens": serde_json::to_value(tokens).unwrap()
            });

            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                              .json(json)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn generate_access_token(req: HttpRequest, state: Data<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Memcache supports multiple tokens but to preserve legacy behavior
    // we must purge any existing tokens AFTER generating new ones
    let access_tokens = match AccountToken::list(account_id, &conn).map_err(Error::DieselError) {
        Ok(access_tokens) => access_tokens,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    // TODO: Provide an API for this
    let flags = {
        let extension = req.extensions();
        let session = extension.get::<originsrv::Session>().unwrap();
        session.get_flags()
    };

    let token = match CoreAccessToken::user_token(&state.config.api.key_path, account_id, flags) {
        Ok(token) => token.to_string(),
        Err(err) => {
            debug!("{}", err);
            return Error::from(err).into();
        }
    };

    let new_token = NewAccountToken { account_id: account_id as i64,
                                      token:      &token, };

    match AccountToken::create(&new_token, &conn).map_err(Error::DieselError) {
        Ok(account_token) => {
            let mut memcache = state.memcache.borrow_mut();
            for token in access_tokens {
                memcache.delete_session_key(&token.token)
            }
            HttpResponse::Ok().json(account_token)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn revoke_access_token(req: HttpRequest,
                             path: Path<String>,
                             state: Data<AppState>)
                             -> HttpResponse {
    let token_id_str = path.into_inner();
    let token_id = match token_id_str.parse::<u64>() {
        Ok(id) => id,
        Err(_) => {
            let body = Bytes::from_static(b"Error parsing access token.");
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
        }
    };

    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let access_tokens = match AccountToken::list(account_id, &conn).map_err(Error::DieselError) {
        Ok(access_tokens) => access_tokens,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let valid_token = access_tokens.iter()
                                   .find(|token| token.id == token_id as i64);

    if valid_token.is_none() {
        let body = Bytes::from_static(b"Unauthorized access.");
        return HttpResponse::with_body(StatusCode::UNAUTHORIZED, BoxBody::new(body));
    }

    match AccountToken::delete(token_id, &conn).map_err(Error::DieselError) {
        Ok(_) => {
            let mut memcache = state.memcache.borrow_mut();
            for token in access_tokens {
                memcache.delete_session_key(&token.token)
            }
            HttpResponse::Ok().finish()
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn set_license(req: HttpRequest,
                     state: Data<AppState>,
                     Json(payload): Json<LicensePayload>)
                     -> HttpResponse {
    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match authorize_session(&req, None, None) {
        Ok(_session) => {
            let expiration_date =
                match fetch_license_expiration(&payload.license_key,
                                               &state.config.api.license_server_url)
                {
                    Ok(date) => date,
                    Err(err) => {
                        return err;
                    }
                };

            let new_license =
                NewLicenseKey { account_id: payload.account_id.trim().parse::<i64>().unwrap(),
                                license_key: &payload.license_key,
                                expiration_date };

            match LicenseKey::create(&new_license, &conn).map_err(Error::DieselError) {
                Ok(license) => {
                    HttpResponse::Ok().json(json!({
                              "expiration_date": license.expiration_date.to_string()
                          }))
                }
                Err(err) => {
                    debug!("{}", err);
                    err.into()
                }
            }
        }
        Err(err) => err.into(),
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn delete_license(req: HttpRequest, state: Data<AppState>) -> HttpResponse {
    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id() as i64,
        Err(err) => return err.into(),
    };

    match LicenseKey::delete_by_account_id(account_id, &conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_license(req: HttpRequest, state: Data<AppState>) -> HttpResponse {
    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id() as i64,
        Err(err) => return err.into(),
    };

    match LicenseKey::get_by_account_id(account_id, &conn).map_err(Error::DieselError) {
        Ok(Some(license)) => {
            HttpResponse::Ok().json(json!({
                                        "license_key": license.license_key,
                                        "expiration_date": license.expiration_date
                                    }))
        }
        Ok(None) => HttpResponse::Ok().json(json!({})),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn update_account(req: HttpRequest,
                        body: Json<UserUpdateReq>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(_err) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    if body.email.is_empty() {
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Account::update(account_id, &body.email, &conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

pub fn fetch_license_expiration(license_key: &str,
                                base_url: &str)
                                -> std::result::Result<NaiveDate, HttpResponse> {
    let license_url = format!("{}/License/download?licenseId={}&version=2",
                              base_url.trim_end_matches('/'),
                              license_key);

    let response =
        reqwest::blocking::Client::new().get(license_url)
                                        .header("Accept", "application/json")
                                        .send()
                                        .map_err(|e| {
                                            debug!("License API request failed: {}", e);
                                            HttpResponse::BadRequest().body(format!("License API \
                                                                                     error: {}",
                                                                                    e))
                                        })?;

    let status = response.status();
    let body = response.text().map_err(|e| {
                                   debug!("Failed to read license server response: {}", e);
                                   HttpResponse::BadRequest().body(format!("Failed to read \
                                                                            license server \
                                                                            response: {}",
                                                                           e))
                               })?;

    if !status.is_success() {
        debug!("License server returned error: {}", body);
        return Err(HttpResponse::BadRequest().body(body));
    }

    let json: Value = serde_json::from_str(&body).map_err(|e| {
                          debug!("Failed to parse license server response: {}", e);
                          HttpResponse::BadRequest().body(format!("JSON parse error: {}", e))
                      })?;

    let entitlements = json["entitlements"].as_array()
                                           .filter(|ents| !ents.is_empty())
                                           .ok_or_else(|| {
                                               debug!("No entitlements found in license data");
                                               HttpResponse::BadRequest().body("Invalid license \
                                                                                key.")
                                           })?;

    let today = chrono::Utc::now().date_naive();

    let expiration = entitlements.iter().find_map(|ent| {
                                            let end_str = ent.get("period")?.get("end")?.as_str()?;
                                            match NaiveDate::parse_from_str(end_str, "%Y-%m-%d") {
                                                Ok(end_date) if end_date >= today => Some(end_date),
                                                _ => None,
                                            }
                                        });

    expiration.ok_or_else(|| {
                  debug!("No valid (non-expired) entitlement found in license");
                  HttpResponse::BadRequest().body("License key has expired.")
              })
}
