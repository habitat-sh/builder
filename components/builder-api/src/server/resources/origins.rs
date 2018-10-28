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

use std::collections::HashMap;
use std::str::from_utf8;

use actix_web::http::header::{ContentDisposition, DispositionParam, DispositionType};
use actix_web::http::{self, Method, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Json, Path, Query};
use actix_web::{AsyncResponder, FutureResponse, HttpMessage};
use bytes::Bytes;
use diesel::pg::PgConnection;
use diesel::result::Error::NotFound;
use futures::future::Future;
use serde_json;

use bldr_core;
use hab_core::crypto::keys::{parse_key_str, parse_name_with_rev, PairType};
use hab_core::crypto::BoxKeyPair;
use hab_core::package::ident;
use hab_net::{ErrCode, NetOk};

use protocol::originsrv::*;

use db::models::keys::*;
use db::models::origin::{
    CreateOrigin as CreateOriginMod, Origin as OriginMod, UpdateOrigin as UpdateOriginMod,
};
use db::models::package::PackageVisibility;
use db::models::secrets::OriginSecret as OriginSecretMod;

use server::authorize::{authorize_session, check_origin_owner, get_session_user_name};
use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers::{self, Pagination};
use server::AppState;

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
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route("/depot/{origin}/pkgs", Method::GET, list_unique_packages)
            .route("/depot/origins/{origin}", Method::GET, get_origin)
            .route("/depot/origins/{origin}", Method::PUT, update_origin)
            .route("/depot/origins", Method::POST, create_origin)
            .route(
                "/depot/origins/{origin}/users",
                Method::GET,
                list_origin_members,
            ).route(
                "/depot/origins/{origin}/users/{user}",
                http::Method::DELETE,
                origin_member_delete,
            ).route(
                "/depot/origins/{origin}/invitations",
                Method::GET,
                list_origin_invitations,
            ).route(
                "/depot/origins/{origin}/users/{username}/invitations",
                Method::POST,
                invite_to_origin,
            ).route(
                "/depot/origins/{origin}/invitations/{invitation_id}",
                Method::PUT,
                accept_invitation,
            ).route(
                "/depot/origins/{origin}/invitations/{invitation_id}",
                Method::DELETE,
                rescind_invitation,
            ).route(
                "/depot/origins/{origin}/invitations/{invitation_id}/ignore",
                Method::PUT,
                ignore_invitation,
            ).route(
                "/depot/origins/{origin}/keys/latest",
                Method::GET,
                download_latest_origin_key,
            ).route("/depot/origins/{origin}/keys", Method::POST, create_keys)
            .route(
                "/depot/origins/{origin}/keys",
                Method::GET,
                list_origin_keys,
            ).route(
                "/depot/origins/{origin}/keys/{revision}",
                http::Method::POST,
                upload_origin_key,
            ).route(
                "/depot/origins/{origin}/keys/{revision}",
                http::Method::GET,
                download_origin_key,
            ).route(
                "/depot/origins/{origin}/secret",
                Method::GET,
                list_origin_secrets,
            ).route(
                "/depot/origins/{origin}/secret",
                Method::POST,
                create_origin_secret,
            ).route(
                "/depot/origins/{origin}/encryption_key",
                Method::GET,
                download_latest_origin_encryption_key,
            ).route(
                "/depot/origins/{origin}/integrations",
                Method::GET,
                fetch_origin_integrations,
            ).route(
                "/depot/origins/{origin}/secret/{secret}",
                Method::DELETE,
                delete_origin_secret,
            ).route(
                "/depot/origins/{origin}/secret_keys/latest",
                Method::GET,
                download_latest_origin_secret_key,
            ).route(
                "/depot/origins/{origin}/secret_keys/{revision}",
                Method::POST,
                upload_origin_secret_key,
            ).route(
                "/depot/origins/{origin}/integrations/{integration}/names",
                Method::GET,
                fetch_origin_integration_names,
            ).route(
                "/depot/origins/{origin}/integrations/{integration}/{name}",
                Method::GET,
                get_origin_integration,
            ).route(
                "/depot/origins/{origin}/integrations/{integration}/{name}",
                Method::DELETE,
                delete_origin_integration,
            ).route(
                "/depot/origins/{origin}/integrations/{integration}/{name}",
                Method::PUT,
                create_origin_integration_async,
            )
    }
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_origin(req: HttpRequest<AppState>) -> HttpResponse {
    let origin_name = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::InternalServerError().into(),
    };

    match OriginMod::get(&origin_name, &*conn) {
        Ok(origin) => HttpResponse::Ok()
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .json(origin),
        Err(NotFound) => HttpResponse::NotFound().into(),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

fn create_origin(
    (req, body): (HttpRequest<AppState>, Json<CreateOriginHandlerReq>),
) -> HttpResponse {
    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let account_name = get_session_user_name(&req, account_id);

    let dpv = match body.clone().default_package_visibility {
        Some(viz) => viz,
        None => PackageVisibility::Public,
    };

    if !ident::is_valid_origin_name(&body.name) {
        return HttpResponse::ExpectationFailed().into();
    }

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::InternalServerError().into(),
    };

    match OriginMod::create(
        CreateOriginMod {
            name: body.0.name,
            owner_id: account_id as i64,
            owner_name: account_name,
            default_package_visibility: dpv,
        },
        &*conn,
    ) {
        Ok(origin) => HttpResponse::Created().json(origin),
        Err(_e) => HttpResponse::InternalServerError().into(),
    }
}

