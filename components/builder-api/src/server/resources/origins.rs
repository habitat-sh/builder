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

use actix_web::http::header::{Charset, ContentDisposition, DispositionParam, DispositionType};
use actix_web::http::{self, StatusCode};
use actix_web::FromRequest;
use actix_web::{pred, App, HttpRequest, HttpResponse, Json, Path, Query};
use actix_web::{AsyncResponder, FutureResponse, HttpMessage};
use bytes::Bytes;
use futures::future::Future;
use serde_json;

use bldr_core;
use hab_core::crypto::keys::{parse_key_str, parse_name_with_rev, PairType};
use hab_core::crypto::BoxKeyPair;
use hab_core::package::ident;
use hab_net::{ErrCode, NetOk};

use protocol::originsrv::*;
use protocol::sessionsrv::*;

use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::{route_message, Authenticated, Optional};
use server::helpers::{self, Pagination};
use server::AppState;

// Query param containers
#[derive(Clone, Serialize, Deserialize)]
struct OriginCreateReq {
    #[serde(default)]
    name: String,
    #[serde(default)]
    default_package_visibility: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct OriginUpdateReq {
    #[serde(default)]
    default_package_visibility: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct OriginSecretPayload {
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: String,
}

pub struct Origins {}

impl Origins {
    //
    // Internal - these functions should return Result<..>
    //
    fn do_get_origin(req: &HttpRequest<AppState>, origin: String) -> Result<Origin> {
        let mut request = OriginGet::new();
        request.set_name(origin);

        route_message::<OriginGet, Origin>(req, &request)
    }

    fn do_create_origin(
        req: &HttpRequest<AppState>,
        body: &Json<OriginCreateReq>,
    ) -> Result<Origin> {
        let mut request = OriginCreate::new();

        let (account_id, account_name) = helpers::get_session_id_and_name(req);
        request.set_owner_id(account_id);
        request.set_owner_name(account_name);
        request.set_name(body.name.clone());

        if let Some(ref vis) = body.default_package_visibility {
            let opv = vis.parse::<OriginPackageVisibility>()?;
            request.set_default_package_visibility(opv);
        }

        route_message::<OriginCreate, Origin>(&req, &request)
    }

    fn do_create_keys(req: &HttpRequest<AppState>, origin: String) -> Result<()> {
        let account_id = helpers::check_origin_access(req, &origin)?;

        match helpers::get_origin(req, origin) {
            Ok(origin) => helpers::generate_origin_keys(req, account_id, origin),
            Err(err) => Err(err),
        }
    }

    //
    // Route handlers - these functions should return HttpResponse
    //
    fn get_origin(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        match Self::do_get_origin(req, origin) {
            Ok(origin) => HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(origin),
            Err(err) => err.into(),
        }
    }

    fn create_origin((req, body): (HttpRequest<AppState>, Json<OriginCreateReq>)) -> HttpResponse {
        if !ident::is_valid_origin_name(&body.name) {
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }

        match Self::do_create_origin(&req, &body) {
            Ok(origin) => HttpResponse::Created().json(origin),
            Err(err) => err.into(),
        }
    }

    fn update_origin((req, body): (HttpRequest<AppState>, Json<OriginUpdateReq>)) -> HttpResponse {
        let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginUpdate::new();
        request.set_name(origin.clone());

        let dpv = match body
            .default_package_visibility
            .parse::<OriginPackageVisibility>()
        {
            Ok(x) => x,
            Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
        };
        request.set_default_package_visibility(dpv);

        match helpers::get_origin(&req, request.get_name()) {
            Ok(origin) => request.set_id(origin.get_id()),
            Err(err) => return err.into(),
        }

        match route_message::<OriginUpdate, NetOk>(&req, &request) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    fn create_keys(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        match Self::do_create_keys(req, origin) {
            Ok(_) => HttpResponse::Created().finish(),
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
                    })
                    .collect();

                //let body = serde_json::to_string(&list).unwrap();
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
        let account_id = match helpers::check_origin_access(&req, &origin) {
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
        match route_message::<OriginPublicSigningKeyCreate, OriginPublicSigningKey>(&req, &request)
        {
            Ok(_) => {
                // TODO ?
                //let mut base_url: url::Url = req.url.clone().into();
                //base_url.set_path(&format!("key/{}-{}", &origin, &request.get_revision()));

                HttpResponse::Created().body(format!(
                    "/origins/{}/keys/{}",
                    &origin,
                    &request.get_revision()
                ))
                // .header(http::header::LOCATION, format!("{}", base_url))
            }
            Err(err) => err.into(),
        }
    }

    fn download_origin_key(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, revision) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

        let mut request = OriginPublicSigningKeyGet::new();
        request.set_origin(origin);
        request.set_revision(revision);

        let key = match route_message::<OriginPublicSigningKeyGet, OriginPublicSigningKey>(
            req, &request,
        ) {
            Ok(key) => key,
            Err(err) => return err.into(),
        };

        let xfilename = format!("{}-{}.pub", key.get_name(), key.get_revision());
        download_content_as_file(key.get_body(), xfilename)
    }

    fn download_latest_origin_key(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        let mut request = OriginPublicSigningKeyLatestGet::new();
        request.set_origin(origin);

        let key = match route_message::<OriginPublicSigningKeyLatestGet, OriginPublicSigningKey>(
            req, &request,
        ) {
            Ok(key) => key,
            Err(err) => return err.into(),
        };

        let xfilename = format!("{}-{}.pub", key.get_name(), key.get_revision());
        download_content_as_file(key.get_body(), xfilename)
    }

    fn list_origin_secrets(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginSecretListGet::new();
        match helpers::get_origin(req, &origin) {
            Ok(origin) => request.set_origin_id(origin.get_id()),
            Err(err) => return err.into(),
        }

        match route_message::<OriginSecretListGet, OriginSecretList>(req, &request) {
            Ok(list) => HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(&list.get_secrets()),
            Err(err) => err.into(),
        }
    }

    fn create_origin_secret(
        (req, body): (HttpRequest<AppState>, Json<OriginSecretPayload>),
    ) -> HttpResponse {
        let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        // get metadata from secret payload
        let secret_metadata = match BoxKeyPair::secret_metadata(body.value.as_bytes()) {
            Ok(res) => res,
            Err(e) => {
                debug!("Failed to get metadata from payload: {}", e);
                return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
            }
        };

        debug!("Secret Metadata: {:?}", secret_metadata);

        let mut secret = OriginSecret::new();
        secret.set_name(body.name.clone());
        secret.set_value(body.value.clone());
        let mut db_priv_request = OriginPrivateEncryptionKeyGet::new();
        let mut db_pub_request = OriginPublicEncryptionKeyGet::new();
        match helpers::get_origin(&req, origin) {
            Ok(mut origin) => {
                let origin_name = origin.take_name();
                let origin_owner_id = origin.get_owner_id();
                secret.set_origin_id(origin.get_id());
                db_priv_request.set_owner_id(origin_owner_id.clone());
                db_priv_request.set_origin(origin_name.clone());
                db_pub_request.set_owner_id(origin_owner_id.clone());
                db_pub_request.set_origin(origin_name.clone());
            }
            Err(err) => return err.into(),
        }

        // fetch the private origin encryption key from the database
        let priv_key = match route_message::<
            OriginPrivateEncryptionKeyGet,
            OriginPrivateEncryptionKey,
        >(&req, &db_priv_request)
        {
            Ok(key) => {
                let key_str = from_utf8(key.get_body()).unwrap();
                match BoxKeyPair::secret_key_from_str(key_str) {
                    Ok(key) => key,
                    Err(e) => {
                        debug!("Failed to get secret from payload: {}", e);
                        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                    }
                }
            }
            Err(err) => return err.into(),
        };

        let (name, rev) = match parse_name_with_rev(secret_metadata.sender) {
            Ok(val) => val,
            Err(e) => {
                debug!("Failed to parse name and revision: {}", e);
                return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
            }
        };

        db_pub_request.set_revision(rev.clone());

        debug!("Using key {:?}-{:?}", name, &rev);

        // fetch the public origin encryption key from the database
        let pub_key = match route_message::<OriginPublicEncryptionKeyGet, OriginPublicEncryptionKey>(
            &req,
            &db_pub_request,
        ) {
            Ok(key) => {
                let key_str = from_utf8(key.get_body()).unwrap();
                match BoxKeyPair::public_key_from_str(key_str) {
                    Ok(key) => key,
                    Err(e) => {
                        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
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
                return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
            }
        };

        let mut request = OriginSecretCreate::new();
        request.set_secret(secret);

        match route_message::<OriginSecretCreate, OriginSecret>(&req, &request) {
            Ok(_) => HttpResponse::Created().finish(),
            Err(err) => err.into(),
        }
    }

    fn delete_origin_secret(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, secret) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginSecretDelete::new();
        match helpers::get_origin(req, &origin) {
            Ok(origin) => request.set_origin_id(origin.get_id()),
            Err(err) => return err.into(),
        }
        request.set_name(secret.clone());
        match route_message::<OriginSecretDelete, NetOk>(req, &request) {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(err) => err.into(),
        }
    }

    fn upload_origin_secret_key_async(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
        req.body()
            .from_err()
            .and_then(move |bytes: Bytes| Ok(Self::upload_origin_secret_key(&req, &bytes)))
            .responder()
    }

    fn upload_origin_secret_key(req: &HttpRequest<AppState>, body: &Bytes) -> HttpResponse {
        let (origin, revision) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

        let account_id = match helpers::check_origin_access(&req, &origin) {
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
        match route_message::<OriginPrivateSigningKeyCreate, OriginPrivateSigningKey>(req, &request)
        {
            Ok(_) => HttpResponse::Created().finish(),
            Err(err) => err.into(),
        }
    }

    fn download_latest_origin_secret_key(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginPrivateSigningKeyGet::new();
        match helpers::get_origin(req, origin) {
            Ok(mut origin) => {
                request.set_owner_id(origin.get_owner_id());
                request.set_origin(origin.take_name());
            }
            Err(err) => return err.into(),
        }
        let key = match route_message::<OriginPrivateSigningKeyGet, OriginPrivateSigningKey>(
            req, &request,
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
        let session_id = helpers::get_optional_session_id(&req);

        let mut request = OriginPackageUniqueListRequest::new();
        let (start, stop) = helpers::extract_pagination(&pagination);
        request.set_start(start as u64);
        request.set_stop(stop as u64);
        request.set_visibilities(helpers::visibility_for_optional_session(
            &req, session_id, &origin,
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

                let mut response =
                    if packages.get_count() as isize > (packages.get_stop() as isize + 1) {
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

    fn download_latest_origin_encryption_key(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginPublicEncryptionKeyLatestGet::new();
        let origin = match helpers::get_origin(req, &origin) {
            Ok(mut origin) => {
                request.set_owner_id(origin.get_owner_id());
                request.set_origin(origin.get_name().to_string());
                origin
            }
            Err(err) => return err.into(),
        };

        let key = match route_message::<OriginPublicEncryptionKeyLatestGet, OriginPublicEncryptionKey>(
            req, &request,
        ) {
            Ok(key) => key,
            Err(Error::NetError(err)) => {
                // TODO: redesign to not be generating keys during d/l
                if err.get_code() == ErrCode::ENTITY_NOT_FOUND {
                    match generate_origin_encryption_keys(&origin, req) {
                        Ok(key) => key,
                        Err(Error::NetError(e)) => return Error::NetError(e).into(),
                        Err(_) => unreachable!(),
                    }
                } else {
                    return Error::NetError(err).into();
                }
            }
            Err(err) => return err.into(),
        };

        let xfilename = format!("{}-{}.pub", key.get_name(), key.get_revision());
        download_content_as_file(key.get_body(), xfilename)
    }

    fn invite_to_origin(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, user) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

        let account_id = match helpers::check_origin_access(&req, &origin) {
            Ok(id) => id,
            Err(err) => return err.into(),
        };

        debug!("Creating invitation for user {} origin {}", &user, &origin);

        let mut request = AccountGet::new();
        let mut invite_request = OriginInvitationCreate::new();
        request.set_name(user.to_string());

        match route_message::<AccountGet, Account>(req, &request) {
            Ok(mut account) => {
                invite_request.set_account_id(account.get_id());
                invite_request.set_account_name(account.take_name());
            }
            Err(err) => return err.into(),
        };

        match helpers::get_origin(req, &origin) {
            Ok(mut origin) => {
                invite_request.set_origin_id(origin.get_id());
                invite_request.set_origin_name(origin.take_name());
            }
            Err(err) => return err.into(),
        }

        invite_request.set_owner_id(account_id);

        // store invitations in the originsrv
        match route_message::<OriginInvitationCreate, OriginInvitation>(req, &invite_request) {
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

    fn accept_invitation(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, invitation) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

        let account_id = match helpers::check_origin_access(&req, &origin) {
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

        match route_message::<OriginInvitationAcceptRequest, NetOk>(req, &request) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    fn ignore_invitation(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, invitation) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

        let account_id = match helpers::check_origin_access(&req, &origin) {
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

        match route_message::<OriginInvitationIgnoreRequest, NetOk>(req, &request) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    fn rescind_invitation(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, invitation) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

        let account_id = match helpers::check_origin_access(&req, &origin) {
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

        match route_message::<OriginInvitationRescindRequest, NetOk>(req, &request) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    fn list_origin_invitations(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginInvitationListRequest::new();
        match helpers::get_origin(req, &origin) {
            Ok(origin) => request.set_origin_id(origin.get_id()),
            Err(err) => return err.into(),
        }

        match route_message::<OriginInvitationListRequest, OriginInvitationListResponse>(
            req, &request,
        ) {
            Ok(list) => HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(list),
            Err(err) => err.into(),
        }
    }

    fn list_origin_members(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginMemberListRequest::new();
        match helpers::get_origin(req, &origin) {
            Ok(origin) => request.set_origin_id(origin.get_id()),
            Err(err) => return err.into(),
        }
        match route_message::<OriginMemberListRequest, OriginMemberListResponse>(req, &request) {
            Ok(list) => HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(list),
            Err(err) => err.into(),
        }
    }

    fn origin_member_delete(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, user) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok
        let (account_id, account_name) = helpers::get_session_id_and_name(req);

        if !helpers::check_origin_owner(req, account_id, &origin).unwrap_or(false) {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        // Do not allow the owner to be removed which would orphan the origin
        if user == account_name {
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }

        debug!(
            "Deleting user name {} for user {} origin {}",
            &user, &account_id, &origin
        );

        let mut session_request = AccountOriginRemove::new();
        let mut origin_request = OriginMemberRemove::new();

        match helpers::get_origin(req, origin) {
            Ok(origin) => {
                session_request.set_origin_id(origin.get_id());
                origin_request.set_origin_id(origin.get_id());
            }
            Err(err) => return err.into(),
        }
        session_request.set_account_name(user.to_string());
        origin_request.set_account_name(user.to_string());

        if let Err(err) = route_message::<AccountOriginRemove, NetOk>(req, &session_request) {
            return err.into();
        }

        match route_message::<OriginMemberRemove, NetOk>(req, &origin_request) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    fn fetch_origin_integrations(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginIntegrationRequest::new();
        request.set_origin(origin);
        match route_message::<OriginIntegrationRequest, OriginIntegrationResponse>(req, &request) {
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

    fn fetch_origin_integration_names(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, integration) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginIntegrationGetNames::new();
        request.set_origin(origin);
        request.set_integration(integration);
        match route_message::<OriginIntegrationGetNames, OriginIntegrationNames>(req, &request) {
            Ok(integration) => HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(integration),
            Err(err) => err.into(),
        }
    }

    fn create_origin_integration_async(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
        req.body()
            .from_err()
            .and_then(move |bytes: Bytes| Ok(Self::create_origin_integration(&req, &bytes)))
            .responder()
    }

    fn create_origin_integration(req: &HttpRequest<AppState>, body: &Bytes) -> HttpResponse {
        let (origin, integration, name) = Path::<(String, String, String)>::extract(req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut oi = OriginIntegration::new();
        oi.set_origin(origin);
        oi.set_integration(integration);
        oi.set_name(name);

        match encrypt(req, &body) {
            Ok(encrypted) => oi.set_body(encrypted),
            Err(err) => return err.into(),
        }

        let mut request = OriginIntegrationCreate::new();
        request.set_integration(oi);

        match route_message::<OriginIntegrationCreate, NetOk>(req, &request) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    fn delete_origin_integration(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, integration, name) = Path::<(String, String, String)>::extract(req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut oi = OriginIntegration::new();
        oi.set_origin(origin);
        oi.set_integration(integration);
        oi.set_name(name);

        let mut request = OriginIntegrationDelete::new();
        request.set_integration(oi);

        match route_message::<OriginIntegrationDelete, NetOk>(req, &request) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    fn get_origin_integration(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, integration, name) = Path::<(String, String, String)>::extract(req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut oi = OriginIntegration::new();
        oi.set_origin(origin);
        oi.set_integration(integration);
        oi.set_name(name);

        let mut request = OriginIntegrationGet::new();
        request.set_integration(oi);

        match route_message::<OriginIntegrationGet, OriginIntegration>(req, &request) {
            Ok(integration) => match decrypt(req, integration.get_body()) {
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
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.resource("/depot/{origin}/pkgs", |r| {
            r.middleware(Optional);
            r.method(http::Method::GET).with(Self::list_unique_packages)
        }).resource("/depot/origins/{origin}", |r| {
                r.get().f(Origins::get_origin)
            })
            .resource("/depot/origins/{origin}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::PUT).with(Self::update_origin)
            })
            .resource("/depot/origins", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).with(Self::create_origin);
            })
            .resource("/depot/origins/{origin}/users", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::GET).f(Self::list_origin_members)
            })
            .resource("/depot/origins/{origin}/users/{user}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::DELETE).f(Self::origin_member_delete)
            })
            .resource("/depot/origins/{origin}/invitations", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::GET).f(Self::list_origin_invitations)
            })
            .resource(
                "/depot/origins/{origin}/users/{username}/invitations",
                |r| {
                    r.middleware(Authenticated);
                    r.method(http::Method::POST).f(Self::invite_to_origin);
                },
            )
            .resource("/depot/origins/{origin}/invitations/{invitation_id}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::PUT).f(Self::accept_invitation);
                r.method(http::Method::DELETE).f(Self::rescind_invitation);
            })
            .resource(
                "/depot/origins/{origin}/invitations/{invitation_id}/ignore",
                |r| {
                    r.middleware(Authenticated);
                    r.method(http::Method::PUT).f(Self::ignore_invitation);
                },
            )
            .resource("/depot/origins/{origin}/keys/latest", |r| {
                r.method(http::Method::GET)
                    .f(Self::download_latest_origin_key);
            })
            .resource("/depot/origins/{origin}/keys", |r| {
                r.middleware(Authenticated);
                r.route().filter(pred::Post()).f(Self::create_keys);
            })
            .route(
                "/depot/origins/{origin}/keys",
                http::Method::GET,
                Self::list_origin_keys,
            )
            .resource("/depot/origins/{origin}/keys/{revision}", |r| {
                r.method(http::Method::GET).f(Self::download_origin_key);
            })
            .resource("/depot/origins/{origin}/keys/{revision}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).with(Self::upload_origin_key);
            })
            .resource("/depot/origins/{origin}/secret", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::GET).f(Self::list_origin_secrets);
                r.method(http::Method::POST)
                    .with(Self::create_origin_secret);
            })
            .resource("/depot/origins/{origin}/encryption_key", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::GET)
                    .f(Self::download_latest_origin_encryption_key);
            })
            .resource("/depot/origins/{origin}/integrations", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::GET)
                    .f(Self::fetch_origin_integrations);
            })
            .resource("/depot/origins/{origin}/{secret}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::DELETE).f(Self::delete_origin_secret);
            })
            .resource("/depot/origins/{origin}/secret_keys/latest", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::GET)
                    .f(Self::download_latest_origin_secret_key);
            })
            .resource("/depot/origins/{origin}/secret_keys/{revision}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST)
                    .with(Self::upload_origin_secret_key_async);
            })
            .resource(
                "/depot/origins/{origin}/integrations/{integration}/names",
                |r| {
                    r.middleware(Authenticated);
                    r.method(http::Method::GET)
                        .f(Self::fetch_origin_integration_names);
                },
            )
            .resource(
                "/depot/origins/{origin}/integrations/{integration}/{name}",
                |r| {
                    r.middleware(Authenticated);
                    r.method(http::Method::GET).f(Self::get_origin_integration);
                    r.method(http::Method::DELETE)
                        .f(Self::delete_origin_integration);
                    r.method(http::Method::POST)
                        .with(Self::create_origin_integration_async);
                },
            )
    }
}

