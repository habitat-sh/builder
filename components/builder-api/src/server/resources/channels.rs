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

use std::str::FromStr;

use actix_web::http::{self, Method, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Path, Query};
use serde_json;

use hab_core::package::{Identifiable, PackageTarget};
use hab_net::NetOk;
use protocol::originsrv::*;

use super::pkgs::{notify_upstream, postprocess_package, postprocess_package_list};
use server::authorize::{authorize_session, get_session_user_name};
use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers::{self, Pagination, Target};
use server::AppState;

// Query param containers
#[derive(Debug, Default, Clone, Deserialize)]
struct SandboxBool {
    #[serde(default)]
    is_set: bool,
}

pub struct Channels;

impl Channels {
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route("/depot/channels/{origin}", Method::GET, get_channels)
            .route(
                "/depot/channels/{origin}/{channel}",
                Method::POST,
                create_channel,
            )
            .route(
                "/depot/channels/{origin}/{channel}",
                Method::DELETE,
                delete_channel,
            )
            .route(
                "/depot/channels/{origin}/{channel}/pkgs",
                Method::GET,
                get_packages_for_origin_channel,
            )
            .route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}",
                Method::GET,
                get_packages_for_origin_channel_package,
            )
            .route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/latest",
                Method::GET,
                get_latest_package_for_origin_channel_package,
            )
            .route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}",
                Method::GET,
                get_packages_for_origin_channel_package_version,
            )
            .route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/latest",
                Method::GET,
                get_latest_package_for_origin_channel_package_version,
            )
            .route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}",
                Method::GET,
                get_package_fully_qualified,
            )
            .route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}/promote",
                Method::PUT,
                promote_package,
            )
            .route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}/demote",
                Method::PUT,
                demote_package,
            )
    }
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_channels((req, sandbox): (HttpRequest<AppState>, Query<SandboxBool>)) -> HttpResponse {
    let origin = Path::<(String)>::extract(&req).unwrap().into_inner();

    // Pass ?sandbox=true to this endpoint to include sandbox channels in the list. They are not
    // there by default.
    let mut request = OriginChannelListRequest::new();
    request.set_include_sandbox_channels(sandbox.is_set);

    match helpers::get_origin(&req, &origin) {
        Ok(orgn) => request.set_origin_id(orgn.get_id()),
        Err(err) => return err.into(),
    }

    match route_message::<OriginChannelListRequest, OriginChannelListResponse>(&req, &request) {
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
            let mut response = HttpResponse::Ok();
            response
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .body(body)
        }
        Err(err) => return err.into(),
    }
}

fn create_channel(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner();

    match helpers::create_channel(&req, &origin, &channel) {
        Ok(origin_channel) => HttpResponse::Created().json(&origin_channel),
        Err(err) => err.into(),
    }
}

fn delete_channel(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner();

    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(origin.clone());
    channel_req.set_name(channel.clone());
    match route_message::<OriginChannelGet, OriginChannel>(&req, &channel_req) {
        Ok(origin_channel) => {
            if let Err(err) = authorize_session(&req, Some(&origin)) {
                return err.into();
            }

            if channel == "stable" || channel == "unstable" {
                return HttpResponse::new(StatusCode::FORBIDDEN);
            }

            let mut delete = OriginChannelDelete::new();
            delete.set_id(origin_channel.get_id());
            delete.set_origin_id(origin_channel.get_origin_id());
            match route_message::<OriginChannelDelete, NetOk>(&req, &delete) {
                Ok(_) => HttpResponse::new(StatusCode::OK),
                Err(err) => err.into(),
            }
        }
        Err(err) => err.into(),
    }
}

fn promote_package(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner();

    match do_promote_package(&req, origin, channel, pkg, version, release) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => err.into(),
    }
}

fn demote_package(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner();

    match do_demote_package(&req, origin, channel, pkg, version, release) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => err.into(),
    }
}