fn update_origin(
    (req, body): (HttpRequest<AppState>, Json<UpdateOriginHandlerReq>),
) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::InternalServerError().into(),
    };

    let dpv = match body.0.default_package_visibility {
        Some(viz) => viz,
        None => PackageVisibility::Public,
    };

    match OriginMod::update(
        UpdateOriginMod {
            name: origin,
            default_package_visibility: dpv,
        },
        &*conn,
    ) {
        Ok(_) => HttpResponse::NoContent().into(),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

fn create_keys(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    match helpers::get_origin(&req, origin) {
        Ok(origin) => match helpers::generate_origin_keys(&req, account_id, origin) {
            Ok(_) => HttpResponse::Created().finish(),
            Err(err) => err.into(),
        },
        Err(err) => err.into(),
    }
}

fn list_origin_keys(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let mut request = OriginPublicSigningKeyListRequest::new();
    match helpers::get_origin(&req, &origin) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return err.into(),
    }
    match route_message::<OriginPublicSigningKeyListRequest, OriginPublicSigningKeyListResponse>(
        &req, &request,
    ) {
        Ok(list) => {
            let list: Vec<OriginKeyIdent> = list
                .get_keys()
                .iter()
                .map(|key| {
                    let mut ident = OriginKeyIdent::new();
                    ident.set_location(format!(
                        "/origins/{}/keys/{}",
                        &key.get_name(),
                        &key.get_revision()
                    ));
                    ident.set_origin(key.get_name().to_string());
                    ident.set_revision(key.get_revision().to_string());
                    ident
                }).collect();

            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(&list)
        }
        Err(err) => err.into(),
    }
}