fn download_content_as_file(content: &[u8], filename: String) -> HttpResponse {
    HttpResponse::Ok()
        .header(
            http::header::CONTENT_DISPOSITION,
            ContentDisposition {
                disposition: DispositionType::Attachment,
                parameters: vec![DispositionParam::Filename(
                    Charset::Iso_8859_1,          // The character set for the bytes of the filename
                    None, // The optional language tag (see `language-tag` crate)
                    filename.as_bytes().to_vec(), // the actual bytes of the filename
                )],
            },
        )
        .header(
            http::header::HeaderName::from_static(headers::XFILENAME),
            filename,
        )
        .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
        .body(Bytes::from(content))
}

fn generate_origin_encryption_keys(
    origin: &Origin,
    req: &HttpRequest<AppState>,
) -> Result<OriginPublicEncryptionKey> {
    debug!("Generate Origin Encryption Keys {:?} for {:?}", req, origin);
    let session_id = helpers::get_session_id(req);

    let mut public_request = OriginPublicEncryptionKeyCreate::new();
    let mut private_request = OriginPrivateEncryptionKeyCreate::new();
    let mut public_key = OriginPublicEncryptionKey::new();
    let mut private_key = OriginPrivateEncryptionKey::new();

    public_key.set_owner_id(session_id);
    private_key.set_owner_id(session_id);
    public_key.set_name(origin.get_name().to_string());
    public_key.set_origin_id(origin.get_id());
    private_key.set_name(origin.get_name().to_string());
    private_key.set_origin_id(origin.get_id());

    let pair = BoxKeyPair::generate_pair_for_origin(origin.get_name()).map_err(Error::HabitatCore)?;
    public_key.set_revision(pair.rev.clone());
    public_key.set_body(
        pair.to_public_string()
            .map_err(Error::HabitatCore)?
            .into_bytes(),
    );
    private_key.set_revision(pair.rev.clone());
    private_key.set_body(
        pair.to_secret_string()
            .map_err(Error::HabitatCore)?
            .into_bytes(),
    );

    public_request.set_public_encryption_key(public_key);
    private_request.set_private_encryption_key(private_key);

    let key = route_message::<OriginPublicEncryptionKeyCreate, OriginPublicEncryptionKey>(
        req,
        &public_request,
    )?;
    route_message::<OriginPrivateEncryptionKeyCreate, OriginPrivateEncryptionKey>(
        req,
        &private_request,
    )?;

    Ok(key)
}

fn encrypt(req: &HttpRequest<AppState>, content: &Bytes) -> Result<String> {
    bldr_core::integrations::encrypt(&req.state().config.api.key_path, content)
        .map_err(Error::BuilderCore)
}

fn decrypt(req: &HttpRequest<AppState>, content: &str) -> Result<String> {
    let bytes = bldr_core::integrations::decrypt(&req.state().config.api.key_path, content)?;
    Ok(String::from_utf8(bytes)?)
}
