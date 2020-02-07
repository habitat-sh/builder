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

// TODO: Origins is still huge ... should it break down further into
// sub-resources?

use std::{collections::HashMap,
          str::from_utf8};

use actix_web::{body::Body,
                http::{self,
                       header::{Charset,
                                ContentDisposition,
                                DispositionParam,
                                DispositionType,
                                ExtendedValue},
                       StatusCode},
                web::{self,
                      Bytes as ActixBytes,
                      Data,
                      Json,
                      Path,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};
use builder_core::Error::OriginDeleteError;
use bytes::Bytes;
use diesel::{pg::PgConnection,
             result::Error::NotFound};
use serde_json;

use crate::{bldr_core,
            hab_core::{crypto::{keys::{box_key_pair::WrappedSealedBox,
                                       parse_key_str,
                                       parse_name_with_rev,
                                       PairType},
                                BoxKeyPair,
                                SigKeyPair},
                       package::{ident,
                                 PackageIdent}}};

use crate::protocol::originsrv::OriginKeyIdent;

use crate::db::models::{account::*,
                        channel::Channel,
                        integration::*,
                        invitations::*,
                        keys::*,
                        origin::*,
                        package::{BuilderPackageIdent,
                                  ListPackages,
                                  Package,
                                  PackageVisibility},
                        projects::Project,
                        secrets::*};

use crate::server::{authorize::{authorize_session,
                                check_origin_member,
                                check_origin_owner},
                    error::{Error,
                            Result},
                    framework::headers,
                    helpers::{self,
                              req_state,
                              Pagination},
                    resources::pkgs::postprocess_package_list,
                    AppState};

#[derive(Clone, Serialize, Deserialize)]
struct OriginSecretPayload {
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CreateOriginHandlerReq {
    pub name: String,
    pub default_package_visibility: Option<PackageVisibility>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateOriginHandlerReq {
    pub default_package_visibility: Option<PackageVisibility>,
}

pub struct Origins {}

impl Origins {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/depot/{origin}/pkgs", web::get().to(list_unique_packages))
           .route("/depot/origins/{origin}", web::get().to(get_origin))
           .route("/depot/origins/{origin}", web::put().to(update_origin))
           .route("/depot/origins/{origin}", web::delete().to(delete_origin))
           .route("/depot/origins", web::post().to(create_origin))
           .route("/depot/origins/{origin}/users",
                  web::get().to(list_origin_members))
           .route("/depot/origins/{origin}/users/{user}",
                  web::delete().to(origin_member_delete))
           .route("/depot/origins/{origin}/transfer/{user}",
                  web::post().to(transfer_origin_ownership))
           .route("/depot/origins/{origin}/depart",
                  web::post().to(depart_from_origin))
           .route("/depot/origins/{origin}/invitations",
                  web::get().to(list_origin_invitations))
           .route("/depot/origins/{origin}/users/{username}/invitations",
                  web::post().to(invite_to_origin))
           .route("/depot/origins/{origin}/invitations/{invitation_id}",
                  web::put().to(accept_invitation))
           .route("/depot/origins/{origin}/invitations/{invitation_id}",
                  web::delete().to(rescind_invitation))
           .route("/depot/origins/{origin}/invitations/{invitation_id}/ignore",
                  web::put().to(ignore_invitation))
           .route("/depot/origins/{origin}/keys/latest",
                  web::get().to(download_latest_origin_key))
           .route("/depot/origins/{origin}/keys", web::post().to(create_keys))
           .route("/depot/origins/{origin}/keys",
                  web::get().to(list_origin_keys))
           .route("/depot/origins/{origin}/keys/{revision}",
                  web::post().to(upload_origin_key))
           .route("/depot/origins/{origin}/keys/{revision}",
                  web::get().to(download_origin_key))
           .route("/depot/origins/{origin}/secret",
                  web::get().to(list_origin_secrets))
           .route("/depot/origins/{origin}/secret",
                  web::post().to(create_origin_secret))
           .route("/depot/origins/{origin}/encryption_key",
                  web::get().to(download_latest_origin_encryption_key))
           .route("/depot/origins/{origin}/integrations",
                  web::get().to(fetch_origin_integrations))
           .route("/depot/origins/{origin}/secret/{secret}",
                  web::delete().to(delete_origin_secret))
           .route("/depot/origins/{origin}/secret_keys/latest",
                  web::get().to(download_latest_origin_secret_key))
           .route("/depot/origins/{origin}/secret_keys/{revision}",
                  web::post().to_async(upload_origin_secret_key))
           .route("/depot/origins/{origin}/integrations/{integration}/names",
                  web::get().to(fetch_origin_integration_names))
           .route("/depot/origins/{origin}/integrations/{integration}/{name}",
                  web::get().to(get_origin_integration))
           .route("/depot/origins/{origin}/integrations/{integration}/{name}",
                  web::delete().to(delete_origin_integration))
           .route("/depot/origins/{origin}/integrations/{integration}/{name}",
                  web::put().to_async(create_origin_integration));
    }
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
fn get_origin(path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin_name = path.into_inner();

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Origin::get(&origin_name, &*conn) {
        Ok(origin) => {
            HttpResponse::Ok().header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                              .json(origin)
        }
        Err(NotFound) => HttpResponse::NotFound().into(),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_origin(req: HttpRequest,
                 body: Json<CreateOriginHandlerReq>,
                 state: Data<AppState>)
                 -> HttpResponse {
    let session = match authorize_session(&req, None) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    let dpv = match body.clone().default_package_visibility {
        Some(viz) => viz,
        None => PackageVisibility::Public,
    };

    if !ident::is_valid_origin_name(&body.name) {
        return HttpResponse::ExpectationFailed().into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let new_origin = NewOrigin { name: &body.0.name,
                                 owner_id: session.get_id() as i64,
                                 default_package_visibility: &dpv, };

    match Origin::create(&new_origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => HttpResponse::Created().json(origin),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn update_origin(req: HttpRequest,
                 path: Path<String>,
                 body: Json<UpdateOriginHandlerReq>,
                 state: Data<AppState>)
                 -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let dpv = match body.0.default_package_visibility {
        Some(viz) => viz,
        None => PackageVisibility::Public,
    };

    match Origin::update(&origin, dpv, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().into(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_origin(req: HttpRequest, path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    let session = match authorize_session(&req, None) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    if !check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    debug!("Request to delete origin {}", &origin);

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Prior to passing the deletion request to the backend, we validate
    // that the user has already cleaned up the most critical origin data.
    match origin_delete_preflight(&origin, &*conn) {
        Ok(_) => {
            match Origin::delete(&origin, &*conn).map_err(Error::DieselError) {
                Ok(_) => HttpResponse::NoContent().into(),
                Err(err) => {
                    debug!("Origin {} deletion failed! err = {}", origin, err);
                    // We do not want to expose any database details from diesel
                    // thus we simply return a 409 with an empty body.
                    HttpResponse::new(StatusCode::CONFLICT)
                }
            }
        }
        Err(err) => {
            debug!("Origin preflight determined that {} is not deletable, err = {}!",
                   origin, err);
            // Here we want to enrich the http response with a sanitized error
            // by returning a 409 with a helpful message in the body.
            HttpResponse::with_body(StatusCode::CONFLICT, Body::from_message(format!("{}", err)))
        }
    }
}

fn origin_delete_preflight(origin: &str, conn: &PgConnection) -> Result<()> {
    match Project::count_origin_projects(&origin, &*conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} projects remaining in origin {}. Must be zero.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => return Err(Error::DieselError(e)),
    };

    match OriginMember::count_origin_members(&origin, &*conn) {
        // allow 1 - the origin owner
        Ok(1) => {}
        Ok(count) => {
            let err = format!("There are {} members remaining in origin {}. Only one is allowed.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    match OriginSecret::count_origin_secrets(&origin, &*conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} secrets remaining in origin {}. Must be zero.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    match OriginIntegration::count_origin_integrations(&origin, &*conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} integrations remaining in origin {}. Must be zero.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    match Channel::count_origin_channels(&origin, &*conn) {
        // allow 2 - [unstable, stable] channels cannot be deleted
        Ok(2) => {}
        Ok(count) => {
            let err = format!("There are {} channels remaining in origin {}. Only two are \
                               allowed [unstable, stable].",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    match Package::count_origin_packages(&origin, &*conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} packages remaining in origin {}. Must be zero.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
fn create_keys(req: HttpRequest, path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let pair = match SigKeyPair::generate_pair_for_origin(&origin).map_err(Error::HabitatCore) {
        Ok(pair) => pair,
        Err(err) => {
            error!("Failed to generate origin key pair for {}, err={}",
                   origin, err);
            return err.into();
        }
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => {
            error!("create_keys: Failed to get DB connection, err={}", err);
            return err.into();
        }
    };

    let pk_body = match pair.to_public_string().map_err(Error::HabitatCore) {
        Ok(pk) => pk.into_bytes(),
        Err(err) => {
            error!("create_keys: Failed to get pk body, err={}", err);
            return err.into();
        }
    };

    let new_pk = NewOriginPublicSigningKey { owner_id:  account_id as i64,
                                             origin:    &origin,
                                             full_name: &format!("{}-{}", &origin, &pair.rev),
                                             name:      &origin,
                                             revision:  &pair.rev,
                                             body:      &pk_body, };

    match OriginPublicSigningKey::create(&new_pk, &*conn).map_err(Error::DieselError) {
        Ok(_) => (),
        Err(err) => {
            error!("create_keys: Failed to create public key, err={}", err);
            return err.into();
        }
    }

    let sk_body = match pair.to_secret_string().map_err(Error::HabitatCore) {
        Ok(sk) => sk.into_bytes(),
        Err(err) => {
            error!("create_keys: Failed to get sk body, err={}", err);
            return err.into();
        }
    };

    let (sk_encrypted, bldr_key_rev) = match encrypt(&req, &Bytes::from(sk_body)) {
        Ok((encrypted, rev)) => (encrypted, rev),
        Err(err) => {
            debug!("create_keys: Failed to encrypt sk_body, err={:?}", err);
            return err.into();
        }
    };

    let new_sk = NewOriginPrivateSigningKey { owner_id:           account_id as i64,
                                              origin:             &origin,
                                              full_name:          &format!("{}-{}",
                                                                           &origin, &pair.rev),
                                              name:               &origin,
                                              revision:           &pair.rev,
                                              body:               &sk_encrypted.as_bytes(),
                                              encryption_key_rev: &bldr_key_rev, };

    match OriginPrivateSigningKey::create(&new_sk, &*conn).map_err(Error::DieselError) {
        Ok(_) => (),
        Err(err) => {
            error!("create_keys: Failed to create private key, err={}", err);
            return err.into();
        }
    }

    HttpResponse::Created().finish()
}

#[allow(clippy::needless_pass_by_value)]
fn list_origin_keys(path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginPublicSigningKey::list(&origin, &*conn).map_err(Error::DieselError) {
        Ok(list) => {
            let list: Vec<OriginKeyIdent> =
                list.iter()
                    .map(|key| {
                        let mut ident = OriginKeyIdent::new();
                        ident.set_location(format!("/origins/{}/keys/{}",
                                                   &key.name, &key.revision));
                        ident.set_origin(key.name.to_string());
                        ident.set_revision(key.revision.to_string());
                        ident
                    })
                    .collect();

            HttpResponse::Ok().header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                              .json(&list)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn upload_origin_key(req: HttpRequest,
                     body: String,
                     path: Path<(String, String)>,
                     state: Data<AppState>)
                     -> HttpResponse {
    let (origin, revision) = path.into_inner();

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match parse_key_str(&body) {
        Ok((PairType::Public, ..)) => {
            debug!("Received a valid public key");
        }
        Ok(_) => {
            debug!("Received a secret key instead of a public key");
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Err(e) => {
            debug!("Invalid public key content: {}", e);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    let new_pk = NewOriginPublicSigningKey { owner_id:  account_id as i64,
                                             origin:    &origin,
                                             full_name: &format!("{}-{}", &origin, &revision),
                                             name:      &origin,
                                             revision:  &revision,
                                             body:      &body.into_bytes(), };

    match OriginPublicSigningKey::create(&new_pk, &*conn).map_err(Error::DieselError) {
        Ok(_) => {
            HttpResponse::Created().header(http::header::LOCATION, format!("{}", req.uri()))
                                   .body(format!("/origins/{}/keys/{}", &origin, &revision))
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn download_origin_key(path: Path<(String, String)>, state: Data<AppState>) -> HttpResponse {
    let (origin, revision) = path.into_inner();

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key =
        match OriginPublicSigningKey::get(&origin, &revision, &*conn).map_err(Error::DieselError) {
            Ok(key) => key,
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let xfilename = format!("{}-{}.pub", key.name, key.revision);
    download_content_as_file(&key.body, xfilename)
}

#[allow(clippy::needless_pass_by_value)]
fn download_latest_origin_key(path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match OriginPublicSigningKey::latest(&origin, &*conn).map_err(Error::DieselError) {
        Ok(key) => key,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let xfilename = format!("{}-{}.pub", key.name, key.revision);
    download_content_as_file(&key.body, xfilename)
}

#[allow(clippy::needless_pass_by_value)]
fn list_origin_secrets(req: HttpRequest,
                       path: Path<String>,
                       state: Data<AppState>)
                       -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginSecret::list(&origin, &*conn).map_err(Error::DieselError) {
        Ok(list) => {
            // Need to map to different struct for hab cli backward compat
            let new_list: Vec<OriginSecretWithOriginId> =
                list.into_iter().map(|s| s.into()).collect();
            HttpResponse::Ok().header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                              .json(&new_list)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_origin_secret(req: HttpRequest,
                        body: Json<OriginSecretPayload>,
                        path: Path<String>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let origin = path.into_inner();

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id() as i64,
        Err(err) => return err.into(),
    };

    if body.name.is_empty() {
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                       Body::from_message("Missing value for field `name`"));
    }

    if body.value.is_empty() {
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                       Body::from_message("Missing value for field `value`"));
    }

    // get metadata from secret payload
    let ciphertext = WrappedSealedBox::from(body.value.as_str());
    let secret_metadata = match BoxKeyPair::secret_metadata(&ciphertext) {
        Ok(res) => {
            debug!("Secret Metadata: {:?}", res);
            res
        }
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                           Body::from_message(format!("Failed to get metadata \
                                                                       from payload: {}",
                                                                      err)));
        }
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // fetch the private origin encryption key from the database
    let priv_key =
        match OriginPrivateEncryptionKey::get(&origin, &*conn).map_err(Error::DieselError) {
            Ok(key) => {
                let key_str = from_utf8(&key.body).unwrap();
                match BoxKeyPair::secret_key_from_str(key_str) {
                    Ok(key) => key,
                    Err(err) => {
                        debug!("{}", err);
                        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                                       Body::from_message(format!("Failed to get secret from \
                                                                payload: {}",
                                                               err)));
                    }
                }
            }
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let (name, rev) = match parse_name_with_rev(secret_metadata.sender) {
        Ok(val) => val,
        Err(e) => {
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                           Body::from_message(format!("Failed to parse name \
                                                                       and revision: {}",
                                                                      e)));
        }
    };

    debug!("Using key {:?}-{:?}", name, &rev);

    // fetch the public origin encryption key from the database
    let pub_key =
        match OriginPublicEncryptionKey::get(&origin, &rev, &*conn).map_err(Error::DieselError) {
            Ok(key) => {
                let key_str = from_utf8(&key.body).unwrap();
                match BoxKeyPair::public_key_from_str(key_str) {
                    Ok(key) => key,
                    Err(err) => {
                        debug!("{}", err);
                        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                                       Body::from_message(format!("{}", err)));
                    }
                }
            }
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let box_key_pair = BoxKeyPair::new(name, rev, Some(pub_key), Some(priv_key));

    debug!("Decrypting string: {:?}", &secret_metadata.ciphertext);

    // verify we can decrypt the message
    match box_key_pair.decrypt(&secret_metadata.ciphertext, None, None) {
        Ok(_) => (),
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                           Body::from_message(format!("{}", err)));
        }
    };

    match OriginSecret::create(&NewOriginSecret { origin:   &origin,
                                                  name:     &body.name,
                                                  value:    &body.value,
                                                  owner_id: account_id, },
                               &*conn).map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_origin_secret(req: HttpRequest,
                        path: Path<(String, String)>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let (origin, secret) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginSecret::delete(&origin, &secret, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn upload_origin_secret_key(req: HttpRequest,
                            path: Path<(String, String)>,
                            body: ActixBytes,
                            state: Data<AppState>)
                            -> HttpResponse {
    let (origin, revision) = path.into_inner();

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match String::from_utf8(body.to_vec()) {
        Ok(content) => {
            match parse_key_str(&content) {
                Ok((PairType::Secret, ..)) => {
                    debug!("Received a valid secret key");
                }
                Ok(_) => {
                    debug!("Received a public key instead of a secret key");
                    return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                }
                Err(e) => {
                    debug!("Invalid secret key content: {}", e);
                    return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                }
            }
        }
        Err(e) => {
            debug!("Can't parse secret key upload content: {}", e);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    let (encrypted_body, bldr_key_rev) = match encrypt(&req, &body) {
        Ok((encrypted, rev)) => (encrypted, rev),
        Err(err) => {
            debug!("Failed to encrypt body, err={:?}", err);
            return err.into();
        }
    };

    let new_sk = NewOriginPrivateSigningKey { owner_id:           account_id as i64,
                                              origin:             &origin,
                                              name:               &origin,
                                              full_name:          &format!("{}-{}",
                                                                           &origin, &revision),
                                              revision:           &revision,
                                              body:               &encrypted_body.as_bytes(),
                                              encryption_key_rev: &bldr_key_rev, };

    match OriginPrivateSigningKey::create(&new_sk, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn download_latest_origin_secret_key(req: HttpRequest,
                                     path: Path<String>,
                                     state: Data<AppState>)
                                     -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match OriginPrivateSigningKey::get(&origin, &*conn).map_err(Error::DieselError) {
        Ok(key) => key,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let key_body = if key.encryption_key_rev.is_some() {
        let str_body = match String::from_utf8(key.body).map_err(Error::Utf8) {
            Ok(s) => s,
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };
        match decrypt(&req, &str_body) {
            Ok(decrypted) => decrypted.as_bytes().to_vec(),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        }
    } else {
        key.body
    };

    let xfilename = format!("{}-{}.sig.key", key.name, key.revision);
    download_content_as_file(&key_body, xfilename)
}

#[allow(clippy::needless_pass_by_value)]
fn list_unique_packages(req: HttpRequest,
                        pagination: Query<Pagination>,
                        path: Path<String>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let origin = path.into_inner();

    let opt_session_id = match authorize_session(&req, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => {
            return {
                debug!("{}", err);
                err.into()
            };
        }
    };

    let ident = PackageIdent::new(origin.clone(), String::from(""), None, None);

    let (page, per_page) = helpers::extract_pagination_in_pages(&pagination);

    let lpr = ListPackages { ident:      BuilderPackageIdent(ident),
                             visibility: helpers::visibility_for_optional_session(&req,
                                                                                  opt_session_id,
                                                                                  &origin),
                             page:       page as i64,
                             limit:      per_page as i64, };

    match Package::distinct_for_origin(lpr, &*conn) {
        Ok((packages, count)) => postprocess_package_list(&req, &packages, count, &pagination),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn download_latest_origin_encryption_key(req: HttpRequest,
                                         path: Path<String>,
                                         state: Data<AppState>)
                                         -> HttpResponse {
    let origin = path.into_inner();

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match OriginPublicEncryptionKey::latest(&origin, &*conn) {
        Ok(key) => key,
        Err(NotFound) => {
            // TODO: redesign to not be generating keys during d/l
            match generate_origin_encryption_keys(&origin, account_id, &conn) {
                Ok(key) => key,
                Err(err) => {
                    debug!("{}", err);
                    return err.into();
                }
            }
        }
        Err(err) => {
            debug!("{}", err);
            return Error::DieselError(err).into();
        }
    };

    let xfilename = format!("{}-{}.pub", key.name, key.revision);
    download_content_as_file(&key.body, xfilename)
}

#[allow(clippy::needless_pass_by_value)]
fn invite_to_origin(req: HttpRequest,
                    path: Path<(String, String)>,
                    state: Data<AppState>)
                    -> HttpResponse {
    let (origin, user) = path.into_inner();

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    debug!("Creating invitation for user {} origin {}", &user, &origin);

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (recipient_id, recipient_name) =
        match Account::get(&user, &*conn).map_err(Error::DieselError) {
            Ok(account) => (account.id, account.name),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let new_invitation = NewOriginInvitation { origin:       &origin,
                                               account_id:   recipient_id,
                                               account_name: &recipient_name,
                                               owner_id:     account_id as i64, };

    // store invitations in the originsrv
    match OriginInvitation::create(&new_invitation, &*conn).map_err(Error::DieselError) {
        Ok(invitation) => HttpResponse::Created().json(&invitation),
        // TODO (SA): Check for error case where invitation already exists
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn accept_invitation(req: HttpRequest,
                     path: Path<(String, String)>,
                     state: Data<AppState>)
                     -> HttpResponse {
    let (origin, invitation) = path.into_inner();

    let account_id = match authorize_session(&req, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let invitation_id = match invitation.parse::<u64>() {
        Ok(invitation_id) => invitation_id,
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
    };

    debug!("Accepting invitation for user {} origin {}",
           account_id, origin);

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::accept(invitation_id, false, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn ignore_invitation(req: HttpRequest,
                     path: Path<(String, String)>,
                     state: Data<AppState>)
                     -> HttpResponse {
    let (origin, invitation) = path.into_inner();

    let _ = match authorize_session(&req, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let invitation_id = match invitation.parse::<u64>() {
        Ok(invitation_id) => invitation_id,
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    debug!("Ignoring invitation id {} for origin {}",
           invitation_id, &origin);

    match OriginInvitation::ignore(invitation_id, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn rescind_invitation(req: HttpRequest,
                      path: Path<(String, String)>,
                      state: Data<AppState>)
                      -> HttpResponse {
    let (origin, invitation) = path.into_inner();

    let _ = match authorize_session(&req, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let invitation_id = match invitation.parse::<u64>() {
        Ok(invitation_id) => invitation_id,
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    debug!("Rescinding invitation id {} for user from origin {}",
           invitation_id, &origin);

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::rescind(invitation_id, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn list_origin_invitations(req: HttpRequest,
                           path: Path<String>,
                           state: Data<AppState>)
                           -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::list_by_origin(&origin, &*conn).map_err(Error::DieselError) {
        Ok(list) => {
            let json = json!({
                "origin": &origin,
                "invitations": serde_json::to_value(list).unwrap()
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
fn transfer_origin_ownership(req: HttpRequest,
                             path: Path<(String, String)>,
                             state: Data<AppState>)
                             -> HttpResponse {
    let (origin, user) = path.into_inner();

    let session = match authorize_session(&req, None) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    if !check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // Do not allow the owner to transfer ownership to themselves
    if user == session.get_name() {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    debug!(" Transferring origin {} to new owner {}", &origin, &user);

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (recipient_id, _recipient_name) =
        match Account::get(&user, &*conn).map_err(Error::DieselError) {
            Ok(account) => (account.id, account.name),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    // Do not allow transfer to recipent that is not already an origin member
    if !check_origin_member(&req, &origin, recipient_id as u64).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    match Origin::transfer(&origin, recipient_id, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn depart_from_origin(req: HttpRequest, path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    let session = match authorize_session(&req, None) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    // Do not allow an origin owner to depart which would orphan the origin
    if check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // Pass a meaningful error in the case that the user isn't a member of origin
    if !check_origin_member(&req, &origin, session.get_id()).unwrap_or(false) {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    debug!("Departing user {} from origin {}",
           session.get_name(),
           &origin);

    match Origin::depart(&origin, session.get_id() as i64, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn list_origin_members(req: HttpRequest,
                       path: Path<String>,
                       state: Data<AppState>)
                       -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginMember::list(&origin, &*conn).map_err(Error::DieselError) {
        Ok(users) => {
            let json = json!({
                "origin": &origin,
                "members": serde_json::to_value(users).unwrap()
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
fn origin_member_delete(req: HttpRequest,
                        path: Path<(String, String)>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let (origin, user) = path.into_inner();

    let session = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    if !check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // Do not allow the owner to be removed which would orphan the origin
    if user == session.get_name() {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    debug!("Deleting origin member {} from origin {}", &user, &origin);

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginMember::delete(&origin, &user, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn fetch_origin_integrations(req: HttpRequest,
                             path: Path<String>,
                             state: Data<AppState>)
                             -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::list_for_origin(&origin, &*conn).map_err(Error::DieselError) {
        Ok(oir) => {
            let integrations_response: HashMap<String, Vec<String>> =
                oir.iter().fold(HashMap::new(), |mut acc, ref i| {
                              acc.entry(i.integration.to_owned())
                                 .or_insert_with(Vec::new)
                                 .push(i.name.to_owned());
                              acc
                          });
            HttpResponse::Ok().header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                              .json(integrations_response)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn fetch_origin_integration_names(req: HttpRequest,
                                  path: Path<(String, String)>,
                                  state: Data<AppState>)
                                  -> HttpResponse {
    let (origin, integration) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::list_for_origin_integration(&origin, &integration, &*conn)
        .map_err(Error::DieselError)
    {
        Ok(integrations) => {
            let names: Vec<String> = integrations.iter().map(|i| i.name.to_string()).collect();
            let mut hm: HashMap<String, Vec<String>> = HashMap::new();
            hm.insert("names".to_string(), names);
            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(hm)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_origin_integration(req: HttpRequest,
                             path: Path<(String, String, String)>,
                             body: ActixBytes,
                             state: Data<AppState>)
                             -> HttpResponse {
    let (origin, integration, name) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (encrypted, _) = match encrypt(&req, &body) {
        Ok(encrypted) => encrypted,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let noi = NewOriginIntegration { origin:      &origin,
                                     integration: &integration,
                                     name:        &name,
                                     body:        &encrypted, };

    match OriginIntegration::create(&noi, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_origin_integration(req: HttpRequest,
                             path: Path<(String, String, String)>,
                             state: Data<AppState>)
                             -> HttpResponse {
    let (origin, integration, name) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::delete(&origin, &integration, &name, &*conn)
        .map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_origin_integration(req: HttpRequest,
                          path: Path<(String, String, String)>,
                          state: Data<AppState>)
                          -> HttpResponse {
    let (origin, integration, name) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::get(&origin, &integration, &name, &*conn).map_err(Error::DieselError) {
        Ok(integration) => {
            match decrypt(&req, &integration.body) {
                Ok(decrypted) => {
                    let val = serde_json::from_str(&decrypted).unwrap();
                    let mut map: serde_json::Map<String, serde_json::Value> =
                        serde_json::from_value(val).unwrap();

                    map.remove("password");

                    let sanitized = json!({
                        "origin": integration.origin.to_string(),
                        "integration": integration.integration.to_string(),
                        "name": integration.name.to_string(),
                        "body": serde_json::to_value(map).unwrap()
                    });

                    HttpResponse::Ok().header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                                      .json(sanitized)
                }
                Err(err) => {
                    debug!("{}", err);
                    err.into()
                }
            }
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// Internal helpers
//

fn download_content_as_file(content: &[u8], filename: String) -> HttpResponse {
    HttpResponse::Ok()
        .header(
            http::header::CONTENT_DISPOSITION,
            ContentDisposition {
                disposition: DispositionType::Attachment,
                parameters: vec![DispositionParam::FilenameExt(ExtendedValue {
                    charset: Charset::Iso_8859_1, // The character set for the bytes of the filename
                    language_tag: None, // The optional language tag (see `language-tag` crate)
                    value: filename.as_bytes().to_vec(), // the actual bytes of the filename
                })],
            },
        )
        .header(
            http::header::HeaderName::from_static(headers::XFILENAME),
            filename,
        )
        .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
        .body(Bytes::from(content))
}

fn generate_origin_encryption_keys(origin: &str,
                                   session_id: u64,
                                   conn: &PgConnection)
                                   -> Result<OriginPublicEncryptionKey> {
    debug!("Generating encryption keys for {}", origin);

    let pair = BoxKeyPair::generate_pair_for_origin(origin).map_err(Error::HabitatCore)?;

    let pk_body = pair.to_public_string()
                      .map_err(Error::HabitatCore)?
                      .into_bytes();

    let new_pk = NewOriginPublicEncryptionKey { owner_id:  session_id as i64,
                                                name:      &origin,
                                                origin:    &origin,
                                                full_name: &format!("{}-{}", &origin, &pair.rev),
                                                revision:  &pair.rev,
                                                body:      &pk_body, };

    let sk_body = pair.to_secret_string()
                      .map_err(Error::HabitatCore)?
                      .into_bytes();

    let new_sk = NewOriginPrivateEncryptionKey { owner_id:  session_id as i64,
                                                 name:      origin,
                                                 origin:    &origin,
                                                 full_name: &format!("{}-{}", &origin, &pair.rev),
                                                 revision:  &pair.rev,
                                                 body:      &sk_body, };

    OriginPrivateEncryptionKey::create(&new_sk, &*conn)?;
    OriginPublicEncryptionKey::create(&new_pk, &*conn).map_err(Error::DieselError)
}

fn encrypt(req: &HttpRequest, content: &Bytes) -> Result<(String, String)> {
    bldr_core::integrations::encrypt(&req_state(req).config.api.key_path, content)
        .map_err(Error::BuilderCore)
}

fn decrypt(req: &HttpRequest, content: &str) -> Result<String> {
    let bytes = bldr_core::integrations::decrypt(&req_state(req).config.api.key_path, content)?;
    Ok(String::from_utf8(bytes)?)
}