fn upload_origin_key((req, body): (HttpRequest<AppState>, String)) -> HttpResponse {
    let (origin, revision) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let mut request = OriginPublicSigningKeyCreate::new();
    request.set_owner_id(account_id);
    request.set_revision(revision);

    match helpers::get_origin(&req, &origin) {
        Ok(mut origin) => {
            request.set_name(origin.take_name());
            request.set_origin_id(origin.get_id());
        }
        Err(err) => return err.into(),
    };

    match parse_key_str(&body) {
        Ok((PairType::Public, _, _)) => {
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

    request.set_body(body.into_bytes());
    request.set_owner_id(0);
    match route_message::<OriginPublicSigningKeyCreate, OriginPublicSigningKey>(&req, &request) {
        Ok(_) => HttpResponse::Created()
            .header(http::header::LOCATION, format!("{}", req.uri()))
            .body(format!(
                "/origins/{}/keys/{}",
                &origin,
                &request.get_revision()
            )),
        Err(err) => err.into(),
    }
}

fn download_origin_key(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, revision) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let mut request = OriginPublicSigningKeyGet::new();
    request.set_origin(origin);
    request.set_revision(revision);

    let key =
        match route_message::<OriginPublicSigningKeyGet, OriginPublicSigningKey>(&req, &request) {
            Ok(key) => key,
            Err(err) => return err.into(),
        };

    let xfilename = format!("{}-{}.pub", key.get_name(), key.get_revision());
    download_content_as_file(key.get_body(), xfilename)
}

fn download_latest_origin_key(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let mut request = OriginPublicSigningKeyLatestGet::new();
    request.set_origin(origin);

    let key = match route_message::<OriginPublicSigningKeyLatestGet, OriginPublicSigningKey>(
        &req, &request,
    ) {
        Ok(key) => key,
        Err(err) => return err.into(),
    };

    let xfilename = format!("{}-{}.pub", key.get_name(), key.get_revision());
    download_content_as_file(key.get_body(), xfilename)
}

fn list_origin_secrets(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let origin_id = match OriginMod::get(&origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => origin.id,
        Err(err) => return err.into(),
    };

    match OriginSecretMod::list(origin_id as i64, &*conn).map_err(Error::DieselError) {
        Ok(list) => HttpResponse::Ok()
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .json(&list),
        Err(err) => err.into(),
    }
}

fn create_origin_secret(
    (req, body): (HttpRequest<AppState>, Json<OriginSecretPayload>),
) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let _account_id = match authorize_session(&req, Some(&origin)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    if body.name.len() <= 0 {
        return HttpResponse::with_body(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Missing value for field `name`",
        );
    }

    if body.value.len() <= 0 {
        return HttpResponse::with_body(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Missing value for field `value`",
        );
    }

    // get metadata from secret payload
    let secret_metadata = match BoxKeyPair::secret_metadata(body.value.as_bytes()) {
        Ok(res) => res,
        Err(e) => {
            return HttpResponse::with_body(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("Failed to get metadata from payload: {}", e),
            );
        }
    };

    debug!("Secret Metadata: {:?}", secret_metadata);

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let origin_id = match OriginMod::get(&origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => origin.id,
        Err(err) => return err.into(),
    };

    // fetch the private origin encryption key from the database
    let priv_key = match PrivateEncryptionKey::get(&origin, &*conn).map_err(Error::DieselError) {
        Ok(key) => {
            let key_str = from_utf8(&key.body).unwrap();
            match BoxKeyPair::secret_key_from_str(key_str) {
                Ok(key) => key,
                Err(e) => {
                    return HttpResponse::with_body(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        format!("Failed to get secret from payload: {}", e),
                    );
                }
            }
        }
        Err(err) => return err.into(),
    };

    let (name, rev) = match parse_name_with_rev(secret_metadata.sender) {
        Ok(val) => val,
        Err(e) => {
            return HttpResponse::with_body(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("Failed to parse name and revision: {}", e),
            );
        }
    };

    debug!("Using key {:?}-{:?}", name, &rev);

    // fetch the public origin encryption key from the database
    let pub_key = match PublicEncryptionKey::get(&origin, &rev, &*conn).map_err(Error::DieselError)
    {
        Ok(key) => {
            let key_str = from_utf8(&key.body).unwrap();
            match BoxKeyPair::public_key_from_str(key_str) {
                Ok(key) => key,
                Err(e) => {
                    return HttpResponse::with_body(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        format!("{}", e),
                    );
                }
            }
        }
        Err(err) => return err.into(),
    };

    let box_key_pair = BoxKeyPair::new(name, rev.clone(), Some(pub_key), Some(priv_key));

    debug!("Decrypting string: {:?}", &secret_metadata.ciphertext);

    // verify we can decrypt the message
    match box_key_pair.decrypt(&secret_metadata.ciphertext, None, None) {
        Ok(_) => (),
        Err(e) => {
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, format!("{}", e));
        }
    };

    match OriginSecretMod::create(origin_id as i64, &body.name, &body.value, &*conn)
        .map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => err.into(),
    }
}

fn delete_origin_secret(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, secret) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let origin_id = match OriginMod::get(&origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => origin.id,
        Err(err) => return err.into(),
    };

    let mut request = OriginSecretDelete::new();
    match helpers::get_origin(&req, &origin) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return err.into(),
    }
    request.set_name(secret.clone());
    match OriginSecretMod::delete(origin_id as i64, &secret, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => err.into(),
    }
}

fn upload_origin_secret_key(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    req.body()
        .from_err()
        .and_then(move |bytes: Bytes| Ok(do_upload_origin_secret_key(&req, &bytes)))
        .responder()
}

