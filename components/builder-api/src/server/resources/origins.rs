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

use actix_web::http::{self, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Json, Path};
use protocol::originsrv::*;

use hab_core::crypto::keys::{parse_key_str, parse_name_with_rev, PairType};
use hab_core::crypto::BoxKeyPair;
use hab_core::package::ident;

use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::{route_message, Authenticated};
use server::helpers;
use server::AppState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OriginCreateReq {
    name: String,
    default_package_visibility: Option<String>,
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
        debug!("get_origin called, origin = {}", origin);

        match Self::do_get_origin(req, origin) {
            Ok(origin) => HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(origin),
            Err(err) => err.into(),
        }
    }

    fn create_origin((req, body): (HttpRequest<AppState>, Json<OriginCreateReq>)) -> HttpResponse {
        debug!("origin_create called, body = {:?}", body);

        if !ident::is_valid_origin_name(&body.name) {
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }

        match Self::do_create_origin(&req, &body) {
            Ok(origin) => HttpResponse::Created().json(origin),
            Err(err) => err.into(),
        }
    }

    fn create_keys(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok
        debug!("generate_origin_keys called, origin = {}", origin);

        match Self::do_create_keys(req, origin) {
            Ok(_) => HttpResponse::Created().finish(),
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

    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.resource("/depot/origins/{origin}", |r| {
            r.get().f(Origins::get_origin)
        }).resource("/depot/origins", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).with(Self::create_origin);
            })
            .resource("/depot/origins/{origin}/keys", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).f(Self::create_keys);
            })
            .resource("/depot/origins/{origin}/keys/{revision}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).with(Self::upload_origin_key);
            })
    }
}

// TODO: ORIGIN HANDLERS: "/depot/origins/..."

/*
   r.post(
        "/origins",
        XHandler::new(origin_create).before(basic.clone()),
        "origin_create",
    );
    r.put(
        "/origins/:name",
        XHandler::new(origin_update).before(basic.clone()),
        "origin_update",
    );
    r.get("/origins/:origin", origin_show, "origin");
    r.get("/origins/:origin/keys", list_origin_keys, "origin_keys");
    r.get(
        "/origins/:origin/keys/latest",
        download_latest_origin_key,
        "origin_key_latest",
    );
    r.get(
        "/origins/:origin/keys/:revision",
        download_origin_key,
        "origin_key",
    );
    r.get(
        "/origins/:origin/encryption_key",
        XHandler::new(download_latest_origin_encryption_key).before(basic.clone()),
        "origin_encryption_key_download",
    );
    r.post(
        "/origins/:origin/keys",
        XHandler::new(generate_origin_keys).before(basic.clone()),
        "origin_key_generate",
    );
    r.post(
        "/origins/:origin/secret_keys/:revision",
        XHandler::new(upload_origin_secret_key).before(basic.clone()),
        "origin_secret_key_create",
    );
    r.post(
        "/origins/:origin/secret",
        XHandler::new(create_origin_secret).before(basic.clone()),
        "origin_secret_create",
    );
    r.get(
        "/origins/:origin/secret",
        XHandler::new(list_origin_secrets).before(basic.clone()),
        "origin_secret_list",
    );
    r.delete(
        "/origins/:origin/secret/:secret",
        XHandler::new(delete_origin_secret).before(basic.clone()),
        "origin_secret_delete",
    );
    r.get(
        "/origins/:origin/secret_keys/latest",
        XHandler::new(download_latest_origin_secret_key).before(basic.clone()),
        "origin_secret_key_latest",
    );
    r.get(
        "/origins/:origin/integrations/:integration/names",
        XHandler::new(handlers::integrations::fetch_origin_integration_names).before(basic.clone()),
        "origin_integration_get_names",
    );
    r.put(
        "/origins/:origin/integrations/:integration/:name",
        XHandler::new(handlers::integrations::create_origin_integration).before(basic.clone()),
        "origin_integration_put",
    );
    r.delete(
        "/origins/:origin/integrations/:integration/:name",
        XHandler::new(handlers::integrations::delete_origin_integration).before(basic.clone()),
        "origin_integration_delete",
    );
    r.get(
        "/origins/:origin/integrations/:integration/:name",
        XHandler::new(handlers::integrations::get_origin_integration).before(basic.clone()),
        "origin_integration_get",
    );
    r.get(
        "/origins/:origin/integrations",
        XHandler::new(handlers::integrations::fetch_origin_integrations).before(basic.clone()),
        "origin_integrations",
    );
    r.post(
        "/origins/:origin/users/:username/invitations",
        XHandler::new(invite_to_origin).before(basic.clone()),
        "origin_invitation_create",
    );
    r.put(
        "/origins/:origin/invitations/:invitation_id",
        XHandler::new(accept_invitation).before(basic.clone()),
        "origin_invitation_accept",
    );
    r.put(
        "/origins/:origin/invitations/:invitation_id/ignore",
        XHandler::new(ignore_invitation).before(basic.clone()),
        "origin_invitation_ignore",
    );
    r.delete(
        "/origins/:origin/invitations/:invitation_id",
        XHandler::new(rescind_invitation).before(basic.clone()),
        "origin_invitation_rescind",
    );
    r.get(
        "/origins/:origin/invitations",
        XHandler::new(list_origin_invitations).before(basic.clone()),
        "origin_invitations",
    );
    r.get(
        "/origins/:origin/users",
        XHandler::new(list_origin_members).before(basic.clone()),
        "origin_users",
    );
    r.delete(
        "/origins/:origin/users/:username",
        XHandler::new(origin_member_delete).before(basic.clone()),
        "origin_member_delete",
    );
    r.get(
        "/:origin/pkgs",
        XHandler::new(list_unique_packages).before(opt.clone()),
        "packages_unique",
    );
}
*/

