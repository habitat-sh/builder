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
use actix_web::{error, App, HttpRequest, HttpResponse, Path, Query};
use actix_web::{AsyncResponder, FromRequest, FutureResponse};
use futures::{future, Future};

use bldr_core::metrics::CounterMetric;
use hab_core::package::{Identifiable, PackageTarget};
use hab_net::NetOk;
use protocol::originsrv::*;
use serde_json;

use super::pkgs::{is_a_service, notify_upstream, postprocess_package_list};
use hab_core::package::PackageIdent;
use server::authorize::{authorize_session, get_session_user_name};
use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers::{self, Pagination, Target};
use server::models::channel::{
    AuditPackageRankChange, CreateChannel, DeleteChannel, DemotePackage, ListChannels,
    PackageChannelOperation, PromotePackage,
};
use server::services::metrics::Counter;
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
            ).route(
                "/depot/channels/{origin}/{channel}",
                Method::DELETE,
                delete_channel,
            ).route(
                "/depot/channels/{origin}/{channel}/pkgs",
                Method::GET,
                get_packages_for_origin_channel,
            ).route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}",
                Method::GET,
                get_packages_for_origin_channel_package,
            ).route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/latest",
                Method::GET,
                get_latest_package_for_origin_channel_package,
            ).route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}",
                Method::GET,
                get_packages_for_origin_channel_package_version,
            ).route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/latest",
                Method::GET,
                get_latest_package_for_origin_channel_package_version,
            ).route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}",
                Method::GET,
                get_package_fully_qualified,
            ).route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}/promote",
                Method::PUT,
                promote_package,
            ).route(
                "/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}/demote",
                Method::PUT,
                demote_package,
            )
    }
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_channels(
    (req, sandbox): (HttpRequest<AppState>, Query<SandboxBool>),
) -> FutureResponse<HttpResponse> {
    let origin = Path::<(String)>::extract(&req).unwrap().into_inner();

    req.state()
        .db
        .send(ListChannels {
            origin: origin,
            include_sandbox_channels: sandbox.is_set,
        }).from_err()
        .and_then(|res| match res {
            Ok(list) => {
                // TED: This is to maintain backwards API compat while killing some proto definitions
                // currently the output looks like [{"name": "foo"}] when it probably should be ["foo"]
                #[derive(Serialize)]
                struct Temp {
                    name: String,
                }
                let ident_list: Vec<Temp> = list
                    .iter()
                    .map(|channel| Temp {
                        name: channel.name.clone(),
                    }).collect();
                Ok(HttpResponse::Ok()
                    .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                    .json(ident_list))
            }
            Err(_err) => Ok(HttpResponse::InternalServerError().into()),
        }).responder()
}

fn create_channel(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner();

    let session_id = match authorize_session(&req, Some(&origin)) {
        Ok(session_id) => session_id as i64,
        Err(e) => return future::err(error::ErrorUnauthorized(e)).responder(),
    };

    req.state()
        .db
        .send(CreateChannel {
            channel: channel,
            origin: origin,
            owner_id: session_id,
        }).from_err()
        .and_then(|res| match res {
            Ok(channel) => Ok(HttpResponse::Created().json(channel)),
            Err(e) => Err(e),
        }).responder()
}

fn delete_channel(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return future::err(error::ErrorUnauthorized(err)).responder();
    }

    if channel == "stable" || channel == "unstable" {
        return future::err(error::ErrorForbidden(format!("{} is protected", channel))).responder();
    }

    req.state()
        .memcache
        .borrow_mut()
        .clear_cache_for_channel(&origin, &channel);

    req.state()
        .db
        .send(DeleteChannel {
            origin: origin,
            channel: channel,
        }).from_err()
        .and_then(|res| match res {
            Ok(_) => Ok(HttpResponse::new(StatusCode::OK)),
            Err(e) => Err(e),
        }).responder()
}

