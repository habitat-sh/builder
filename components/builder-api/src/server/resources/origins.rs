// TODO: Origins is still huge ... should it break down further into
// sub-resources?

use crate::{bldr_core::crypto,
            db::models::{account::*,
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
                         secrets::*,
                         settings::OriginPackageSettings},
            protocol::originsrv::OriginKeyIdent,
            server::{authorize::{authorize_session,
                                 check_origin_member,
                                 check_origin_owner},
                     error::{Error,
                             Result},
                     framework::headers,
                     helpers::{self,
                               role_results_json,
                               Pagination,
                               Role},
                     resources::pkgs::postprocess_package_list,
                     AppState}};
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
use habitat_core::{crypto::keys::{generate_origin_encryption_key_pair,
                                  generate_signing_key_pair,
                                  AnonymousBox,
                                  Key,
                                  KeyCache,
                                  KeyFile,
                                  OriginSecretEncryptionKey,
                                  PublicOriginSigningKey,
                                  SecretOriginSigningKey},
                   package::{ident,
                             PackageIdent}};
use std::{collections::HashMap,
          str::{from_utf8,
                FromStr}};

#[derive(Clone, Serialize, Deserialize)]
struct OriginSecretPayload {
    #[serde(default)]
    name:  String,
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
           .route("/depot/origins/{origin}/users/{username}/role",
                  web::get().to(get_origin_member_role))
           .route("/depot/origins/{origin}/users/{username}/role",
                  web::put().to(update_origin_member_role))
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
                  web::post().to(upload_origin_secret_key))
           .route("/depot/origins/{origin}/integrations/{integration}/names",
                  web::get().to(fetch_origin_integration_names))
           .route("/depot/origins/{origin}/integrations/{integration}/{name}",
                  web::get().to(get_origin_integration))
           .route("/depot/origins/{origin}/integrations/{integration}/{name}",
                  web::delete().to(delete_origin_integration))
           .route("/depot/origins/{origin}/integrations/{integration}/{name}",
                  web::put().to(create_origin_integration));
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
    let session = match authorize_session(&req, None, None) {
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
        Ok(origin) => {
            origin_audit(&body.0.name,
                         OriginOperation::OriginCreate,
                         &body.0.name,
                         session.get_id() as i64,
                         session.get_name(),
                         &*conn);
            HttpResponse::Created().json(origin)
        }
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

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator))
    {
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

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Owner)) {
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
                Ok(_) => {
                    origin_audit(&origin,
                                 OriginOperation::OriginDelete,
                                 &origin,
                                 session.get_id() as i64,
                                 session.get_name(),
                                 &*conn);
                    HttpResponse::NoContent().into()
                }
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

    match OriginPackageSettings::count_origin_package_settings(&origin, &*conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} package settings entries remaining in origin {}. \
                               Must be zero.",
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

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => {
            error!("create_keys: Failed to get DB connection, err={}", err);
            return err.into();
        }
    };

    // For Builder, we actually don't want to go through the KeyCache
    // to create a pair, because we don't want to store anything to
    // disk. That's why we have a database.
    let (public, secret) = generate_signing_key_pair(&origin);

    if let Err(e) = save_public_origin_signing_key(account_id, &origin, &public, &*conn) {
        error!("Failed to save public signing key for origin '{}', err={}",
               origin, e);
        return e.into();
    }

    if let Err(e) = save_secret_origin_signing_key(account_id,
                                                   &origin,
                                                   &state.config.api.key_path,
                                                   &secret,
                                                   &*conn)
    {
        error!("Failed to save secret signing key for origin '{}', err={}",
               origin, e);
        return e.into();
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

    // Since this route allows users to upload keys, we verify their membership
    // before we determine if the key they're using actually exists. This is a
    // backward compatibility workaround needed when RBAC was introduced. hab
    // pkg upload optimistically uploads keys as well as packages, but the RBAC
    // changes only give 'members' and 'maintainers' upload permissions for
    // packages, not keys.
    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Member)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    if OriginPublicSigningKey::get(&origin, &revision, &conn).is_ok() {
        HttpResponse::new(StatusCode::CONFLICT)
    } else {
        // In this case we are checking if the user actually has permissions to write a
        // NEW key into the origin_public_keys data table
        let account_id =
            match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
                Ok(session) => session.get_id(),
                Err(_) => {
                    debug!("Unable to upload origin public signing key due to lack of permissions");
                    return HttpResponse::with_body(StatusCode::FORBIDDEN,
                                                   Body::from_message(format!("You do not \
                                                                               have permissions \
                                                                               to upload a \
                                                                               new origin \
                                                                               signing public \
                                                                               key: {}-{}",
                                                                              origin, revision)));
                }
            };
        let key = match body.parse::<PublicOriginSigningKey>() {
            Ok(key) => key,
            Err(e) => {
                debug!("Invalid public key content: {}", e);
                return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
            }
        };

        match save_public_origin_signing_key(account_id, &origin, &key, &*conn) {
            Ok(_) => {
                HttpResponse::Created().header(http::header::LOCATION, format!("{}", req.uri()))
                                       .body(format!("/origins/{}/keys/{}",
                                                     origin,
                                                     key.named_revision().revision()))
            }
            Err(err) => {
                debug!("{}", err);
                err.into()
            }
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

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator))
    {
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

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
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

    let anonymous_box = match body.value.parse::<AnonymousBox>() {
        Ok(res) => {
            debug!("Secret Metadata: {:?}", res);
            res
        }
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                           Body::from_message(format!("Failed to parse \
                                                                       encrypted message from \
                                                                       payload: {}",
                                                                      err)));
        }
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Fetch the origin's secret encryption key from the database
    let secret_encryption_key =
        match OriginPrivateEncryptionKey::get(&origin, &*conn).map_err(Error::DieselError) {
            Ok(key) => {
                let key_str = from_utf8(&key.body).unwrap();
                match key_str.parse::<OriginSecretEncryptionKey>() {
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

    // Though we're storing the data in its encrypted form, we still
    // need to ensure that we have the ability to decrypt it.
    if let Err(err) = secret_encryption_key.decrypt(&anonymous_box) {
        debug!("{}", err);
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                       Body::from_message(format!("{}", err)));
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

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator))
    {
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
    let (origin, _revision) = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match String::from_utf8(body.to_vec()) {
        Ok(content) => {
            match content.parse::<SecretOriginSigningKey>() {
                Ok(key) => key,
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
    };

    if let Err(e) = save_secret_origin_signing_key(account_id,
                                                   &origin,
                                                   &state.config.api.key_path,
                                                   &key,
                                                   &*conn)
    {
        error!("Failed to save uploaded secret signing key for origin '{}', err={}",
               origin, e);
        return e.into();
    }

    HttpResponse::Created().finish()
}

#[allow(clippy::needless_pass_by_value)]
fn download_latest_origin_secret_key(req: HttpRequest,
                                     path: Path<String>,
                                     state: Data<AppState>)
                                     -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Member)) {
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
        match crypto::decrypt(&state.config.api.key_path, &str_body).map_err(Error::BuilderCore) {
            Ok(decrypted) => decrypted,
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

    let opt_session_id = match authorize_session(&req, None, None) {
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
        Ok((packages, count)) => {
            postprocess_package_list(&req, packages.as_slice(), count, &pagination)
        }
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

    let account_id = match authorize_session(&req, Some(&origin), None) {
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

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
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

    let account_id = match authorize_session(&req, None, None) {
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

    let _ = match authorize_session(&req, None, None) {
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

    let _ = match authorize_session(&req, None, None) {
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

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
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
fn get_origin_member_role(req: HttpRequest,
                          path: Path<(String, String)>,
                          state: Data<AppState>)
                          -> HttpResponse {
    let (origin, username) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Member)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // The account id of the user being requested
    let (target_user_id, _) = match Account::get(&username, &*conn).map_err(Error::DieselError) {
        Ok(account) => (account.id, account.name),
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    match OriginMember::member_role(&origin, target_user_id, &*conn) {
        Ok(role) => {
            let body = role_results_json(role);

            HttpResponse::Ok().header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                              .header(http::header::CACHE_CONTROL,
                                      headers::Cache::NoCache.to_string())
                              .body(body)
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn update_origin_member_role(req: HttpRequest,
                             path: Path<(String, String)>,
                             req_role: Query<Role>,
                             state: Data<AppState>)
                             -> HttpResponse {
    let (origin, username) = path.into_inner();
    let target_role = match OriginMemberRole::from_str(&req_role.role) {
        Ok(r) => {
            debug!("role {}", r);
            r
        }
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    // Account id of the user making the request
    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // We cannot allow a user to escalate to Owner. That must be done via Origin owner transfer.
    if target_role == OriginMemberRole::Owner {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // The account id of the user being requested
    let (target_user_id, _) = match Account::get(&username, &*conn) {
        Ok(account) => (account.id, account.name),
        Err(err) => {
            debug!("{}", err);
            return Error::DieselError(err).into();
        }
    };

    // We cannot allow a user to change their own role
    if account_id as i64 == target_user_id {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // We cannot allow a user to change the role of the origin owner
    if check_origin_owner(&req, target_user_id as u64, &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    state.memcache
         .borrow_mut()
         .clear_cache_for_member_role(&origin, target_user_id as u64);

    match OriginMember::update_member_role(&origin, target_user_id as i64, &*conn, target_role) {
        Ok(0) => HttpResponse::NotFound().into(),
        Ok(_) => HttpResponse::NoContent().into(),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn transfer_origin_ownership(req: HttpRequest,
                             path: Path<(String, String)>,
                             state: Data<AppState>)
                             -> HttpResponse {
    let (origin, user) = path.into_inner();

    let session = match authorize_session(&req, None, None) {
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
        Ok(_) => {
            origin_audit(&origin,
                         OriginOperation::OwnerTransfer,
                         &recipient_id.to_string(),
                         session.get_id() as i64,
                         session.get_name(),
                         &*conn);
            HttpResponse::NoContent().finish()
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn depart_from_origin(req: HttpRequest, path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    let session = match authorize_session(&req, None, None) {
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

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
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

    let session =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
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

    let (target_account_id, _target_account_name) =
        match Account::get(&user, &*conn).map_err(Error::DieselError) {
            Ok(account) => (account.id, account.name),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    match OriginMember::delete(&origin, &user, &*conn).map_err(Error::DieselError) {
        Ok(_) => {
            state.memcache
                 .borrow_mut()
                 .clear_cache_for_member_role(&origin, target_account_id as u64);
            HttpResponse::NoContent().finish()
        }
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

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
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

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
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

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (encrypted, _) =
        match crypto::encrypt(&state.config.api.key_path, &body).map_err(Error::BuilderCore) {
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

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
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

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::get(&origin, &integration, &name, &*conn).map_err(Error::DieselError) {
        Ok(integration) => {
            match crypto::decrypt(&state.config.api.key_path, &integration.body).map_err(Error::BuilderCore) {
                Ok(decrypted) => {
                    let val = serde_json::from_slice(&decrypted).unwrap();
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
        .body(Bytes::copy_from_slice(content))
}

fn generate_origin_encryption_keys(origin: &str,
                                   session_id: u64,
                                   conn: &PgConnection)
                                   -> Result<OriginPublicEncryptionKey> {
    debug!("Generating encryption keys for {}", origin);
    let (public, secret) = generate_origin_encryption_key_pair(origin);

    let pk_body = public.to_key_string();
    let new_pk = NewOriginPublicEncryptionKey { owner_id:  session_id as i64,
                                                origin:    &origin,
                                                name:      public.named_revision().name(),
                                                full_name: &public.named_revision().to_string(),
                                                revision:  &public.named_revision().revision(),
                                                body:      pk_body.as_ref(), };

    let sk_body = secret.to_key_string();
    let new_sk = NewOriginPrivateEncryptionKey { owner_id:  session_id as i64,
                                                 origin:    &origin,
                                                 name:      secret.named_revision().name(),
                                                 full_name: &secret.named_revision().to_string(),
                                                 revision:  &secret.named_revision().revision(),
                                                 body:      sk_body.as_ref(), };

    OriginPrivateEncryptionKey::create(&new_sk, &*conn)?;
    Ok(OriginPublicEncryptionKey::create(&new_pk, &*conn)?)
}

fn save_public_origin_signing_key(account_id: u64,
                                  origin: &str,
                                  key: &PublicOriginSigningKey,
                                  conn: &PgConnection)
                                  -> Result<()> {
    // Note that this is *not* base64 encoded
    let key_body = key.to_key_string();

    let new_pk = NewOriginPublicSigningKey { owner_id: account_id as i64,
                                             origin,
                                             full_name: &key.named_revision().to_string(),
                                             name: key.named_revision().name(),
                                             revision: key.named_revision().revision(),
                                             body: key_body.as_ref() };

    OriginPublicSigningKey::create(&new_pk, conn)?;
    Ok(())
}

fn save_secret_origin_signing_key(account_id: u64,
                                  origin: &str,
                                  key_cache: &KeyCache,
                                  key: &SecretOriginSigningKey,
                                  conn: &PgConnection)
                                  -> Result<()> {
    // Here we want to encrypt the full contents of the secret signing
    // key (i.e., encrypt the full "file", not merely the
    // cryptographic material) using our Builder encryption key. The
    // resulting bytes are what need to be saved in the database.
    let (sk_encrypted, bldr_key_rev) = crypto::encrypt(key_cache, key.to_key_string())?;

    let new_sk = NewOriginPrivateSigningKey { owner_id: account_id as i64,
                                              origin,
                                              full_name: &key.named_revision().to_string(),
                                              name: key.named_revision().name(),
                                              revision: key.named_revision().revision(),
                                              body: sk_encrypted.as_ref(),
                                              encryption_key_rev: &bldr_key_rev };

    OriginPrivateSigningKey::create(&new_sk, &*conn)?;
    Ok(())
}