fn do_upload_origin_secret_key(req: &HttpRequest<AppState>, body: &Bytes) -> HttpResponse {
    let (origin, revision) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

    let account_id = match authorize_session(req, Some(&origin)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let mut request = OriginPrivateSigningKeyCreate::new();
    request.set_owner_id(account_id);
    request.set_revision(revision);

    match helpers::get_origin(req, &origin) {
        Ok(mut origin) => {
            request.set_name(origin.take_name());
            request.set_origin_id(origin.get_id());
        }
        Err(err) => return err.into(),
    }

    match String::from_utf8(body.to_vec()) {
        Ok(content) => match parse_key_str(&content) {
            Ok((PairType::Secret, _, _)) => {
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
        },
        Err(e) => {
            debug!("Can't parse secret key upload content: {}", e);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    request.set_body(body.to_vec());
    request.set_owner_id(0);
    match route_message::<OriginPrivateSigningKeyCreate, OriginPrivateSigningKey>(req, &request) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => err.into(),
    }
}

fn download_latest_origin_secret_key(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let mut request = OriginPrivateSigningKeyGet::new();
    match helpers::get_origin(&req, origin) {
        Ok(mut origin) => {
            request.set_owner_id(origin.get_owner_id());
            request.set_origin(origin.take_name());
        }
        Err(err) => return err.into(),
    }
    let key = match route_message::<OriginPrivateSigningKeyGet, OriginPrivateSigningKey>(
        &req, &request,
    ) {
        Ok(key) => key,
        Err(err) => return err.into(),
    };

    let xfilename = format!("{}-{}.sig.key", key.get_name(), key.get_revision());
    download_content_as_file(key.get_body(), xfilename)
}

fn list_unique_packages(
    (req, pagination): (HttpRequest<AppState>, Query<Pagination>),
) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let opt_session_id = match authorize_session(&req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let mut request = OriginPackageUniqueListRequest::new();
    let (start, stop) = helpers::extract_pagination(&pagination);
    request.set_start(start as u64);
    request.set_stop(stop as u64);
    request.set_visibilities(helpers::visibility_for_optional_session(
        &req,
        opt_session_id,
        &origin,
    ));
    request.set_origin(origin);

    match route_message::<OriginPackageUniqueListRequest, OriginPackageUniqueListResponse>(
        &req, &request,
    ) {
        Ok(packages) => {
            debug!(
                "list_unique_packages start: {}, stop: {}, total count: {}",
                packages.get_start(),
                packages.get_stop(),
                packages.get_count()
            );
            let body = helpers::package_results_json(
                &packages.get_idents().to_vec(),
                packages.get_count() as isize,
                packages.get_start() as isize,
                packages.get_stop() as isize,
            );

            let mut response = if packages.get_count() as isize > (packages.get_stop() as isize + 1)
            {
                HttpResponse::PartialContent()
            } else {
                HttpResponse::Ok()
            };

            response
                .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .body(body)
        }
        Err(err) => return err.into(),
    }
}

fn download_latest_origin_encryption_key(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match PublicEncryptionKey::latest(&origin, &*conn) {
        Ok(key) => key,
        Err(NotFound) => {
            // TODO: redesign to not be generating keys during d/l
            match generate_origin_encryption_keys(&origin, account_id, &conn) {
                Ok(key) => key,
                Err(Error::NetError(e)) => return Error::NetError(e).into(),
                Err(_) => unreachable!(),
            }
        }
        Err(err) => return Error::DieselError(err).into(),
    };

    let xfilename = format!("{}-{}.pub", key.name, key.revision);
    download_content_as_file(&key.body, xfilename)
}

fn invite_to_origin(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, user) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    debug!("Creating invitation for user {} origin {}", &user, &origin);

    let mut request = AccountGet::new();
    let mut invite_request = OriginInvitationCreate::new();
    request.set_name(user.to_string());

    match route_message::<AccountGet, Account>(&req, &request) {
        Ok(mut account) => {
            invite_request.set_account_id(account.get_id());
            invite_request.set_account_name(account.take_name());
        }
        Err(err) => return err.into(),
    };

    match helpers::get_origin(&req, &origin) {
        Ok(mut origin) => {
            invite_request.set_origin_id(origin.get_id());
            invite_request.set_origin_name(origin.take_name());
        }
        Err(err) => return err.into(),
    }

    invite_request.set_owner_id(account_id);

    // store invitations in the originsrv
    match route_message::<OriginInvitationCreate, OriginInvitation>(&req, &invite_request) {
        Ok(invitation) => HttpResponse::Created().json(&invitation),
        Err(Error::NetError(err)) => {
            if err.get_code() == ErrCode::ENTITY_CONFLICT {
                HttpResponse::NoContent().finish()
            } else {
                Error::NetError(err).into()
            }
        }
        Err(err) => err.into(),
    }
}

fn accept_invitation(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, invitation) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let mut request = OriginInvitationAcceptRequest::new();
    request.set_ignore(false);
    request.set_account_id(account_id);
    request.set_origin_name(origin);

    match invitation.parse::<u64>() {
        Ok(invitation_id) => request.set_invite_id(invitation_id),
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
    }

    debug!(
        "Accepting invitation for user {} origin {}",
        &request.get_account_id(),
        request.get_origin_name()
    );

    match route_message::<OriginInvitationAcceptRequest, NetOk>(&req, &request) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn ignore_invitation(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, invitation) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let mut request = OriginInvitationIgnoreRequest::new();
    request.set_account_id(account_id);

    match invitation.parse::<u64>() {
        Ok(invitation_id) => request.set_invitation_id(invitation_id),
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
    }

    debug!(
        "Ignoring invitation id {} for user {} origin {}",
        request.get_invitation_id(),
        request.get_account_id(),
        &origin
    );

    match route_message::<OriginInvitationIgnoreRequest, NetOk>(&req, &request) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn rescind_invitation(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, invitation) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, None) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let mut request = OriginInvitationRescindRequest::new();
    request.set_owner_id(account_id);

    match invitation.parse::<u64>() {
        Ok(invitation_id) => request.set_invitation_id(invitation_id),
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
    }

    debug!(
        "Rescinding invitation id {} for user {} origin {}",
        request.get_invitation_id(),
        request.get_owner_id(),
        &origin
    );

    match route_message::<OriginInvitationRescindRequest, NetOk>(&req, &request) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn list_origin_invitations(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let mut request = OriginInvitationListRequest::new();
    match helpers::get_origin(&req, &origin) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return err.into(),
    }

    match route_message::<OriginInvitationListRequest, OriginInvitationListResponse>(&req, &request)
    {
        Ok(list) => HttpResponse::Ok()
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .json(list),
        Err(err) => err.into(),
    }
}