fn promote_package(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner();
    let session_id = match authorize_session(&req, Some(&origin)) {
        Ok(session_id) => session_id as i64,
        Err(e) => return future::err(error::ErrorUnauthorized(e)).responder(),
    };
    let ident = PackageIdent::new(
        origin.clone(),
        pkg.clone(),
        Some(version.clone()),
        Some(release.clone()),
    );
    req.state()
        .db
        .send(PromotePackage {
            ident: ident.clone(),
            origin: origin.clone(),
            channel: channel.clone(),
        }).from_err()
        .and_then(move |res| match res {
            Ok(_) => {
                audit_package_rank_change(
                    &req,
                    ident.clone(),
                    channel,
                    PackageChannelOperation::Promote,
                    origin,
                    session_id,
                );
                Ok(HttpResponse::new(StatusCode::OK))
            }
            Err(e) => Err(e),
        }).responder()
}

fn demote_package(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner();
    if channel == "unstable" {
        return future::err(error::ErrorUnauthorized("Can't demote from unstable")).responder();
    }
    let ident = PackageIdent::new(
        origin.clone(),
        pkg.clone(),
        Some(version.clone()),
        Some(release.clone()),
    );
    let session_id = match authorize_session(&req, Some(&origin)) {
        Ok(session_id) => session_id as i64,
        Err(e) => return future::err(error::ErrorUnauthorized(e)).responder(),
    };
    req.state()
        .db
        .send(DemotePackage {
            ident: ident.clone(),
            origin: origin.clone(),
            channel: channel.clone(),
        }).from_err()
        .and_then(move |res| match res {
            Ok(_) => {
                audit_package_rank_change(
                    &req,
                    ident.clone(),
                    channel,
                    PackageChannelOperation::Demote,
                    origin,
                    session_id,
                );
                req.state()
                    .memcache
                    .borrow_mut()
                    .clear_cache_for_package(ident.clone().into());
                Ok(HttpResponse::new(StatusCode::OK))
            }
            Err(err) => Err(err),
        }).responder()
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
        Ok(json_body) => HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .header(http::header::CACHE_CONTROL, headers::cache(false))
            .body(json_body),
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
        Ok(json_body) => HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .header(http::header::CACHE_CONTROL, headers::cache(false))
            .body(json_body),
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
        Ok(json_body) => HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .header(http::header::CACHE_CONTROL, headers::cache(false))
            .body(json_body),
        Err(err) => err.into(),
    }
}

//
// Internal - these functions should return Result<..>
//

fn audit_package_rank_change(
    req: &HttpRequest<AppState>,
    ident: PackageIdent,
    channel: String,
    operation: PackageChannelOperation,
    origin: String,
    session_id: i64,
) {
    req.state().db.do_send(AuditPackageRankChange {
        ident,
        channel,
        operation,
        origin,
        session_id,
    })
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
) -> Result<String> {
    let opt_session_id = match authorize_session(req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };
    Counter::GetChannelPackage.increment();

    let mut memcache = req.state().memcache.borrow_mut();
    let req_ident = ident.clone();

    // TODO: Deprecate target from headers
    let target = match qtarget.target.clone() {
        Some(t) => {
            debug!("Query requested target = {}", t);
            PackageTarget::from_str(&t)?
        }
        None => helpers::target_from_headers(req),
    };

    match memcache.get_package(req_ident.clone().into(), &channel, &target) {
        Some(pkg_json) => {
            trace!("Cache Hit!");
            return Ok(pkg_json);
        }
        None => {
            trace!("Cache Miss!");
        }
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

        ident =
            match route_message::<OriginChannelPackageLatestGet, OriginPackageIdent>(req, &request)
            {
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

    let pkg = match route_message::<OriginPackageGet, OriginPackage>(req, &request) {
        Ok(pkg) => {
            // Notify upstream with a fully qualified ident
            notify_upstream(req, &ident, &(PackageTarget::from_str(&pkg.get_target())?));
            pkg
        }
        Err(err) => return Err(err),
    };

    let mut pkg_json = serde_json::to_value(pkg.clone()).unwrap();
    let channels = helpers::channels_for_package_ident(req, pkg.get_ident());
    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(is_a_service(&pkg));

    let json_body = serde_json::to_string(&pkg_json).unwrap();
    memcache.set_package(req_ident.clone().into(), &json_body, &channel, &target);

    Ok(json_body)
}