fn get_packages_for_origin_channel_package_version(
    (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg, version) = Path::<(String, String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(pkg);
    ident.set_version(version);

    match do_get_channel_packages(&req, &pagination, ident, channel) {
        Ok(olpr) => postprocess_package_list(&req, &olpr, pagination.distinct),
        Err(err) => err.into(),
    }
}

fn get_packages_for_origin_channel_package(
    (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(pkg);

    match do_get_channel_packages(&req, &pagination, ident, channel) {
        Ok(olpr) => postprocess_package_list(&req, &olpr, pagination.distinct),
        Err(err) => err.into(),
    }
}

fn get_packages_for_origin_channel(
    (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);

    match do_get_channel_packages(&req, &pagination, ident, channel) {
        Ok(olpr) => postprocess_package_list(&req, &olpr, pagination.distinct),
        Err(err) => err.into(),
    }
}

fn get_latest_package_for_origin_channel_package(
    (qtarget, req): (Query<Target>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(pkg);

    match do_get_channel_package(&req, &qtarget, ident, channel) {
        Ok(package) => postprocess_package(&req, &package, false),
        Err(err) => err.into(),
    }
}

fn get_latest_package_for_origin_channel_package_version(
    (qtarget, req): (Query<Target>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg, version) = Path::<(String, String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(pkg);
    ident.set_version(version);

    match do_get_channel_package(&req, &qtarget, ident, channel) {
        Ok(package) => postprocess_package(&req, &package, false),
        Err(err) => err.into(),
    }
}

fn get_package_fully_qualified(
    (qtarget, req): (Query<Target>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(pkg);
    ident.set_version(version);
    ident.set_release(release);

    match do_get_channel_package(&req, &qtarget, ident, channel) {
        Ok(package) => postprocess_package(&req, &package, false),
        Err(err) => err.into(),
    }
}

//
// Internal - these functions should return Result<..>
//
fn do_promote_package(
    req: &HttpRequest<AppState>,
    origin: String,
    channel: String,
    pkg: String,
    version: String,
    release: String,
) -> Result<()> {
    let session_id = authorize_session(req, Some(&origin))?;

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(pkg);
    ident.set_version(version);
    ident.set_release(release);

    let mut origin_get = OriginGet::new();
    origin_get.set_name(ident.get_origin().to_string());

    let origin_id = match route_message::<OriginGet, Origin>(req, &origin_get) {
        Ok(o) => o.get_id(),
        Err(err) => return Err(err),
    };

    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(ident.get_origin().to_string());
    channel_req.set_name(channel.to_string());

    let origin_channel = match route_message::<OriginChannelGet, OriginChannel>(req, &channel_req) {
        Ok(c) => c,
        Err(e) => {
            warn!("Error retrieving channel: {:?}", &e);
            return Err(e);
        }
    };

    let mut request = OriginPackageGet::new();
    request.set_ident(ident.clone());
    request.set_visibilities(helpers::all_visibilities());

    let package = match route_message::<OriginPackageGet, OriginPackage>(req, &request) {
        Ok(p) => p,
        Err(e) => {
            warn!("Error retrieving package {:?}", &e);
            return Err(e);
        }
    };

    let mut promote = OriginPackagePromote::new();
    promote.set_channel_id(origin_channel.get_id());
    promote.set_package_id(package.get_id());
    promote.set_ident(ident.clone());

    match route_message::<OriginPackagePromote, NetOk>(req, &promote) {
        Ok(_) => match audit_package_rank_change(
            req,
            package.get_id(),
            origin_channel.get_id(),
            PackageChannelOperation::Promote,
            origin_id,
            session_id,
        ) {
            Ok(_) => return Ok(()),
            Err(err) => return Err(err),
        },
        Err(err) => return Err(err),
    }
}

fn do_demote_package(
    req: &HttpRequest<AppState>,
    origin: String,
    channel: String,
    pkg: String,
    version: String,
    release: String,
) -> Result<()> {
    let session_id = authorize_session(req, Some(&origin))?;

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(pkg);
    ident.set_version(version);
    ident.set_release(release);

    if channel == "unstable" {
        return Err(Error::Authorization);
    }

    let mut origin_get = OriginGet::new();
    origin_get.set_name(ident.get_origin().to_string());

    let origin_id = match route_message::<OriginGet, Origin>(req, &origin_get) {
        Ok(o) => o.get_id(),
        Err(e) => return Err(e),
    };

    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(ident.get_origin().to_string());
    channel_req.set_name(channel);

    match route_message::<OriginChannelGet, OriginChannel>(req, &channel_req) {
        Ok(origin_channel) => {
            let mut request = OriginPackageGet::new();
            request.set_ident(ident.clone());
            request.set_visibilities(helpers::all_visibilities());
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
                            session_id,
                        ) {
                            Ok(_) => return Ok(()),
                            Err(err) => return Err(err.into()),
                        },
                        Err(err) => return Err(err.into()),
                    }
                }
                Err(err) => return Err(err.into()),
            }
        }
        Err(err) => return Err(err.into()),
    }
}

fn audit_package_rank_change(
    req: &HttpRequest<AppState>,
    package_id: u64,
    channel_id: u64,
    operation: PackageChannelOperation,
    origin_id: u64,
    session_id: u64,
) -> Result<NetOk> {
    let mut audit = PackageChannelAudit::new();
    audit.set_package_id(package_id);
    audit.set_channel_id(channel_id);
    audit.set_operation(operation);

    let jgt = helpers::trigger_from_request(req);
    audit.set_trigger(PackageChannelTrigger::from(jgt));

    let session_name = get_session_user_name(req, session_id);

    audit.set_requester_id(session_id);
    audit.set_requester_name(session_name);
    audit.set_origin_id(origin_id);

    route_message::<PackageChannelAudit, NetOk>(req, &audit)
}

fn do_get_channel_packages(
    req: &HttpRequest<AppState>,
    pagination: &Query<Pagination>,
    ident: OriginPackageIdent,
    channel: String,
) -> Result<OriginPackageListResponse> {
    let opt_session_id = match authorize_session(&req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let (start, stop) = helpers::extract_pagination(pagination);

    let mut request = OriginChannelPackageListRequest::new();
    request.set_name(channel);
    request.set_start(start as u64);
    request.set_stop(stop as u64);
    request.set_visibilities(helpers::visibility_for_optional_session(
        &req,
        opt_session_id,
        &ident.get_origin(),
    ));

    request.set_ident(ident);
    route_message::<OriginChannelPackageListRequest, OriginPackageListResponse>(req, &request)
}

fn do_get_channel_package(
    req: &HttpRequest<AppState>,
    qtarget: &Query<Target>,
    mut ident: OriginPackageIdent,
    channel: String,
) -> Result<OriginPackage> {
    let opt_session_id = match authorize_session(req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    // TODO: Deprecate target from headers
    let target = match qtarget.target.clone() {
        Some(t) => {
            debug!("Query requested target = {}", t);
            PackageTarget::from_str(&t).unwrap() // Unwrap Ok ?
        }
        None => helpers::target_from_headers(req),
    };

    // Fully qualify the ident if needed
    // TODO: Have the OriginPackageLatestGet call just return the package
    // metadata, thus saving us a second call to actually retrieve the package
    if !ident.fully_qualified() {
        let mut request = OriginChannelPackageLatestGet::new();
        request.set_name(channel.clone());
        request.set_target(target.to_string());
        request.set_visibilities(helpers::visibility_for_optional_session(
            req,
            opt_session_id,
            &ident.get_origin(),
        ));
        request.set_ident(ident.clone());

        ident = match route_message::<OriginChannelPackageLatestGet, OriginPackageIdent>(
            req, &request,
        ) {
            Ok(id) => id.into(),
            Err(err) => {
                // Notify upstream with a non-fully qualified ident to handle checking
                // of a package that does not exist in the on-premise depot

                notify_upstream(req, &ident, &target);
                return Err(err);
            }
        };
    }

    let mut request = OriginPackageGet::new();
    request.set_visibilities(helpers::visibility_for_optional_session(
        req,
        opt_session_id,
        &ident.get_origin(),
    ));
    request.set_ident(ident.clone());

    // Notify upstream with a fully qualified ident
    notify_upstream(req, &ident, &target);

    route_message::<OriginPackageGet, OriginPackage>(req, &request)
}