/*

use std::fs::{self, remove_file, File};
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use std::result;
use std::str::{from_utf8, FromStr};
use std::sync::Arc;

use bldr_core::access_token::{BUILDER_ACCOUNT_ID, BUILDER_ACCOUNT_NAME};
use bldr_core::api_client::ApiClient;
use bldr_core::helpers::transition_visibility;

use bldr_core::metrics::CounterMetric;
use bodyparser;
use conn::RouteBroker;
use hab_core::crypto::keys::{parse_key_str, parse_name_with_rev, PairType};
use hab_core::crypto::BoxKeyPair;
use hab_core::package::{
    ident, FromArchive, Identifiable, PackageArchive, PackageIdent, PackageTarget,
};
use hab_net::privilege::FeatureFlags;
use hab_net::{ErrCode, NetError, NetOk, NetResult};
use helpers::{
    self, all_visibilities, check_origin_access, check_origin_owner, dont_cache_response,
    get_param, get_session_id_and_name, trigger_from_request, validate_params,
    visibility_for_optional_session,
};
use hyper::header::{Charset, ContentDisposition, DispositionParam, DispositionType};
use hyper::mime::{Attr, Mime, SubLevel, TopLevel, Value};
use middleware::SegmentCli;
use persistent;
use protobuf;
use protocol::jobsrv::{
    JobGraphPackagePreCreate, JobGraphPackageStats, JobGraphPackageStatsGet, JobGroup,
    JobGroupAbort, JobGroupGet, JobGroupOriginGet, JobGroupOriginResponse, JobGroupSpec,
    JobGroupTrigger,
};
use protocol::originsrv::*;
use protocol::sessionsrv::{Account, AccountGet, AccountOriginRemove};
use regex::Regex;
use router::{Params, Router};
use serde_json;
use tempfile::{tempdir_in, TempDir};
use url;
use uuid::Uuid;

use super::super::headers::*;
use super::super::DepotUtil;
use feat;

use backend::{s3, s3::S3Cli};
use config::Config;
use error::{Error, Result};
use handlers;
use metrics::Counter;
use middleware::{route_message, Authenticated, XHandler};
use net_err::{render_json, render_net_error};
use upstream::UpstreamCli;


#[derive(Clone, Serialize, Deserialize)]
struct OriginCreateReq {
    name: String,
    default_package_visibility: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct OriginUpdateReq {
    default_package_visibility: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct OriginSecretPayload {
    name: String,
    value: String,
}

const ONE_YEAR_IN_SECS: usize = 31536000;

pub fn origin_update(req: &HttpRequest<AppState>) -> HttpResponse {
    let mut request = OriginUpdate::new();
    match get_param(req, "name") {
        Some(origin) => request.set_name(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }

    if !check_origin_access(req, request.get_name()).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    match req.get::<bodyparser::Struct<OriginUpdateReq>>() {
        Ok(Some(body)) => {
            let dpv = match body
                .default_package_visibility
                .parse::<OriginPackageVisibility>()
            {
                Ok(x) => x,
                Err(_) => return Ok(Response::with(status::UnprocessableEntity)),
            };
            request.set_default_package_visibility(dpv);
        }
        _ => return Ok(Response::with(status::UnprocessableEntity)),
    }
    match helpers::get_origin(req, request.get_name()) {
        Ok(origin) => request.set_id(origin.get_id()),
        Err(err) => return Ok(render_net_error(&err)),
    }
    match route_message::<OriginUpdate, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn rescind_invitation(req: &HttpRequest<AppState>) -> HttpResponse {
    let mut request = OriginInvitationRescindRequest::new();
    {
        let session = req.extensions.get::<Authenticated>().unwrap();
        request.set_owner_id(session.get_id());
    }
    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let invitation = match get_param(req, "invitation_id") {
        Some(invitation) => invitation,
        None => return Ok(Response::with(status::BadRequest)),
    };
    match invitation.parse::<u64>() {
        Ok(invitation_id) => request.set_invitation_id(invitation_id),
        Err(_) => return Ok(Response::with(status::BadRequest)),
    }

    debug!(
        "Rescinding invitation id {} for user {} origin {}",
        request.get_invitation_id(),
        request.get_owner_id(),
        &origin
    );

    match route_message::<OriginInvitationRescindRequest, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn ignore_invitation(req: &HttpRequest<AppState>) -> HttpResponse {
    let mut request = OriginInvitationIgnoreRequest::new();
    {
        let session = req.extensions.get::<Authenticated>().unwrap();
        request.set_account_id(session.get_id());
    }
    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let invitation = match get_param(req, "invitation_id") {
        Some(invitation) => invitation,
        None => return Ok(Response::with(status::BadRequest)),
    };
    match invitation.parse::<u64>() {
        Ok(invitation_id) => request.set_invitation_id(invitation_id),
        Err(_) => return Ok(Response::with(status::BadRequest)),
    }

    debug!(
        "Ignoring invitation id {} for user {} origin {}",
        request.get_invitation_id(),
        request.get_account_id(),
        &origin
    );

    match route_message::<OriginInvitationIgnoreRequest, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn accept_invitation(req: &HttpRequest<AppState>) -> HttpResponse {
    let mut request = OriginInvitationAcceptRequest::new();
    request.set_ignore(false);
    {
        let session = req.extensions.get::<Authenticated>().unwrap();
        request.set_account_id(session.get_id());
    }
    match get_param(req, "origin") {
        Some(origin) => request.set_origin_name(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    let invitation = match get_param(req, "invitation_id") {
        Some(invitation) => invitation,
        None => return Ok(Response::with(status::BadRequest)),
    };
    match invitation.parse::<u64>() {
        Ok(invitation_id) => request.set_invite_id(invitation_id),
        Err(_) => return Ok(Response::with(status::BadRequest)),
    }

    debug!(
        "Accepting invitation for user {} origin {}",
        &request.get_account_id(),
        request.get_origin_name()
    );

    match route_message::<OriginInvitationAcceptRequest, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn invite_to_origin(req: &HttpRequest<AppState>) -> HttpResponse {
    let account_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let user_to_invite = match get_param(req, "username") {
        Some(username) => username,
        None => return Ok(Response::with(status::BadRequest)),
    };

    debug!(
        "Creating invitation for user {} origin {}",
        &user_to_invite, &origin
    );

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut request = AccountGet::new();
    let mut invite_request = OriginInvitationCreate::new();
    request.set_name(user_to_invite.to_string());

    match route_message::<AccountGet, Account>(req, &request) {
        Ok(mut account) => {
            invite_request.set_account_id(account.get_id());
            invite_request.set_account_name(account.take_name());
        }
        Err(err) => return Ok(render_net_error(&err)),
    };

    match helpers::get_origin(req, &origin) {
        Ok(mut origin) => {
            invite_request.set_origin_id(origin.get_id());
            invite_request.set_origin_name(origin.take_name());
        }
        Err(err) => return Ok(render_net_error(&err)),
    }

    invite_request.set_owner_id(account_id);

    // store invitations in the originsrv
    match route_message::<OriginInvitationCreate, OriginInvitation>(req, &invite_request) {
        Ok(invitation) => Ok(render_json(status::Created, &invitation)),
        Err(err) => {
            if err.get_code() == ErrCode::ENTITY_CONFLICT {
                Ok(Response::with(status::NoContent))
            } else {
                Ok(render_net_error(&err))
            }
        }
    }
}

pub fn list_origin_invitations(req: &HttpRequest<AppState>) -> HttpResponse {
    let origin_name = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, &origin_name).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut request = OriginInvitationListRequest::new();
    match helpers::get_origin(req, origin_name.as_str()) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return Ok(render_net_error(&err)),
    }

    match route_message::<OriginInvitationListRequest, OriginInvitationListResponse>(req, &request)
    {
        Ok(list) => {
            let mut response = render_json(status::Ok, &list);
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn list_origin_members(req: &HttpRequest<AppState>) -> HttpResponse {
    let origin_name = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, &origin_name).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut request = OriginMemberListRequest::new();
    match helpers::get_origin(req, origin_name.as_str()) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return Ok(render_net_error(&err)),
    }
    match route_message::<OriginMemberListRequest, OriginMemberListResponse>(req, &request) {
        Ok(list) => {
            let mut response = render_json(status::Ok, &list);
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn origin_member_delete(req: &HttpRequest<AppState>) -> HttpResponse {
    let session = req.extensions.get::<Authenticated>().unwrap().clone();
    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_owner(req, session.get_id(), &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let account_name = match get_param(req, "username") {
        Some(user) => user,
        None => return Ok(Response::with(status::BadRequest)),
    };

    // Do not allow the owner to be removed which would orphan the origin
    if account_name == session.get_name() {
        return Ok(Response::with(status::BadRequest));
    }

    debug!(
        "Deleting user name {} for user {} origin {}",
        &account_name,
        &session.get_id(),
        &origin
    );

    let mut session_request = AccountOriginRemove::new();
    let mut origin_request = OriginMemberRemove::new();

    match helpers::get_origin(req, origin) {
        Ok(origin) => {
            session_request.set_origin_id(origin.get_id());
            origin_request.set_origin_id(origin.get_id());
        }
        Err(err) => return Ok(render_net_error(&err)),
    }
    session_request.set_account_name(account_name.to_string());
    origin_request.set_account_name(account_name.to_string());

    if let Err(err) = route_message::<AccountOriginRemove, NetOk>(req, &session_request) {
        return Ok(render_net_error(&err));
    }

    match route_message::<OriginMemberRemove, NetOk>(req, &origin_request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn download_latest_origin_encryption_key(req: &HttpRequest<AppState>) -> HttpResponse {
    let params = match validate_params(req, &["origin"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    let mut request = OriginPublicEncryptionKeyLatestGet::new();
    let origin = match helpers::get_origin(req, &params["origin"]) {
        Ok(mut origin) => {
            request.set_owner_id(origin.get_owner_id());
            request.set_origin(origin.get_name().to_string());
            origin
        }
        Err(err) => return Ok(render_net_error(&err)),
    };

    let key = match route_message::<OriginPublicEncryptionKeyLatestGet, OriginPublicEncryptionKey>(
        req, &request,
    ) {
        Ok(key) => key,
        Err(err) => {
            if err.get_code() == ErrCode::ENTITY_NOT_FOUND {
                match generate_origin_encryption_keys(&origin, req) {
                    Ok(key) => key,
                    Err(Error::NetError(e)) => return Ok(render_net_error(&e)),
                    Err(_) => unreachable!(),
                }
            } else {
                return Ok(render_net_error(&err));
            }
        }
    };

    let xfilename = format!("{}-{}.pub", key.get_name(), key.get_revision());
    download_content_as_file(key.get_body(), xfilename)
}

fn generate_origin_encryption_keys(
    origin: &Origin,
    req: &HttpRequest<AppState>,
) -> Result<OriginPublicEncryptionKey> {
    debug!("Generate Origin Encryption Keys {:?} for {:?}", req, origin);

    let mut public_request = OriginPublicEncryptionKeyCreate::new();
    let mut private_request = OriginPrivateEncryptionKeyCreate::new();
    let mut public_key = OriginPublicEncryptionKey::new();
    let mut private_key = OriginPrivateEncryptionKey::new();
    {
        let session = req.extensions.get::<Authenticated>().unwrap();
        public_key.set_owner_id(session.get_id());
        private_key.set_owner_id(session.get_id());
    }
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

fn download_latest_origin_secret_key(req: &HttpRequest<AppState>) -> HttpResponse {
    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut request = OriginPrivateSigningKeyGet::new();
    match helpers::get_origin(req, origin) {
        Ok(mut origin) => {
            request.set_owner_id(origin.get_owner_id());
            request.set_origin(origin.take_name());
        }
        Err(err) => return Ok(render_net_error(&err)),
    }
    let key =
        match route_message::<OriginPrivateSigningKeyGet, OriginPrivateSigningKey>(req, &request) {
            Ok(key) => key,
            Err(err) => return Ok(render_net_error(&err)),
        };

    let xfilename = format!("{}-{}.sig.key", key.get_name(), key.get_revision());
    download_content_as_file(key.get_body(), xfilename)
}

fn upload_origin_secret_key(req: &HttpRequest<AppState>) -> HttpResponse {
    debug!("Upload Origin Secret Key {:?}", req);

    let account_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    let mut request = OriginPrivateSigningKeyCreate::new();
    request.set_owner_id(account_id);

    match get_param(req, "origin") {
        Some(origin) => {
            if !check_origin_access(req, &origin).unwrap_or(false) {
                return Ok(Response::with(status::Forbidden));
            }

            match helpers::get_origin(req, &origin) {
                Ok(mut origin) => {
                    request.set_name(origin.take_name());
                    request.set_origin_id(origin.get_id());
                }
                Err(err) => return Ok(render_net_error(&err)),
            }
            origin
        }
        None => return Ok(Response::with(status::BadRequest)),
    };

    match get_param(req, "revision") {
        Some(revision) => request.set_revision(revision),
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut key_content = Vec::new();
    if let Err(e) = req.body.read_to_end(&mut key_content) {
        debug!("Can't read key content {}", e);
        return Ok(Response::with(status::BadRequest));
    }

    match String::from_utf8(key_content.clone()) {
        Ok(content) => match parse_key_str(&content) {
            Ok((PairType::Secret, _, _)) => {
                debug!("Received a valid secret key");
            }
            Ok(_) => {
                debug!("Received a public key instead of a secret key");
                return Ok(Response::with(status::BadRequest));
            }
            Err(e) => {
                debug!("Invalid secret key content: {}", e);
                return Ok(Response::with(status::BadRequest));
            }
        },
        Err(e) => {
            debug!("Can't parse secret key upload content: {}", e);
            return Ok(Response::with(status::BadRequest));
        }
    }

    request.set_body(key_content);
    request.set_owner_id(0);
    match route_message::<OriginPrivateSigningKeyCreate, OriginPrivateSigningKey>(req, &request) {
        Ok(_) => Ok(Response::with(status::Created)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This function should not require authentication (session/auth token)
fn download_origin_key(req: &HttpRequest<AppState>) -> HttpResponse {
    let mut request = OriginPublicSigningKeyGet::new();
    match get_param(req, "origin") {
        Some(origin) => request.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "revision") {
        Some(revision) => request.set_revision(revision),
        None => return Ok(Response::with(status::BadRequest)),
    }
    let key =
        match route_message::<OriginPublicSigningKeyGet, OriginPublicSigningKey>(req, &request) {
            Ok(key) => key,
            Err(err) => return Ok(render_net_error(&err)),
        };
    let xfilename = format!("{}-{}.pub", key.get_name(), key.get_revision());
    download_content_as_file(key.get_body(), xfilename)
}

// This function should not require authentication (session/auth token)
fn download_latest_origin_key(req: &HttpRequest<AppState>) -> HttpResponse {
    let mut request = OriginPublicSigningKeyLatestGet::new();
    match get_param(req, "origin") {
        Some(origin) => request.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    let key = match route_message::<OriginPublicSigningKeyLatestGet, OriginPublicSigningKey>(
        req, &request,
    ) {
        Ok(key) => key,
        Err(err) => return Ok(render_net_error(&err)),
    };

    let xfilename = format!("{}-{}.pub", key.get_name(), key.get_revision());
    download_content_as_file(key.get_body(), xfilename)
}


fn list_origin_keys(req: &HttpRequest<AppState>) -> HttpResponse {
    let origin_name = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut request = OriginPublicSigningKeyListRequest::new();
    match helpers::get_origin(req, &origin_name) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return Ok(render_net_error(&err)),
    }
    match route_message::<OriginPublicSigningKeyListRequest, OriginPublicSigningKeyListResponse>(
        req, &request,
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
            let body = serde_json::to_string(&list).unwrap();
            let mut response = Response::with((status::Ok, body));
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn list_unique_packages(req: &HttpRequest<AppState>) -> HttpResponse {
    let session_id = helpers::get_optional_session_id(req);
    let mut request = OriginPackageUniqueListRequest::new();
    let (start, stop) = match helpers::extract_pagination(req) {
        Ok(range) => range,
        Err(response) => return Ok(response),
    };

    request.set_start(start as u64);
    request.set_stop(stop as u64);

    match get_param(req, "origin") {
        Some(origin) => {
            request.set_visibilities(visibility_for_optional_session(req, session_id, &origin));
            request.set_origin(origin);
        }
        None => return Ok(Response::with(status::BadRequest)),
    }

    match route_message::<OriginPackageUniqueListRequest, OriginPackageUniqueListResponse>(
        req, &request,
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
                Response::with((status::PartialContent, body))
            } else {
                Response::with((status::Ok, body))
            };

            response.headers.set(ContentType(Mime(
                TopLevel::Application,
                SubLevel::Json,
                vec![(Attr::Charset, Value::Utf8)],
            )));
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => return Ok(render_net_error(&err)),
    }
}

fn list_packages(req: &HttpRequest<AppState>) -> HttpResponse {
    let session_id = helpers::get_optional_session_id(req);
    let mut distinct = false;
    let (start, stop) = match helpers::extract_pagination(req) {
        Ok(range) => range,
        Err(response) => return Ok(response),
    };

    let (origin, ident, channel) = {
        let params = req.extensions.get::<Router>().unwrap();

        let origin = match params.find("origin") {
            Some(origin) => origin.to_string(),
            None => return Ok(Response::with(status::BadRequest)),
        };

        let ident: String = if params.find("pkg").is_none() {
            origin.clone()
        } else {
            ident_from_params(&params).to_string()
        };

        let channel = match params.find("channel") {
            Some(ch) => Some(ch.to_string()),
            None => None,
        };

        (origin, ident, channel)
    };

    let packages: NetResult<OriginPackageListResponse>;
    match channel {
        Some(channel) => {
            let mut request = OriginChannelPackageListRequest::new();
            request.set_name(channel);
            request.set_start(start as u64);
            request.set_stop(stop as u64);
            request.set_visibilities(visibility_for_optional_session(req, session_id, &origin));

            request.set_ident(
                OriginPackageIdent::from_str(ident.as_str()).expect("invalid package identifier"),
            );
            packages = route_message::<OriginChannelPackageListRequest, OriginPackageListResponse>(
                req, &request,
            );
        }
        None => {
            let mut request = OriginPackageListRequest::new();
            request.set_start(start as u64);
            request.set_stop(stop as u64);
            request.set_visibilities(visibility_for_optional_session(req, session_id, &origin));

            // only set this if "distinct" is present as a URL parameter, e.g. ?distinct=true
            if helpers::extract_query_value("distinct", req).is_some() {
                distinct = true;
                request.set_distinct(true);
            }

            request.set_ident(
                OriginPackageIdent::from_str(ident.as_str()).expect("invalid package identifier"),
            );
            packages =
                route_message::<OriginPackageListRequest, OriginPackageListResponse>(req, &request);
        }
    }

    match packages {
        Ok(packages) => {
            debug!(
                "list_packages start: {}, stop: {}, total count: {}",
                packages.get_start(),
                packages.get_stop(),
                packages.get_count()
            );

            let mut results = Vec::new();

            // The idea here is for every package we get back, pull its channels using the zmq API
            // and accumulate those results. This avoids the N+1 HTTP requests that would be
            // required to fetch channels for a list of packages in the UI. However, if our request
            // has been marked as "distinct" then skip this step because it doesn't make sense in
            // that case. Let's get platforms at the same time.
            for package in packages.get_idents().to_vec() {
                let mut channels: Option<Vec<String>> = None;
                let mut platforms: Option<Vec<String>> = None;

                if !distinct {
                    channels = helpers::channels_for_package_ident(req, &package);
                    platforms = helpers::platforms_for_package_ident(req, &package);
                }

                let mut pkg_json = serde_json::to_value(package).unwrap();

                if channels.is_some() {
                    pkg_json["channels"] = json!(channels);
                }

                if platforms.is_some() {
                    pkg_json["platforms"] = json!(platforms);
                }

                results.push(pkg_json);
            }

            let body = helpers::package_results_json(
                &results,
                packages.get_count() as isize,
                packages.get_start() as isize,
                packages.get_stop() as isize,
            );

            let mut response = if packages.get_count() as isize > (packages.get_stop() as isize + 1)
            {
                Response::with((status::PartialContent, body))
            } else {
                Response::with((status::Ok, body))
            };

            response.headers.set(ContentType(Mime(
                TopLevel::Application,
                SubLevel::Json,
                vec![(Attr::Charset, Value::Utf8)],
            )));
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}


fn show_package(req: &HttpRequest<AppState>) -> HttpResponse {
    let session_id = helpers::get_optional_session_id(req);
    let channel = get_param(req, "channel");

    let mut ident = ident_from_req(req);
    let qualified = ident.fully_qualified();
    let target = target_from_req(req);

    if let Some(channel) = channel {
        if !qualified {
            let mut request = OriginChannelPackageLatestGet::new();
            request.set_name(channel.clone());
            request.set_target(target.to_string());
            request.set_visibilities(visibility_for_optional_session(
                req,
                session_id,
                &ident.get_origin(),
            ));
            request.set_ident(ident.clone());

            match route_message::<OriginChannelPackageLatestGet, OriginPackageIdent>(req, &request)
            {
                Ok(id) => ident = id.into(),
                Err(err) => {
                    // Notify upstream with a non-fully qualified ident to handle checking
                    // of a package that does not exist in the on-premise depot
                    notify_upstream(req, &ident, &target);
                    return Ok(render_net_error(&err));
                }
            }
        }

        // Notify upstream with a fully qualified ident
        notify_upstream(req, &ident, &target);

        let mut request = OriginChannelPackageGet::new();
        request.set_name(channel);
        request.set_visibilities(visibility_for_optional_session(
            req,
            session_id,
            &ident.get_origin(),
        ));
        request.set_ident(ident);

        match route_message::<OriginChannelPackageGet, OriginPackage>(req, &request) {
            Ok(pkg) => render_package(req, &pkg, false),
            Err(err) => Ok(render_net_error(&err)),
        }
    } else {
        if !qualified {
            let mut request = OriginPackageLatestGet::new();
            request.set_target(target.to_string());
            request.set_visibilities(visibility_for_optional_session(
                req,
                session_id,
                &ident.get_origin(),
            ));
            request.set_ident(ident.clone());

            match route_message::<OriginPackageLatestGet, OriginPackageIdent>(req, &request) {
                Ok(id) => ident = id.into(),
                Err(err) => {
                    // Notify upstream with a non-fully qualified ident to handle checking
                    // of a package that does not exist in the on-premise depot
                    notify_upstream(req, &ident, &target);
                    return Ok(render_net_error(&err));
                }
            }
        }

        // Notify upstream with a fully qualified ident
        notify_upstream(req, &ident, &target);

        let mut request = OriginPackageGet::new();
        request.set_visibilities(visibility_for_optional_session(
            req,
            session_id,
            &ident.get_origin(),
        ));
        request.set_ident(ident.clone());

        match route_message::<OriginPackageGet, OriginPackage>(req, &request) {
            Ok(pkg) => render_package(req, &pkg, qualified), // Cache if request was qualified
            Err(err) => Ok(render_net_error(&err)),
        }
    }
}

fn render_package(
    req: &HttpRequest<AppState>,
    pkg: &OriginPackage,
    should_cache: bool,
) -> HttpResponse {
    let mut pkg_json = serde_json::to_value(pkg.clone()).unwrap();
    let channels = helpers::channels_for_package_ident(req, pkg.get_ident());
    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(is_a_service(pkg));

    let body = serde_json::to_string(&pkg_json).unwrap();
    let mut response = Response::with((status::Ok, body));
    response.headers.set(ETag(pkg.get_checksum().to_string()));
    response.headers.set(ContentType(Mime(
        TopLevel::Application,
        SubLevel::Json,
        vec![(Attr::Charset, Value::Utf8)],
    )));

    if should_cache {
        do_cache_response(&mut response);
    } else {
        dont_cache_response(&mut response);
    }

    Ok(response)
}




pub fn create_origin_secret(req: &HttpRequest<AppState>) -> HttpResponse {
    let params = match validate_params(req, &["origin"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    let origin = &params["origin"];

    let body = match req.get::<bodyparser::Struct<OriginSecretPayload>>() {
        Ok(Some(body)) => {
            if body.name.len() <= 0 {
                return Ok(Response::with((
                    status::UnprocessableEntity,
                    "Missing value for field: `name`",
                )));
            }
            if body.value.len() <= 0 {
                return Ok(Response::with((
                    status::UnprocessableEntity,
                    "Missing value for field: `value`",
                )));
            }
            body
        }
        _ => return Ok(Response::with(status::UnprocessableEntity)),
    };

    // get metadata from secret payload
    let secret_metadata = match BoxKeyPair::secret_metadata(body.value.as_bytes()) {
        Ok(res) => res,
        Err(e) => {
            return Ok(Response::with((
                status::UnprocessableEntity,
                format!("Failed to get metadata from payload: {}", e),
            )))
        }
    };

    debug!("Secret Metadata: {:?}", secret_metadata);

    let mut secret = OriginSecret::new();
    secret.set_name(body.name);
    secret.set_value(body.value.clone());
    let mut db_priv_request = OriginPrivateEncryptionKeyGet::new();
    let mut db_pub_request = OriginPublicEncryptionKeyGet::new();
    match helpers::get_origin(req, origin) {
        Ok(mut origin) => {
            let origin_name = origin.take_name();
            let origin_owner_id = origin.get_owner_id();
            secret.set_origin_id(origin.get_id());
            db_priv_request.set_owner_id(origin_owner_id.clone());
            db_priv_request.set_origin(origin_name.clone());
            db_pub_request.set_owner_id(origin_owner_id.clone());
            db_pub_request.set_origin(origin_name.clone());
        }
        Err(err) => return Ok(render_net_error(&err)),
    }

    // fetch the private origin encryption key from the database
    let priv_key = match route_message::<OriginPrivateEncryptionKeyGet, OriginPrivateEncryptionKey>(
        req,
        &db_priv_request,
    ) {
        Ok(key) => {
            let key_str = from_utf8(key.get_body()).unwrap();
            match BoxKeyPair::secret_key_from_str(key_str) {
                Ok(key) => key,
                Err(e) => {
                    return Ok(Response::with((
                        status::UnprocessableEntity,
                        format!("{}", e),
                    )))
                }
            }
        }
        Err(err) => return Ok(render_net_error(&err)),
    };

    let (name, rev) = match parse_name_with_rev(secret_metadata.sender) {
        Ok(val) => val,
        Err(e) => {
            return Ok(Response::with((
                status::UnprocessableEntity,
                format!("Failed to parse name and revision: {}", e),
            )))
        }
    };

    db_pub_request.set_revision(rev.clone());

    debug!("Using key {:?}-{:?}", name, &rev);

    // fetch the public origin encryption key from the database
    let pub_key = match route_message::<OriginPublicEncryptionKeyGet, OriginPublicEncryptionKey>(
        req,
        &db_pub_request,
    ) {
        Ok(key) => {
            let key_str = from_utf8(key.get_body()).unwrap();
            match BoxKeyPair::public_key_from_str(key_str) {
                Ok(key) => key,
                Err(e) => {
                    return Ok(Response::with((
                        status::UnprocessableEntity,
                        format!("{}", e),
                    )))
                }
            }
        }
        Err(err) => return Ok(render_net_error(&err)),
    };

    let box_key_pair = BoxKeyPair::new(name, rev.clone(), Some(pub_key), Some(priv_key));

    debug!("Decrypting string: {:?}", &secret_metadata.ciphertext);

    // verify we can decrypt the message
    match box_key_pair.decrypt(&secret_metadata.ciphertext, None, None) {
        Ok(_) => (),
        Err(e) => {
            return Ok(Response::with((
                status::UnprocessableEntity,
                format!("{}", e),
            )))
        }
    };

    let mut request = OriginSecretCreate::new();
    request.set_secret(secret);

    match route_message::<OriginSecretCreate, OriginSecret>(req, &request) {
        Ok(_) => Ok(Response::with(status::Created)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn list_origin_secrets(req: &HttpRequest<AppState>) -> HttpResponse {
    let params = match validate_params(req, &["origin"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    let mut request = OriginSecretListGet::new();
    match helpers::get_origin(req, &params["origin"]) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return Ok(render_net_error(&err)),
    }

    match route_message::<OriginSecretListGet, OriginSecretList>(req, &request) {
        Ok(list) => {
            let mut response = render_json(status::Ok, &list.get_secrets());
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn delete_origin_secret(req: &HttpRequest<AppState>) -> HttpResponse {
    let params = match validate_params(req, &["origin", "secret"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    let mut request = OriginSecretDelete::new();
    match helpers::get_origin(req, &params["origin"]) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return Ok(render_net_error(&err)),
    }
    request.set_name(params["secret"].to_string());
    match route_message::<OriginSecretDelete, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::Ok)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn ident_from_req(req: &HttpRequest<AppState>) -> OriginPackageIdent {
    let params = req.extensions.get::<Router>().unwrap();
    ident_from_params(&params)
}

fn ident_from_params(params: &Params) -> OriginPackageIdent {
    let mut ident = OriginPackageIdent::new();
    if let Some(origin) = params.find("origin") {
        ident.set_origin(origin.to_string());
    }
    if let Some(name) = params.find("pkg") {
        ident.set_name(name.to_string());
    }
    if let Some(ver) = params.find("version") {
        ident.set_version(ver.to_string());
    }
    if let Some(rel) = params.find("release") {
        ident.set_release(rel.to_string());
    }
    ident
}

fn download_content_as_file(content: &[u8], filename: String) -> HttpResponse {
    let mut response = Response::with((status::Ok, content));
    response.headers.set(ContentDisposition(format!(
        "attachment; filename=\"{}\"",
        filename
    )));
    response.headers.set(XFileName(filename));
    dont_cache_response(&mut response);
    Ok(response)
}

fn target_from_req(req: &HttpRequest<AppState>) -> PackageTarget {
    // A target in a query param over-rides the user agent platform
    let target = match helpers::extract_query_value("target", req) {
        Some(t) => {
            debug!("Query requested target = {}", t);
            t
        }
        None => {
            let user_agent_header = req.headers.get::<UserAgent>().unwrap();
            let user_agent = user_agent_header.as_str();
            debug!("Headers = {}", &user_agent);

            let user_agent_regex = Regex::new(
                r"(?P<client>[^\s]+)\s?(\((?P<target>\w+-\w+); (?P<kernel>.*)\))?",
            ).unwrap();

            match user_agent_regex.captures(user_agent) {
                Some(user_agent_capture) => {
                    if let Some(target_match) = user_agent_capture.name("target") {
                        target_match.as_str().to_string()
                    } else {
                        "".to_string()
                    }
                }
                None => "".to_string(),
            }
        }
    };

    // All of our tooling that depends on this function to return a target will have a user
    // agent that includes the platform, or will specify a target in the query.
    // Therefore, if we can't find a valid target, it's safe to assume that some other kind of HTTP
    // tool is being used, e.g. curl, with looser constraints. For those kinds of cases,
    // let's default it to Linux instead of returning a bad request if we can't properly parse
    // the inbound target.
    match PackageTarget::from_str(&target) {
        Ok(t) => t,
        Err(_) => PackageTarget::from_str("x86_64-linux").unwrap(),
    }
}


fn notify_upstream(req: &HttpRequest<AppState>, ident: &OriginPackageIdent, target: &PackageTarget) {
    let upstream_cli = req.get::<persistent::Read<UpstreamCli>>().unwrap();
    upstream_cli.refresh(ident, target).unwrap();
}

struct TempDownloadPath;

impl typemap::Key for TempDownloadPath {
    type Value = TempDir;
}


fn is_a_service(package: &OriginPackage) -> bool {
    let m = package.get_manifest();

    // TODO: This is a temporary workaround until we plumb in a better solution for
    // determining whether a package is a service from the DB instead of needing
    // to crack the archive file to look for a SVC_USER file
    m.contains("pkg_exposes") || m.contains("pkg_binds") || m.contains("pkg_exports")
}

fn do_cache_response(response: &mut Response) {
    response.headers.set(CacheControl(format!(
        "public, max-age={}",
        ONE_YEAR_IN_SECS
    )));
}
 
*/
