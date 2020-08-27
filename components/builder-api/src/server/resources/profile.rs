use actix_web::{http::{self,
                       StatusCode},
                web::{self,
                      Data,
                      Json,
                      Path,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};

use crate::{bldr_core,
            protocol::originsrv};

use crate::db::models::account::*;

use crate::server::{authorize::authorize_session,
                    error::{Error,
                            Result},
                    framework::headers,
                    helpers::req_state,
                    AppState};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserUpdateReq {
    #[serde(default)]
    pub email: String,
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
                  web::delete().to(revoke_access_token));
    }
}

// do_get_access_tokens is used in the framework middleware so it has to be public
pub fn do_get_access_tokens(req: &HttpRequest, account_id: u64) -> Result<Vec<AccountToken>> {
    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    AccountToken::list(account_id, &*conn).map_err(Error::DieselError)
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
fn get_account(req: HttpRequest, state: Data<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id() as i64,
        Err(_err) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Account::get_by_id(account_id, &*conn).map_err(Error::DieselError) {
        Ok(account) => HttpResponse::Ok().json(account),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_access_tokens(req: HttpRequest) -> HttpResponse {
    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    match do_get_access_tokens(&req, account_id) {
        Ok(tokens) => {
            let json = json!({
                "tokens": serde_json::to_value(tokens).unwrap()
            });

            HttpResponse::Ok().header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                              .json(json)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn generate_access_token(req: HttpRequest, state: Data<AppState>) -> HttpResponse {
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
    let access_tokens = match AccountToken::list(account_id, &*conn).map_err(Error::DieselError) {
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

    let token = bldr_core::access_token::generate_user_token(&state.config.api.key_path,
                                                             account_id,
                                                             flags).unwrap();

    let new_token = NewAccountToken { account_id: account_id as i64,
                                      token:      &token, };

    match AccountToken::create(&new_token, &*conn).map_err(Error::DieselError) {
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
fn revoke_access_token(req: HttpRequest,
                       path: Path<String>,
                       state: Data<AppState>)
                       -> HttpResponse {
    let token_id_str = path.into_inner();
    let token_id = match token_id_str.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
    };

    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let access_tokens = match AccountToken::list(account_id, &*conn).map_err(Error::DieselError) {
        Ok(access_tokens) => access_tokens,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    match AccountToken::delete(token_id, &*conn).map_err(Error::DieselError) {
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
fn update_account(req: HttpRequest,
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

    match Account::update(account_id, &body.email, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}