fn list_origin_members(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let mut request = OriginMemberListRequest::new();
    match helpers::get_origin(&req, &origin) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return err.into(),
    }
    match route_message::<OriginMemberListRequest, OriginMemberListResponse>(&req, &request) {
        Ok(list) => HttpResponse::Ok()
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .json(list),
        Err(err) => err.into(),
    }
}

fn origin_member_delete(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, user) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let account_name = get_session_user_name(&req, account_id);

    if !check_origin_owner(&req, account_id, &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // Do not allow the owner to be removed which would orphan the origin
    if user == account_name {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    debug!(
        "Deleting user name {} for user {} origin {}",
        &user, &account_id, &origin
    );

    let mut origin_request = OriginMemberRemove::new();

    match helpers::get_origin(&req, origin) {
        Ok(origin) => {
            origin_request.set_origin_id(origin.get_id());
        }
        Err(err) => return err.into(),
    }
    origin_request.set_account_name(user.to_string());

    match route_message::<OriginMemberRemove, NetOk>(&req, &origin_request) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn fetch_origin_integrations(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let mut request = OriginIntegrationRequest::new();
    request.set_origin(origin);
    match route_message::<OriginIntegrationRequest, OriginIntegrationResponse>(&req, &request) {
        Ok(oir) => {
            let integrations_response: HashMap<String, Vec<String>> = oir
                .get_integrations()
                .iter()
                .fold(HashMap::new(), |mut acc, ref i| {
                    acc.entry(i.get_integration().to_owned())
                        .or_insert(Vec::new())
                        .push(i.get_name().to_owned());
                    acc
                });
            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(integrations_response)
        }
        Err(err) => err.into(),
    }
}

fn fetch_origin_integration_names(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, integration) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let mut request = OriginIntegrationGetNames::new();
    request.set_origin(origin);
    request.set_integration(integration);
    match route_message::<OriginIntegrationGetNames, OriginIntegrationNames>(&req, &request) {
        Ok(integration) => HttpResponse::Ok()
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .json(integration),
        Err(err) => err.into(),
    }
}

fn create_origin_integration_async(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    req.body()
        .from_err()
        .and_then(move |bytes: Bytes| Ok(create_origin_integration(req, &bytes)))
        .responder()
}

fn create_origin_integration(req: HttpRequest<AppState>, body: &Bytes) -> HttpResponse {
    let (origin, integration, name) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let mut oi = OriginIntegration::new();
    oi.set_origin(origin);
    oi.set_integration(integration);
    oi.set_name(name);

    match encrypt(&req, &body) {
        Ok(encrypted) => oi.set_body(encrypted),
        Err(err) => return err.into(),
    }

    let mut request = OriginIntegrationCreate::new();
    request.set_integration(oi);

    match route_message::<OriginIntegrationCreate, NetOk>(&req, &request) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => err.into(),
    }
}

