// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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
use iron::headers::{ContentType, UserAgent};
use iron::middleware::BeforeMiddleware;
use iron::prelude::*;
use iron::request::Body;
use iron::typemap;
use iron::{headers, status};
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

pub fn origin_update(req: &mut Request) -> IronResult<Response> {
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

pub fn origin_create(req: &mut Request) -> IronResult<Response> {
    let mut request = OriginCreate::new();
    {
        let session = req.extensions.get::<Authenticated>().unwrap();
        request.set_owner_id(session.get_id());
        request.set_owner_name(session.get_name().to_string());
    }

    match req.get::<bodyparser::Struct<OriginCreateReq>>() {
        Ok(Some(body)) => {
            if let Some(vis) = body.default_package_visibility {
                match vis.parse::<OriginPackageVisibility>() {
                    Ok(vis) => request.set_default_package_visibility(vis),
                    Err(_) => return Ok(Response::with(status::UnprocessableEntity)),
                }
            }
            request.set_name(body.name);
        }
        _ => return Ok(Response::with(status::UnprocessableEntity)),
    }

    if !ident::is_valid_origin_name(request.get_name()) {
        return Ok(Response::with(status::UnprocessableEntity));
    }

    match route_message::<OriginCreate, Origin>(req, &request) {
        Ok(origin) => Ok(render_json(status::Created, &origin)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn origin_show(req: &mut Request) -> IronResult<Response> {
    let mut request = OriginGet::new();
    match get_param(req, "origin") {
        Some(origin) => request.set_name(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match route_message::<OriginGet, Origin>(req, &request) {
        Ok(origin) => {
            let mut response = render_json(status::Ok, &origin);
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn rescind_invitation(req: &mut Request) -> IronResult<Response> {
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

pub fn ignore_invitation(req: &mut Request) -> IronResult<Response> {
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

pub fn accept_invitation(req: &mut Request) -> IronResult<Response> {
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

pub fn invite_to_origin(req: &mut Request) -> IronResult<Response> {
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

pub fn list_origin_invitations(req: &mut Request) -> IronResult<Response> {
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

pub fn list_origin_members(req: &mut Request) -> IronResult<Response> {
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

pub fn origin_member_delete(req: &mut Request) -> IronResult<Response> {
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

fn check_forced(req: &mut Request) -> bool {
    if let Some(flag) = helpers::extract_query_value("forced", req) {
        flag == "true"
    } else {
        false
    }
}

fn write_archive(filename: &PathBuf, body: &mut Body) -> Result<PackageArchive> {
    let file = File::create(&filename)?;
    let mut writer = BufWriter::new(file);
    let mut written: i64 = 0;
    let mut buf = [0u8; 100000]; // Our byte buffer

    loop {
        let len = body.read(&mut buf)?; // Raise IO errors
        match len {
            0 => {
                // 0 == EOF, so stop writing and finish progress
                break;
            }
            _ => {
                // Write the buffer to the BufWriter on the Heap
                let bytes_written = writer.write(&buf[0..len])?;
                if bytes_written == 0 {
                    return Err(Error::WriteSyncFailed);
                }
                written = written + (bytes_written as i64);
            }
        };
    }

    Ok(PackageArchive::new(filename))
}

fn download_latest_origin_encryption_key(req: &mut Request) -> IronResult<Response> {
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
    req: &mut Request,
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

fn generate_origin_keys(req: &mut Request) -> IronResult<Response> {
    debug!("Generate Origin Keys {:?}", req);
    let session_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    match get_param(req, "origin") {
        Some(origin) => {
            if !check_origin_access(req, &origin).unwrap_or(false) {
                return Ok(Response::with(status::Forbidden));
            }

            match helpers::get_origin(req, origin) {
                Ok(origin) => match helpers::generate_origin_keys(req, session_id, origin) {
                    Ok(_) => Ok(Response::with(status::Created)),
                    Err(err) => Ok(render_net_error(&err)),
                },
                Err(err) => Ok(render_net_error(&err)),
            }
        }
        None => Ok(Response::with(status::BadRequest)),
    }
}

fn upload_origin_key(req: &mut Request) -> IronResult<Response> {
    debug!("Upload Origin Public Key {:?}", req);

    let account_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    let mut request = OriginPublicSigningKeyCreate::new();
    request.set_owner_id(account_id);

    let origin = match get_param(req, "origin") {
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
            Ok((PairType::Public, _, _)) => {
                debug!("Received a valid public key");
            }
            Ok(_) => {
                debug!("Received a secret key instead of a public key");
                return Ok(Response::with(status::BadRequest));
            }
            Err(e) => {
                debug!("Invalid public key content: {}", e);
                return Ok(Response::with(status::BadRequest));
            }
        },
        Err(e) => {
            debug!("Can't parse public key upload content: {}", e);
            return Ok(Response::with(status::BadRequest));
        }
    }

    request.set_body(key_content);
    request.set_owner_id(0);
    match route_message::<OriginPublicSigningKeyCreate, OriginPublicSigningKey>(req, &request) {
        Ok(_) => {
            let mut response = Response::with((
                status::Created,
                format!("/origins/{}/keys/{}", &origin, &request.get_revision()),
            ));
            let mut base_url: url::Url = req.url.clone().into();
            base_url.set_path(&format!("key/{}-{}", &origin, &request.get_revision()));
            response
                .headers
                .set(headers::Location(format!("{}", base_url)));
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn download_latest_origin_secret_key(req: &mut Request) -> IronResult<Response> {
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

fn upload_origin_secret_key(req: &mut Request) -> IronResult<Response> {
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

fn upload_package(req: &mut Request) -> IronResult<Response> {
    let ident = ident_from_req(req);
    let s3handler = req.get::<persistent::Read<S3Cli>>().unwrap();

    let (session_id, session_name) = get_session_id_and_name(req);

    if !ident.valid() || !ident.fully_qualified() {
        info!(
            "Invalid or not fully qualified package identifier: {}",
            ident
        );
        return Ok(Response::with(status::BadRequest));
    }

    if !check_origin_access(req, &ident.get_origin()).unwrap_or(false) {
        debug!("Failed origin access check, ident: {}", ident);

        return Ok(Response::with(status::Forbidden));
    }

    let depot = req.get::<persistent::Read<Config>>().unwrap();
    let checksum_from_param = match helpers::extract_query_value("checksum", req) {
        Some(checksum) => checksum,
        None => return Ok(Response::with(status::BadRequest)),
    };

    debug!(
        "UPLOADING checksum={}, ident={}",
        checksum_from_param, ident
    );

    // Find the path to folder where archive should be created, and
    // create the folder if necessary
    let parent_path = depot.packages_path();

    match fs::create_dir_all(parent_path.clone()) {
        Ok(_) => {}
        Err(e) => {
            error!("Unable to create archive directory, err={:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    };

    // Create a temp file at the archive location
    let dir = tempdir_in(depot.packages_path()).expect("Unable to create a tempdir!");
    let file_path = dir.path();
    let temp_name = format!("{}.tmp", Uuid::new_v4());
    let temp_path = parent_path.join(file_path).join(temp_name);

    let mut archive = match write_archive(&temp_path, &mut req.body) {
        Ok(a) => a,
        Err(e) => {
            warn!("Error writing archive to disk: {:?}", &e);
            return Ok(Response::with((
                status::InternalServerError,
                format!("ds:up:7, err={:?}", e),
            )));
        }
    };

    debug!("Package Archive: {:#?}", archive);

    let target_from_artifact = match archive.target() {
        Ok(target) => target,
        Err(e) => {
            info!("Could not read the target for {:#?}: {:#?}", archive, e);
            return Ok(Response::with((
                status::UnprocessableEntity,
                format!("ds:up:1, err={:?}", e),
            )));
        }
    };

    if !depot.api.targets.contains(&target_from_artifact) {
        debug!(
            "Unsupported package platform or architecture {}.",
            target_from_artifact
        );
        return Ok(Response::with(status::NotImplemented));
    };

    let mut ident_req = OriginPackageGet::new();
    ident_req.set_ident(ident.clone());
    ident_req.set_visibilities(all_visibilities());

    // Return conflict only if we have BOTH package metadata and a valid
    // archive on disk.
    let upload_forced = check_forced(req);
    let origin_package_found =
        match route_message::<OriginPackageGet, OriginPackage>(req, &ident_req) {
            Ok(_) => {
                if upload_forced {
                    debug!(
                        "Upload was forced, bypassing database validation: {}!",
                        ident
                    );
                    true
                } else {
                    return Ok(Response::with(status::Conflict));
                }
            }
            Err(err) => {
                if err.get_code() == ErrCode::ENTITY_NOT_FOUND {
                    false
                } else {
                    return Ok(render_net_error(&err));
                }
            }
        };

    let checksum_from_artifact = match archive.checksum() {
        Ok(cksum) => cksum,
        Err(e) => {
            info!("Could not compute a checksum for {:#?}: {:#?}", archive, e);
            return Ok(Response::with((status::UnprocessableEntity, "ds:up:2")));
        }
    };

    if checksum_from_param != checksum_from_artifact {
        info!(
            "Checksums did not match: from_param={:?}, from_artifact={:?}",
            checksum_from_param, checksum_from_artifact
        );
        return Ok(Response::with((status::UnprocessableEntity, "ds:up:3")));
    }

    // Check with scheduler to ensure we don't have circular deps, if configured
    if feat::is_enabled(feat::Jobsrv) {
        if let Err(r) = check_circular_deps(req, &ident, &target_from_artifact, &mut archive) {
            warn!("Failed to check circular dependency, err={:?}", r);
            return Ok(r);
        }
    }

    let filename = file_path.join(Config::archive_name(
        &(&ident).into(),
        &target_from_artifact,
    ));
    let temp_ident = ident.to_owned().into();

    match fs::rename(&temp_path, &filename) {
        Ok(_) => {}
        Err(e) => {
            error!(
                "Unable to rename temp archive {:?} to {:?}, err={:?}",
                temp_path, filename, e
            );
            return Ok(Response::with(status::InternalServerError));
        }
    }

    if s3handler
        .upload(&filename, &temp_ident, &target_from_artifact)
        .is_err()
    {
        error!("Unable to upload archive to s3!");
        return Ok(Response::with(status::InternalServerError));
    } else {
        info!("File added to Depot: {:?}", &filename);
        let mut archive = PackageArchive::new(filename.clone());
        let mut package = match OriginPackageCreate::from_archive(&mut archive) {
            Ok(package) => package,
            Err(e) => {
                info!("Error building package from archive: {:#?}", e);
                return Ok(Response::with((status::UnprocessableEntity, "ds:up:5")));
            }
        };

        if !ident.satisfies(package.get_ident()) {
            info!(
                "Ident mismatch, expected={:?}, got={:?}",
                ident,
                package.get_ident()
            );

            return Ok(Response::with((status::UnprocessableEntity, "ds:up:6")));
        }

        let builder_flag = helpers::extract_query_value("builder", req);

        match process_upload_for_package_archive(
            &ident,
            &mut package,
            &target_from_artifact,
            session_id,
            session_name,
            origin_package_found,
            builder_flag,
        ) {
            Ok(_) => {
                let mut response = Response::with((
                    status::Created,
                    format!("/pkgs/{}/download", package.get_ident()),
                ));
                let mut base_url: url::Url = req.url.clone().into();
                base_url.set_path(&format!("pkgs/{}/download", package.get_ident()));
                response
                    .headers
                    .set(headers::Location(format!("{}", base_url)));

                match remove_file(&filename) {
                    Ok(_) => debug!(
                        "Successfully removed cached file after upload. {:?}",
                        &filename
                    ),
                    Err(e) => error!(
                        "Failed to remove cached file after upload: {:?}, {}",
                        &filename, e
                    ),
                }

                Ok(response)
            }
            Err(_) => {
                info!(
                    "Ident mismatch, expected={:?}, got={:?}",
                    ident,
                    package.get_ident()
                );
                Ok(Response::with((status::UnprocessableEntity, "ds:up:6")))
            }
        }
    }
}

// This route is unreachable when jobsrv_enabled is false
fn package_stats(req: &mut Request) -> IronResult<Response> {
    let mut request = JobGraphPackageStatsGet::new();
    match get_param(req, "origin") {
        Some(origin) => request.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }

    match route_message::<JobGraphPackageStatsGet, JobGraphPackageStats>(req, &request) {
        Ok(stats) => {
            let mut response = render_json(status::Ok, &stats);
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This route is unreachable when jobsrv_enabled is false
fn schedule(req: &mut Request) -> IronResult<Response> {
    let (session_id, session_name) = get_session_id_and_name(req);

    let segment = req.get::<persistent::Read<SegmentCli>>().unwrap();
    let origin_name = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, &origin_name).unwrap_or(false) {
        debug!("Failed origin access check, origin: {}", &origin_name);
        return Ok(Response::with(status::Forbidden));
    }

    let package = match get_param(req, "pkg") {
        Some(pkg) => pkg,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let target = match helpers::extract_query_value("target", req) {
        Some(target) => target,
        None => String::from("x86_64-linux"),
    };
    let deps_only = helpers::extract_query_value("deps_only", req).is_some();
    let origin_only = helpers::extract_query_value("origin_only", req).is_some();
    let package_only = helpers::extract_query_value("package_only", req).is_some();

    // We only support building for Linux x64 only currently
    if target != "x86_64-linux" {
        info!("Rejecting build with target: {}", target);
        return Ok(Response::with(status::BadRequest));
    }

    let mut secret_key_request = OriginPrivateSigningKeyGet::new();
    let origin = match helpers::get_origin(req, &origin_name) {
        Ok(origin) => {
            secret_key_request.set_owner_id(origin.get_owner_id());
            secret_key_request.set_origin(origin_name.clone());
            origin
        }
        Err(err) => return Ok(render_net_error(&err)),
    };
    let account_name = session_name.clone();
    let need_keys = match route_message::<OriginPrivateSigningKeyGet, OriginPrivateSigningKey>(
        req,
        &secret_key_request,
    ) {
        Ok(key) => {
            let mut pub_key_request = OriginPublicSigningKeyGet::new();
            pub_key_request.set_origin(origin_name.clone());
            pub_key_request.set_revision(key.get_revision().to_string());
            route_message::<OriginPublicSigningKeyGet, OriginPublicSigningKey>(
                req,
                &pub_key_request,
            ).is_err()
        }
        Err(_) => true,
    };

    if need_keys {
        if let Err(err) = helpers::generate_origin_keys(req, session_id, origin) {
            return Ok(render_net_error(&err));
        }
    }

    let mut request = JobGroupSpec::new();
    request.set_origin(origin_name);
    request.set_package(package);
    request.set_target(target);
    request.set_deps_only(deps_only);
    request.set_origin_only(origin_only);
    request.set_package_only(package_only);
    request.set_trigger(trigger_from_request(req));
    request.set_requester_id(session_id);
    request.set_requester_name(session_name);

    match route_message::<JobGroupSpec, JobGroup>(req, &request) {
        Ok(group) => {
            let msg = format!("Scheduled job group for {}", group.get_project_name());

            // We don't really want to abort anything just because a call to segment failed. Let's
            // just log it and move on.
            if let Err(e) = segment.track(&account_name, &msg) {
                warn!("Error tracking scheduling of job group in segment, {}", e);
            }

            let mut response = render_json(status::Ok, &group);
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This route is unreachable when jobsrv_enabled is false
fn get_origin_schedule_status(req: &mut Request) -> IronResult<Response> {
    let mut request = JobGroupOriginGet::new();

    match get_param(req, "origin") {
        Some(origin) => request.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }

    let limit = match helpers::extract_query_value("limit", req) {
        Some(limit) => limit.parse::<u32>().unwrap_or(10),
        None => 10,
    };
    request.set_limit(limit);

    match route_message::<JobGroupOriginGet, JobGroupOriginResponse>(req, &request) {
        Ok(jgor) => {
            let mut response = render_json(status::Ok, &jgor.get_job_groups());
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(e) => Ok(render_net_error(&e)),
    }
}

// This route is unreachable when jobsrv_enabled is false
fn get_schedule(req: &mut Request) -> IronResult<Response> {
    let group_id = {
        let group_id_str = match get_param(req, "groupid") {
            Some(s) => s,
            None => return Ok(Response::with(status::BadRequest)),
        };

        match group_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return Ok(Response::with(status::BadRequest)),
        }
    };

    let include_projects = match helpers::extract_query_value("include_projects", req) {
        Some(val) => val.parse::<bool>().unwrap_or(true),
        None => true,
    };

    let mut request = JobGroupGet::new();
    request.set_group_id(group_id);
    request.set_include_projects(include_projects);

    match route_message::<JobGroupGet, JobGroup>(req, &request) {
        Ok(group) => {
            let mut response = render_json(status::Ok, &group);
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

// TODO (SA) This is an experiemental dev-only function for now
// This route is unreachable when jobsrv_enabled is false
fn abort_schedule(req: &mut Request) -> IronResult<Response> {
    let group_id = {
        let params = req.extensions.get::<Router>().unwrap();
        let group_id_str = match params.find("groupid") {
            Some(s) => s,
            None => return Ok(Response::with(status::BadRequest)),
        };

        match group_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return Ok(Response::with(status::BadRequest)),
        }
    };

    let mut request = JobGroupAbort::new();
    request.set_group_id(group_id);

    match route_message::<JobGroupAbort, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::Ok)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This function should not require authentication (session/auth token)
fn download_origin_key(req: &mut Request) -> IronResult<Response> {
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
fn download_latest_origin_key(req: &mut Request) -> IronResult<Response> {
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

fn package_channels(req: &mut Request) -> IronResult<Response> {
    let session_id = helpers::get_optional_session_id(req);
    let mut request = OriginPackageChannelListRequest::new();
    let ident = ident_from_req(req);

    if !ident.fully_qualified() {
        return Ok(Response::with(status::BadRequest));
    }

    request.set_visibilities(visibility_for_optional_session(
        req,
        session_id,
        &ident.get_origin(),
    ));
    request.set_ident(ident);

    match route_message::<OriginPackageChannelListRequest, OriginPackageChannelListResponse>(
        req, &request,
    ) {
        Ok(channels) => {
            let list: Vec<String> = channels
                .get_channels()
                .iter()
                .map(|channel| channel.get_name().to_string())
                .collect();
            let body = serde_json::to_string(&list).unwrap();
            let mut response = Response::with((status::Ok, body));
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(e) => Ok(render_net_error(&e)),
    }
}

fn download_package(req: &mut Request) -> IronResult<Response> {
    let depot = req.get::<persistent::Read<Config>>().unwrap();
    let session_id = helpers::get_optional_session_id(req);
    let s3handler = req.get::<persistent::Read<S3Cli>>().unwrap();
    let mut ident_req = OriginPackageGet::new();
    let ident = ident_from_req(req);
    let mut vis = visibility_for_optional_session(req, session_id, &ident.get_origin());
    vis.push(OriginPackageVisibility::Hidden);
    ident_req.set_visibilities(vis);
    ident_req.set_ident(ident.clone());

    let target = target_from_req(req);
    if !depot.api.targets.contains(&target) {
        return Ok(Response::with((
            status::NotImplemented,
            format!("Unsupported client platform ({}).", &target),
        )));
    }

    match route_message::<OriginPackageGet, OriginPackage>(req, &ident_req) {
        Ok(package) => {
            let dir = tempdir_in(depot.packages_path()).expect("Unable to create a tempdir!");
            let file_path = dir
                .path()
                .join(Config::archive_name(&(&package).into(), &target));
            let temp_ident = ident.to_owned().into();
            match s3handler.download(&file_path, &temp_ident, &target) {
                Ok(archive) => download_response_for_archive(archive, dir),
                Err(e) => {
                    warn!("Failed to download package, err={:?}", e);
                    Ok(Response::with(status::NotFound))
                }
            }
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn list_origin_keys(req: &mut Request) -> IronResult<Response> {
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

fn list_unique_packages(req: &mut Request) -> IronResult<Response> {
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

fn list_package_versions(req: &mut Request) -> IronResult<Response> {
    let session_id = helpers::get_optional_session_id(req);
    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let name = match get_param(req, "pkg") {
        Some(pkg) => pkg,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut request = OriginPackageVersionListRequest::new();
    request.set_visibilities(visibility_for_optional_session(req, session_id, &origin));
    request.set_origin(origin);
    request.set_name(name);

    match route_message::<OriginPackageVersionListRequest, OriginPackageVersionListResponse>(
        req, &request,
    ) {
        Ok(packages) => {
            let body = serde_json::to_string(&packages.get_versions().to_vec()).unwrap();
            let mut response = Response::with((status::Ok, body));

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

fn package_privacy_toggle(req: &mut Request) -> IronResult<Response> {
    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let visibility = match get_param(req, "visibility") {
        Some(v) => v,
        None => return Ok(Response::with(status::BadRequest)),
    };

    // users aren't allowed to set packages to hidden manually
    if visibility.to_lowercase() == "hidden" {
        return Ok(Response::with(status::BadRequest));
    }

    let ident = ident_from_req(req);

    if !ident.valid() || !ident.fully_qualified() {
        info!(
            "Invalid or not fully qualified package identifier: {}",
            ident
        );
        return Ok(Response::with(status::BadRequest));
    }

    let opv: OriginPackageVisibility = match visibility.parse() {
        Ok(o) => o,
        Err(_) => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut opg = OriginPackageGet::new();
    opg.set_ident(ident);
    opg.set_visibilities(all_visibilities());

    match route_message::<OriginPackageGet, OriginPackage>(req, &opg) {
        Ok(mut package) => {
            let real_visibility = transition_visibility(opv, package.get_visibility());
            let mut opu = OriginPackageUpdate::new();
            package.set_visibility(real_visibility);
            opu.set_pkg(package);

            match route_message::<OriginPackageUpdate, NetOk>(req, &opu) {
                Ok(_) => Ok(Response::with(status::Ok)),
                Err(e) => Ok(render_net_error(&e)),
            }
        }
        Err(e) => Ok(render_net_error(&e)),
    }
}

fn list_packages(req: &mut Request) -> IronResult<Response> {
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

fn list_channels(req: &mut Request) -> IronResult<Response> {
    let origin_name = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut request = OriginChannelListRequest::new();
    request.set_include_sandbox_channels(false);

    match helpers::get_origin(req, &origin_name) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return Ok(render_net_error(&err)),
    }

    // Pass ?sandbox=true to this endpoint to include sanbox channels in the list. They are not
    // there by default.
    if let Some(sandbox) = helpers::extract_query_value("sandbox", req) {
        if sandbox == "true" {
            request.set_include_sandbox_channels(true);
        }
    }

    match route_message::<OriginChannelListRequest, OriginChannelListResponse>(req, &request) {
        Ok(list) => {
            let list: Vec<OriginChannelIdent> = list
                .get_channels()
                .iter()
                .map(|channel| {
                    let mut ident = OriginChannelIdent::new();
                    ident.set_name(channel.get_name().to_string());
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

fn create_channel(req: &mut Request) -> IronResult<Response> {
    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let channel = match get_param(req, "channel") {
        Some(channel) => channel,
        None => return Ok(Response::with(status::BadRequest)),
    };

    match helpers::create_channel(req, &origin, &channel) {
        Ok(origin_channel) => Ok(render_json(status::Created, &origin_channel)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn delete_channel(req: &mut Request) -> IronResult<Response> {
    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let channel = match get_param(req, "channel") {
        Some(channel) => channel,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(origin.clone());
    channel_req.set_name(channel.clone());
    match route_message::<OriginChannelGet, OriginChannel>(req, &channel_req) {
        Ok(origin_channel) => {
            // make sure the person trying to create the channel has access to do so
            if !check_origin_access(req, &origin).unwrap_or(false) {
                return Ok(Response::with(status::Forbidden));
            }

            // stable and unstable can't be deleted
            if channel == "stable" || channel == "unstable" {
                return Ok(Response::with(status::Forbidden));
            }

            let mut delete = OriginChannelDelete::new();
            delete.set_id(origin_channel.get_id());
            delete.set_origin_id(origin_channel.get_origin_id());
            match route_message::<OriginChannelDelete, NetOk>(req, &delete) {
                Ok(_) => Ok(Response::with(status::Ok)),
                Err(err) => return Ok(render_net_error(&err)),
            }
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn show_package(req: &mut Request) -> IronResult<Response> {
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

fn search_packages(req: &mut Request) -> IronResult<Response> {
    Counter::SearchPackages.increment();

    let session_id = helpers::get_optional_session_id(req);
    let mut request = OriginPackageSearchRequest::new();
    let (start, stop) = match helpers::extract_pagination(req) {
        Ok(range) => range,
        Err(response) => return Ok(response),
    };
    request.set_start(start as u64);
    request.set_stop(stop as u64);

    if session_id.is_some() {
        let mut my_origins = MyOriginsRequest::new();
        my_origins.set_account_id(session_id.unwrap());

        match route_message::<MyOriginsRequest, MyOriginsResponse>(req, &my_origins) {
            Ok(response) => request.set_my_origins(protobuf::RepeatedField::from_vec(
                response.get_origins().to_vec(),
            )),
            Err(e) => {
                debug!(
                    "Error fetching origins for account id {}, {}",
                    session_id.unwrap(),
                    e
                );
                return Ok(Response::with(status::BadRequest));
            }
        }
    }

    // First, try to parse the query like it's a PackageIdent, since it seems reasonable to expect
    // that many people will try searching using that kind of string, e.g. core/redis.  If that
    // works, set the origin appropriately and do a regular search.  If that doesn't work, do a
    // search across all origins, similar to how the "distinct" search works now, but returning all
    // the details instead of just names.
    let query = match get_param(req, "query") {
        Some(q) => q,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let decoded_query = match url::percent_encoding::percent_decode(query.as_bytes()).decode_utf8()
    {
        Ok(q) => q.to_string(),
        Err(_) => return Ok(Response::with(status::BadRequest)),
    };

    match PackageIdent::from_str(decoded_query.as_ref()) {
        Ok(ident) => {
            request.set_origin(ident.origin().to_string());
            request.set_query(ident.name().to_string());
        }
        Err(_) => {
            request.set_query(decoded_query);
        }
    }

    debug!("search_packages called with: {}", request.get_query());

    // Setting distinct to true makes this query ignore any origin set, because it's going to
    // search both the origin name and the package name for the query string provided. This is
    // likely sub-optimal for performance but it makes things work right now and we should probably
    // switch to some kind of full-text search engine in the future anyway.
    // Also, to get this behavior, you need to ensure that "distinct" is a URL parameter in your
    // request, e.g. blah?distinct=true
    if helpers::extract_query_value("distinct", req).is_some() {
        request.set_distinct(true);
    }

    match route_message::<OriginPackageSearchRequest, OriginPackageListResponse>(req, &request) {
        Ok(packages) => {
            debug!(
                "search_packages start: {}, stop: {}, total count: {}",
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

fn render_package(
    req: &mut Request,
    pkg: &OriginPackage,
    should_cache: bool,
) -> IronResult<Response> {
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

fn promote_package(req: &mut Request) -> IronResult<Response> {
    let mut ident = OriginPackageIdent::new();
    match get_param(req, "origin") {
        Some(origin) => ident.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "pkg") {
        Some(pkg) => ident.set_name(pkg),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "version") {
        Some(version) => ident.set_version(version),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "release") {
        Some(release) => ident.set_release(release),
        None => return Ok(Response::with(status::BadRequest)),
    }
    let channel = match get_param(req, "channel") {
        Some(channel) => channel,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, ident.get_origin()).unwrap_or(false) {
        let err = NetError::new(ErrCode::ACCESS_DENIED, "core:promote-package-to-channel:0");
        return Ok(render_net_error(&err));
    }

    let mut origin_get = OriginGet::new();
    origin_get.set_name(ident.get_origin().to_string());

    let origin_id = match route_message::<OriginGet, Origin>(req, &origin_get) {
        Ok(o) => o.get_id(),
        Err(err) => return Ok(render_net_error(&err)),
    };

    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(ident.get_origin().to_string());
    channel_req.set_name(channel.to_string());

    let origin_channel = match route_message::<OriginChannelGet, OriginChannel>(req, &channel_req) {
        Ok(c) => c,
        Err(e) => {
            error!("Error retrieving channel: {:?}", &e);
            return Ok(render_net_error(&e));
        }
    };

    let mut request = OriginPackageGet::new();
    request.set_ident(ident.clone());
    request.set_visibilities(all_visibilities());

    let package = match route_message::<OriginPackageGet, OriginPackage>(req, &request) {
        Ok(p) => p,
        Err(e) => {
            error!("Error retrieving package: {:?}", &e);
            return Ok(render_net_error(&e));
        }
    };

    let mut promote = OriginPackagePromote::new();
    promote.set_channel_id(origin_channel.get_id());
    promote.set_package_id(package.get_id());
    promote.set_ident(ident.clone());

    match route_message::<OriginPackagePromote, NetOk>(req, &promote) {
        Ok(_) => match route_message::<OriginPackagePromote, NetOk>(req, &promote) {
            Ok(_) => match audit_package_rank_change(
                req,
                package.get_id(),
                origin_channel.get_id(),
                PackageChannelOperation::Promote,
                origin_id,
            ) {
                Ok(_) => Ok(Response::with(status::Ok)),
                Err(err) => return Ok(render_net_error(&err)),
            },
            Err(err) => return Ok(render_net_error(&err)),
        },
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn demote_package(req: &mut Request) -> IronResult<Response> {
    let mut ident = OriginPackageIdent::new();
    match get_param(req, "origin") {
        Some(origin) => ident.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "pkg") {
        Some(pkg) => ident.set_name(pkg),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "version") {
        Some(version) => ident.set_version(version),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "release") {
        Some(release) => ident.set_release(release),
        None => return Ok(Response::with(status::BadRequest)),
    }
    let channel = match get_param(req, "channel") {
        Some(channel) => channel,
        None => return Ok(Response::with(status::BadRequest)),
    };

    // you can't demote from "unstable"
    if channel == "unstable" {
        return Ok(Response::with(status::Forbidden));
    }

    if !check_origin_access(req, &ident.get_origin()).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut origin_get = OriginGet::new();
    origin_get.set_name(ident.get_origin().to_string());

    let origin_id = match route_message::<OriginGet, Origin>(req, &origin_get) {
        Ok(o) => o.get_id(),
        Err(err) => return Ok(render_net_error(&err)),
    };

    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(ident.get_origin().to_string());
    channel_req.set_name(channel);
    match route_message::<OriginChannelGet, OriginChannel>(req, &channel_req) {
        Ok(origin_channel) => {
            let mut request = OriginPackageGet::new();
            request.set_ident(ident.clone());
            request.set_visibilities(all_visibilities());
            match route_message::<OriginPackageGet, OriginPackage>(req, &request) {
                Ok(package) => {
                    let mut demote = OriginPackageDemote::new();
                    demote.set_channel_id(origin_channel.get_id());
                    demote.set_package_id(package.get_id());
                    demote.set_ident(ident.clone());
                    match route_message::<OriginPackageDemote, NetOk>(req, &demote) {
                        Ok(_) => match audit_package_rank_change(
                            req,
                            package.get_id(),
                            origin_channel.get_id(),
                            PackageChannelOperation::Demote,
                            origin_id,
                        ) {
                            Ok(_) => Ok(Response::with(status::Ok)),
                            Err(err) => return Ok(render_net_error(&err)),
                        },
                        Err(err) => return Ok(render_net_error(&err)),
                    }
                }
                Err(err) => Ok(render_net_error(&err)),
            }
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn create_origin_secret(req: &mut Request) -> IronResult<Response> {
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

pub fn list_origin_secrets(req: &mut Request) -> IronResult<Response> {
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

fn delete_origin_secret(req: &mut Request) -> IronResult<Response> {
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

fn ident_from_req(req: &mut Request) -> OriginPackageIdent {
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

fn download_content_as_file(content: &[u8], filename: String) -> IronResult<Response> {
    let mut response = Response::with((status::Ok, content));
    response.headers.set(ContentDisposition(format!(
        "attachment; filename=\"{}\"",
        filename
    )));
    response.headers.set(XFileName(filename));
    dont_cache_response(&mut response);
    Ok(response)
}

fn target_from_req(req: &mut Request) -> PackageTarget {
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

fn check_circular_deps(
    req: &mut Request,
    ident: &OriginPackageIdent,
    target: &PackageTarget,
    archive: &mut PackageArchive,
) -> result::Result<(), Response> {
    let mut pcr_req = JobGraphPackagePreCreate::new();
    pcr_req.set_ident(format!("{}", ident));
    pcr_req.set_target(target.to_string());

    let mut pcr_deps = protobuf::RepeatedField::new();
    let deps_from_artifact = match archive.deps() {
        Ok(deps) => deps,
        Err(e) => {
            info!("Could not get deps from {:#?}: {:#?}", archive, e);
            return Err(Response::with((status::UnprocessableEntity, "ds:up:4")));
        }
    };

    for ident in deps_from_artifact {
        let dep_str = format!("{}", ident);
        pcr_deps.push(dep_str);
    }
    pcr_req.set_deps(pcr_deps);

    match route_message::<JobGraphPackagePreCreate, NetOk>(req, &pcr_req) {
        Ok(_) => Ok(()),
        Err(err) => {
            if err.get_code() == ErrCode::ENTITY_CONFLICT {
                warn!(
                    "Failed package circular dependency check: {}, err: {:?}",
                    ident, err
                );
                return Err(Response::with(status::FailedDependency));
            }
            return Err(render_net_error(&err));
        }
    }
}

fn process_upload_for_package_archive(
    ident: &OriginPackageIdent,
    package: &mut OriginPackageCreate,
    target: &PackageTarget,
    owner_id: u64,
    owner_name: String,
    origin_package_found: bool,
    builder_flag: Option<String>,
) -> NetResult<()> {
    // We need to do it this way instead of via route_message because this function can't be passed
    // a Request struct, because it's sometimes called from a background thread, and Request
    // structs are not cloneable.
    let mut conn = RouteBroker::connect().unwrap();

    package.set_owner_id(owner_id);

    // Let's make sure this origin actually exists. Yes, I know we have a helper function for this
    // but it requires the Request struct, which is not available here.
    let mut request = OriginGet::new();
    request.set_name(ident.get_origin().to_string());
    match conn.route::<OriginGet, Origin>(&request) {
        Ok(origin) => package.set_origin_id(origin.get_id()),
        Err(err) => return Err(err),
    }

    // Zero this out initially
    package.clear_visibility();

    // First, try to fetch visibility settings from a project, if one exists
    let mut project_get = OriginProjectGet::new();
    let project_name = format!("{}/{}", ident.get_origin(), ident.get_name());
    project_get.set_name(project_name);

    match conn.route::<OriginProjectGet, OriginProject>(&project_get) {
        Ok(proj) => {
            if proj.has_visibility() {
                package.set_visibility(proj.get_visibility());
            }
        }
        Err(_) => {
            // There's no project for this package. No worries - we'll check the origin
            let mut origin_get = OriginGet::new();
            origin_get.set_name(ident.get_origin().to_string());

            match conn.route::<OriginGet, Origin>(&origin_get) {
                Ok(o) => {
                    if o.has_default_package_visibility() {
                        package.set_visibility(o.get_default_package_visibility());
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    // If, after checking both the project and the origin, there's still no visibility set
    // (this is highly unlikely), then just make it public.
    if !package.has_visibility() {
        package.set_visibility(OriginPackageVisibility::Public);
    }

    // Don't re-create the origin package if it already exists
    if !origin_package_found {
        if let Err(err) = conn.route::<OriginPackageCreate, OriginPackage>(&package) {
            return Err(err);
        }

        // Schedule re-build of dependent packages (if requested)
        // Don't schedule builds if the upload is being done by the builder
        if builder_flag.is_none() && feat::is_enabled(feat::Jobsrv) {
            let mut request = JobGroupSpec::new();
            request.set_origin(ident.get_origin().to_string());
            request.set_package(ident.get_name().to_string());
            request.set_target(target.to_string());
            request.set_deps_only(true);
            request.set_origin_only(false);
            request.set_package_only(false);
            request.set_trigger(JobGroupTrigger::Upload);
            request.set_requester_id(owner_id);
            request.set_requester_name(owner_name);

            match conn.route::<JobGroupSpec, JobGroup>(&request) {
                Ok(group) => debug!(
                    "Scheduled reverse dependecy build for {}, group id: {}, origin_only: {}",
                    ident,
                    group.get_id(),
                    false
                ),
                Err(err) => warn!("Unable to schedule build, err: {:?}", err),
            }
        }
    }

    Ok(())
}

// This function is called from a background thread, so we can't pass the Request object into it.
// TBD: Move this to upstream module and refactor later
pub fn download_package_from_upstream_depot(
    depot: &Config,
    depot_cli: &ApiClient,
    s3_handler: &s3::S3Handler,
    ident: OriginPackageIdent,
    channel: &str,
    target: &str,
) -> Result<OriginPackage> {
    let parent_path = depot.packages_path();

    match fs::create_dir_all(parent_path.clone()) {
        Ok(_) => {}
        Err(e) => {
            error!("Unable to create archive directory, err={:?}", e);
            return Err(Error::IO(e));
        }
    };

    match depot_cli.fetch_package(&ident, target, &parent_path, None) {
        Ok(mut archive) => {
            let target_from_artifact = archive.target().map_err(Error::HabitatCore)?;
            if !depot.api.targets.contains(&target_from_artifact) {
                debug!(
                    "Unsupported package platform or architecture {}.",
                    &target_from_artifact
                );
                return Err(Error::UnsupportedPlatform(target_from_artifact.to_string()));
            };

            let archive_path = parent_path.join(archive.file_name());

            s3_handler.upload(
                &archive_path,
                &PackageIdent::from(&ident),
                &target_from_artifact,
            )?;

            let mut package_create = match OriginPackageCreate::from_archive(&mut archive) {
                Ok(p) => p,
                Err(e) => {
                    info!("Error building package from archive: {:#?}", e);
                    return Err(Error::HabitatCore(e));
                }
            };

            if let Err(e) = process_upload_for_package_archive(
                &ident,
                &mut package_create,
                &target_from_artifact,
                BUILDER_ACCOUNT_ID,
                BUILDER_ACCOUNT_NAME.to_string(),
                false,
                None,
            ) {
                return Err(Error::NetError(e));
            }

            // We need to ensure that the new package is in the proper channels. Right now, this function
            // is only called when we need to fetch packages from an upstream depot, whether that's
            // in-band with a request, such as 'hab pkg install', or in a background thread. Either
            // way, though, its purpose is to make packages on our local depot here mirror what
            // they look like in the upstream.
            //
            // Given this, we need to ensure that packages fetched from this mechanism end up in
            // the stable channel, since that's where 'hab pkg install' tries to install them from.
            // It'd be a pretty jarring experience if someone did a 'hab pkg install' for
            // core/tree, and it succeeded the first time when it fetched it from the upstream
            // depot, and failed the second time from the local depot because it couldn't be found
            // in the stable channel.
            //
            // Creating and promoting to channels without the use of the Request struct is messy and will
            // require much refactoring of code, so at the moment, we're going to punt on the hard problem
            // here and just check to see if the channel is stable, and if so, do the right thing. If it's
            // not stable, we do nothing (though the odds of this happening are vanishingly small).
            if channel == "stable" {
                let mut conn = RouteBroker::connect().unwrap();
                let mut channel_get = OriginChannelGet::new();
                channel_get.set_origin_name(ident.get_origin().to_string());
                channel_get.set_name("stable".to_string());

                let origin_channel = conn
                    .route::<OriginChannelGet, OriginChannel>(&channel_get)
                    .map_err(Error::NetError)?;

                let mut package_get = OriginPackageGet::new();
                package_get.set_ident(ident.clone());
                package_get.set_visibilities(all_visibilities());

                let origin_package = conn
                    .route::<OriginPackageGet, OriginPackage>(&package_get)
                    .map_err(Error::NetError)?;

                let mut promote = OriginPackagePromote::new();
                promote.set_channel_id(origin_channel.get_id());
                promote.set_package_id(origin_package.get_id());
                promote.set_ident(ident);

                match conn.route::<OriginPackagePromote, NetOk>(&promote) {
                    Ok(_) => Ok(origin_package),
                    Err(e) => Err(Error::NetError(e)),
                }
            } else {
                warn!(
                        "Installing packages from an upstream depot and the channel wasn't stable. Instead, it was {}",
                        channel
                    );

                match OriginPackage::from_archive(&mut archive) {
                    Ok(p) => Ok(p),
                    Err(e) => Err(Error::HabitatCore(e)),
                }
            }
        }
        Err(e) => {
            warn!("Failed to download {}. e = {:?}", ident, e);
            Err(Error::DepotClientError(e))
        }
    }
}

fn notify_upstream(req: &mut Request, ident: &OriginPackageIdent, target: &PackageTarget) {
    let upstream_cli = req.get::<persistent::Read<UpstreamCli>>().unwrap();
    upstream_cli.refresh(ident, target).unwrap();
}

struct TempDownloadPath;

impl typemap::Key for TempDownloadPath {
    type Value = TempDir;
}

fn download_response_for_archive(
    archive: PackageArchive,
    tempdir: TempDir,
) -> IronResult<Response> {
    let mut response = Response::with((status::Ok, archive.path.clone()));
    // Yo. This is some serious iron black magic. This is how we can get
    // appropriate timing of the .drop of the tempdir to fire AFTER the
    // response is finished being written
    response.extensions.insert::<TempDownloadPath>(tempdir);
    do_cache_response(&mut response);
    let disp = ContentDisposition {
        disposition: DispositionType::Attachment,
        parameters: vec![DispositionParam::Filename(
            Charset::Iso_8859_1,
            None,
            archive.file_name().as_bytes().to_vec(),
        )],
    };
    response.headers.set(disp);
    response.headers.set(XFileName(archive.file_name()));
    Ok(response)
}

fn audit_package_rank_change(
    req: &mut Request,
    package_id: u64,
    channel_id: u64,
    operation: PackageChannelOperation,
    origin_id: u64,
) -> NetResult<NetOk> {
    let mut audit = PackageChannelAudit::new();
    audit.set_package_id(package_id);
    audit.set_channel_id(channel_id);
    audit.set_operation(operation);

    let jgt = trigger_from_request(req);
    audit.set_trigger(PackageChannelTrigger::from(jgt));

    let (session_id, session_name) = get_session_id_and_name(req);

    audit.set_requester_id(session_id);
    audit.set_requester_name(session_name);
    audit.set_origin_id(origin_id);

    route_message::<PackageChannelAudit, NetOk>(req, &audit)
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

pub fn add_routes<M>(r: &mut Router, basic: Authenticated, worker: M)
where
    M: BeforeMiddleware + Clone,
{
    let opt = basic.clone().optional();

    if feat::is_enabled(feat::Jobsrv) {
        r.get(
            "/pkgs/origins/:origin/stats",
            package_stats,
            "package_stats",
        );
        r.post(
            "/pkgs/schedule/:origin/:pkg",
            XHandler::new(schedule).before(basic.clone()),
            "schedule",
        );
        r.get("/pkgs/schedule/:groupid", get_schedule, "schedule_get");
        r.get(
            "/pkgs/schedule/:origin/status",
            get_origin_schedule_status,
            "schedule_get_global",
        );
        r.delete(
            "/pkgs/schedule/:groupid",
            XHandler::new(abort_schedule).before(worker.clone()),
            "schedule_abort",
        );
    }

    r.get("/channels/:origin", list_channels, "channels");
    r.get(
        "/channels/:origin/:channel/pkgs",
        XHandler::new(list_packages).before(opt.clone()),
        "channel_packages",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg",
        XHandler::new(list_packages).before(opt.clone()),
        "channel_packages_pkg",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg/latest",
        XHandler::new(show_package).before(opt.clone()),
        "channel_package_latest",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg/:version",
        XHandler::new(list_packages).before(opt.clone()),
        "channel_packages_version",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg/:version/latest",
        XHandler::new(show_package).before(opt.clone()),
        "channel_packages_version_latest",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg/:version/:release",
        XHandler::new(show_package).before(opt.clone()),
        "channel_package_release",
    );
    r.put(
        "/channels/:origin/:channel/pkgs/:pkg/:version/:release/promote",
        XHandler::new(promote_package).before(basic.clone()),
        "channel_package_promote",
    );
    r.put(
        "/channels/:origin/:channel/pkgs/:pkg/:version/:release/demote",
        XHandler::new(demote_package).before(basic.clone()),
        "channel_package_demote",
    );
    r.post(
        "/channels/:origin/:channel",
        XHandler::new(create_channel).before(basic.clone()),
        "channel_create",
    );
    r.delete(
        "/channels/:origin/:channel",
        XHandler::new(delete_channel).before(basic.clone()),
        "channel_delete",
    );
    r.get(
        "/pkgs/search/:query",
        XHandler::new(search_packages).before(opt.clone()),
        "package_search",
    );
    r.get(
        "/pkgs/:origin",
        XHandler::new(list_packages).before(opt.clone()),
        "packages",
    );
    r.get(
        "/:origin/pkgs",
        XHandler::new(list_unique_packages).before(opt.clone()),
        "packages_unique",
    );
    r.get(
        "/pkgs/:origin/:pkg",
        XHandler::new(list_packages).before(opt.clone()),
        "packages_pkg",
    );
    r.get(
        "/pkgs/:origin/:pkg/versions",
        XHandler::new(list_package_versions).before(opt.clone()),
        "package_pkg_versions",
    );
    r.get(
        "/pkgs/:origin/:pkg/latest",
        XHandler::new(show_package).before(opt.clone()),
        "package_pkg_latest",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version",
        XHandler::new(list_packages).before(opt.clone()),
        "packages_version",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version/latest",
        XHandler::new(show_package).before(opt.clone()),
        "package_version_latest",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version/:release",
        XHandler::new(show_package).before(opt.clone()),
        "package",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version/:release/channels",
        XHandler::new(package_channels).before(opt.clone()),
        "package_channels",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version/:release/download",
        XHandler::new(download_package).before(opt.clone()),
        "package_download",
    );
    r.post(
        "/pkgs/:origin/:pkg/:version/:release",
        XHandler::new(upload_package).before(basic.clone()),
        "package_upload",
    );
    r.patch(
        "/pkgs/:origin/:pkg/:version/:release/:visibility",
        XHandler::new(package_privacy_toggle).before(basic.clone()),
        "package_privacy_toggle",
    );
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
        "/origins/:origin/keys/:revision",
        XHandler::new(upload_origin_key).before(basic.clone()),
        "origin_key_create",
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
}

pub fn router(depot: Arc<Config>) -> Router {
    let basic = Authenticated::new(depot.api.key_path.clone());
    let worker = Authenticated::new(depot.api.key_path.clone()).require(FeatureFlags::BUILD_WORKER);

    let mut r = Router::new();
    add_routes(&mut r, basic, worker);

    r
}
