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
use diesel::pg::PgConnection;
use diesel::result::{DatabaseErrorKind, Error::DatabaseError, Error::NotFound};

use std::ops::Deref;

use bldr_core::metrics::CounterMetric;
use hab_core::package::{Identifiable, PackageIdent, PackageTarget};
use hab_net::{ErrCode, NetError};
use protocol::originsrv::*;
use serde_json;

use super::pkgs::{is_a_service, postprocess_package_list};
use server::authorize::{authorize_session, get_session_user_name};
use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers::{self, Pagination, Target};
// TED PROTOCLEANUP remove aliases when we get rid of all the protos
use db::models::channel::{
    Channel, CreateChannel, DeleteChannel, GetLatestPackage, ListChannels,
    OriginChannelDemote as OCD, OriginChannelPackage as OCPackage, OriginChannelPromote as OCP,
    PackageChannelAudit as PCA, PackageChannelAudit, PackageChannelOperation as PCO,
};
use db::models::package::{BuilderPackageIdent, GetPackage, Package};
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
fn get_channels((req, sandbox): (HttpRequest<AppState>, Query<SandboxBool>)) -> HttpResponse {
    let origin = Path::<(String)>::extract(&req).unwrap().into_inner();

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    match Channel::list(
        ListChannels {
            origin: origin,
            include_sandbox_channels: sandbox.is_set,
        },
        conn.deref(),
    ) {
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
            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(ident_list)
        }
        Err(_err) => HttpResponse::InternalServerError().into(),
    }
}

fn create_channel(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner();

    let session_id = match authorize_session(&req, Some(&origin)) {
        Ok(session_id) => session_id as i64,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    match Channel::create(
        CreateChannel {
            channel: channel,
            origin: origin,
            owner_id: session_id,
        },
        conn.deref(),
    ) {
        Ok(channel) => HttpResponse::Created().json(channel),
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            HttpResponse::Conflict().into()
        }
        Err(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn delete_channel(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner();

    if let Err(_err) = authorize_session(&req, Some(&origin)) {
        return HttpResponse::new(StatusCode::UNAUTHORIZED);
    }

    if channel == "stable" || channel == "unstable" {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    req.state()
        .memcache
        .borrow_mut()
        .clear_cache_for_channel(&origin, &channel);

    match Channel::delete(
        DeleteChannel {
            origin: origin,
            channel: channel,
        },
        conn.deref(),
    ) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn promote_package(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner();

    let session_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let ident = PackageIdent::new(
        origin.clone(),
        pkg.clone(),
        Some(version.clone()),
        Some(release.clone()),
    );

    match OCPackage::promote(
        OCP {
            ident: BuilderPackageIdent(ident.clone()),
            origin: origin.clone(),
            channel: channel.clone(),
        },
        &*conn,
    ) {
        Ok(_) => match audit_package_rank_change(
            &req,
            &*conn,
            ident,
            channel,
            PCO::Promote,
            origin,
            session_id,
        ) {
            Ok(_) => HttpResponse::new(StatusCode::OK),
            Err(err) => {
                warn!("Failed to save rank change to audit log: {}", err);
                HttpResponse::new(StatusCode::OK)
            }
        },
        Err(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn demote_package(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner();

    if channel == "unstable" {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let ident = PackageIdent::new(
        origin.clone(),
        pkg.clone(),
        Some(version.clone()),
        Some(release.clone()),
    );
    let session_id = match authorize_session(&req, Some(&origin)) {
        Ok(session_id) => session_id,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };
    match OCPackage::demote(
        OCD {
            ident: BuilderPackageIdent(ident.clone()),
            origin: origin.clone(),
            channel: channel.clone(),
        },
        &*conn,
    ) {
        Ok(_) => {
            match audit_package_rank_change(
                &req,
                &conn,
                ident.clone(),
                channel,
                PCO::Demote,
                origin,
                session_id,
            ) {
                Ok(_) => {}
                Err(err) => warn!("Failed to save rank change to audit log: {}", err),
            };
            req.state()
                .memcache
                .borrow_mut()
                .clear_cache_for_package(ident.clone().into());
            HttpResponse::new(StatusCode::OK)
        }
        Err(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
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
        Err(err) => {
            debug!("{:?}", err);
            err.into()
        }
    }
}

//
// Internal - these functions should return Result<..>
//

fn audit_package_rank_change(
    req: &HttpRequest<AppState>,
    conn: &PgConnection,
    ident: PackageIdent,
    channel: String,
    operation: PCO,
    origin: String,
    session_id: u64,
) -> Result<()> {
    match PackageChannelAudit::audit(
        PCA {
            ident: BuilderPackageIdent(ident),
            channel: channel,
            operation: operation,
            trigger: helpers::trigger_from_request_model(req),
            requester_id: session_id as i64,
            requester_name: get_session_user_name(req, session_id),
            origin: origin,
        },
        &*conn,
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
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
    ident: OriginPackageIdent,
    channel: String,
) -> Result<String> {
    let opt_session_id = match authorize_session(req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };
    Counter::GetChannelPackage.increment();

    let mut memcache = req.state().memcache.borrow_mut();
    let req_ident = ident.clone();

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(e) => return Err(e.into()),
    };

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

    let pkg = match Channel::get_latest_package(
        GetLatestPackage {
            ident: BuilderPackageIdent(ident.clone().into()),
            channel: channel.clone(),
            target: target.to_string(),
            visibility: helpers::visibility_for_optional_session_model(
                req,
                opt_session_id,
                &ident.get_origin(),
            ),
        },
        &*conn,
    ) {
        Ok(pkg) => pkg,
        Err(NotFound) => {
            return Err(Error::NetError(NetError::new(
                ErrCode::ENTITY_NOT_FOUND,
                "channel_pkg:get_latest:1",
            )).into())
        }
        Err(err) => return Err(err.into()),
    };

    let mut pkg_json = serde_json::to_value(pkg.clone()).unwrap();
    let channels = helpers::channels_for_package_ident(req, &pkg.ident.clone().into());
    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(is_a_service(&pkg.into()));

    let json_body = serde_json::to_string(&pkg_json).unwrap();
    memcache.set_package(req_ident.clone().into(), &json_body, &channel, &target);

    Ok(json_body)
}