fn delete_origin_integration(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, integration, name) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let mut oi = OriginIntegration::new();
    oi.set_origin(origin);
    oi.set_integration(integration);
    oi.set_name(name);

    let mut request = OriginIntegrationDelete::new();
    request.set_integration(oi);

    match route_message::<OriginIntegrationDelete, NetOk>(&req, &request) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn get_origin_integration(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, integration, name) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let mut oi = OriginIntegration::new();
    oi.set_origin(origin);
    oi.set_integration(integration);
    oi.set_name(name);

    let mut request = OriginIntegrationGet::new();
    request.set_integration(oi);

    match route_message::<OriginIntegrationGet, OriginIntegration>(&req, &request) {
        Ok(integration) => match decrypt(&req, integration.get_body()) {
            Ok(decrypted) => {
                let val = serde_json::from_str(&decrypted).unwrap();
                let mut map: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_value(val).unwrap();

                map.remove("password");

                let sanitized = json!({
                        "origin": integration.get_origin().to_string(),
                        "integration": integration.get_integration().to_string(),
                        "name": integration.get_name().to_string(),
                        "body": serde_json::to_value(map).unwrap()
                    });

                HttpResponse::Ok()
                    .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                    .json(sanitized)
            }
            Err(err) => err.into(),
        },
        Err(err) => err.into(),
    }
}

//
// Internal helpers
//

fn download_content_as_file(content: &[u8], filename: String) -> HttpResponse {
    HttpResponse::Ok()
        .header(
            http::header::CONTENT_DISPOSITION,
            ContentDisposition {
                disposition: DispositionType::Attachment,
                parameters: vec![DispositionParam::Filename(filename.clone())],
            },
        ).header(
            http::header::HeaderName::from_static(headers::XFILENAME),
            filename,
        ).header(http::header::CACHE_CONTROL, headers::NO_CACHE)
        .body(Bytes::from(content))
}

fn generate_origin_encryption_keys(
    origin: &str,
    session_id: u64,
    conn: &PgConnection,
) -> Result<PublicEncryptionKey> {
    debug!("Generating encryption keys for {}", origin);

    let origin_id = match OriginMod::get(origin, &*conn) {
        Ok(origin) => origin.id,
        Err(err) => return Err(Error::DieselError(err)),
    };

    let pair = BoxKeyPair::generate_pair_for_origin(origin).map_err(Error::HabitatCore)?;

    let pk_body = pair
        .to_public_string()
        .map_err(Error::HabitatCore)?
        .into_bytes();

    let new_pk = NewPublicEncryptionKey {
        owner_id: session_id as i64,
        origin_id: origin_id,
        name: origin,
        revision: &pair.rev,
        body: &pk_body,
    };

    let sk_body = pair
        .to_secret_string()
        .map_err(Error::HabitatCore)?
        .into_bytes();

    let new_sk = NewPrivateEncryptionKey {
        owner_id: session_id as i64,
        origin_id: origin_id,
        name: origin,
        revision: &pair.rev,
        body: &sk_body,
    };

    PrivateEncryptionKey::create(&new_sk, &*conn)?;
    PublicEncryptionKey::create(&new_pk, &*conn).map_err(Error::DieselError)
}

fn encrypt(req: &HttpRequest<AppState>, content: &Bytes) -> Result<String> {
    bldr_core::integrations::encrypt(&req.state().config.api.key_path, content)
        .map_err(Error::BuilderCore)
}

fn decrypt(req: &HttpRequest<AppState>, content: &str) -> Result<String> {
    let bytes = bldr_core::integrations::decrypt(&req.state().config.api.key_path, content)?;
    Ok(String::from_utf8(bytes)?)
}
