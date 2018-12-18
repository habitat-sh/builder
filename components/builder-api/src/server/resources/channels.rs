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
use serde_json;

use crate::bldr_core::metrics::CounterMetric;
use crate::hab_core::package::{PackageIdent, PackageTarget};

use crate::db::models::channel::*;
use crate::db::models::package::{BuilderPackageIdent, Package};

use crate::server::authorize::authorize_session;
use crate::server::error::{Error, Result};
use crate::server::framework::headers;
use crate::server::helpers::{self, visibility_for_optional_session, Pagination, Target};
use crate::server::services::metrics::Counter;
use crate::server::AppState;

// Query param containers
#[derive(Debug, Default, Clone, Deserialize)]
struct SandboxBool {
    #[serde(default)]
    sandbox: bool,
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
#[allow(clippy::needless_pass_by_value)]
fn get_channels((req, sandbox): (HttpRequest<AppState>, Query<SandboxBool>)) -> HttpResponse {
    let origin = Path::<(String)>::extract(&req).unwrap().into_inner();

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Channel::list(&origin, sandbox.sandbox, &*conn).map_err(Error::DieselError) {
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
                })
                .collect();
            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(ident_list)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_channel(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner();

    let session_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Channel::create(
        &CreateChannel {
            name: &channel,
            origin: &origin,
            owner_id: session_id as i64,
        },
        &*conn,
    ) {
        Ok(channel) => HttpResponse::Created().json(channel),
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            HttpResponse::Conflict().into()
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
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

    req.state()
        .memcache
        .borrow_mut()
        .clear_cache_for_channel(&origin, &channel);

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Channel::delete(&origin, &channel, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn promote_package(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner();

    let session = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let ident = PackageIdent::new(
        origin.clone(),
        pkg.clone(),
        Some(version.clone()),
        Some(release.clone()),
    );

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginChannelPackage::promote(
        OriginChannelPromote {
            ident: BuilderPackageIdent(ident.clone()),
            origin: origin.clone(),
            channel: channel.clone(),
        },
        &*conn,
    )
    .map_err(Error::DieselError)
    {
        Ok(_) => {
            match PackageChannelAudit::audit(
                &PackageChannelAudit {
                    package_ident: BuilderPackageIdent(ident.clone()),
                    channel: &channel,
                    operation: PackageChannelOperation::Promote,
                    trigger: helpers::trigger_from_request_model(&req),
                    requester_id: session.get_id() as i64,
                    requester_name: &session.get_name(),
                    origin: &origin,
                },
                &*conn,
            ) {
                Ok(_) => {}
                Err(err) => debug!("Failed to save rank change to audit log: {}", err),
            };
            req.state()
                .memcache
                .borrow_mut()
                .clear_cache_for_package(&ident);
            HttpResponse::new(StatusCode::OK)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn demote_package(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner();

    if channel == "unstable" {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    let session = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let ident = PackageIdent::new(
        origin.clone(),
        pkg.clone(),
        Some(version.clone()),
        Some(release.clone()),
    );

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginChannelPackage::demote(
        OriginChannelDemote {
            ident: BuilderPackageIdent(ident.clone()),
            origin: origin.clone(),
            channel: channel.clone(),
        },
        &*conn,
    )
    .map_err(Error::DieselError)
    {
        Ok(_) => {
            match PackageChannelAudit::audit(
                &PackageChannelAudit {
                    package_ident: BuilderPackageIdent(ident.clone()),
                    channel: &channel,
                    operation: PackageChannelOperation::Demote,
                    trigger: helpers::trigger_from_request_model(&req),
                    requester_id: session.get_id() as i64,
                    requester_name: &session.get_name(),
                    origin: &origin,
                },
                &*conn,
            ) {
                Ok(_) => {}
                Err(err) => debug!("Failed to save rank change to audit log: {}", err),
            };
            req.state()
                .memcache
                .borrow_mut()
                .clear_cache_for_package(&ident);
            HttpResponse::new(StatusCode::OK)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_packages_for_origin_channel_package_version(
    (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg, version) = Path::<(String, String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let ident = PackageIdent::new(origin, pkg, Some(version.clone()), None);

    match do_get_channel_packages(&req, &pagination, &ident, &channel) {
        Ok((packages, count)) => {
            postprocess_channel_package_list(&req, &packages, count, &pagination)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_packages_for_origin_channel_package(
    (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_channel_packages(&req, &pagination, &ident, &channel) {
        Ok((packages, count)) => {
            postprocess_channel_package_list(&req, &packages, count, &pagination)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_packages_for_origin_channel(
    (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    // It feels 1000x wrong to set the package name to ""
    let ident = PackageIdent::new(origin, String::from(""), None, None);

    match do_get_channel_packages(&req, &pagination, &ident, &channel) {
        Ok((packages, count)) => {
            postprocess_channel_package_list(&req, &packages, count, &pagination)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_latest_package_for_origin_channel_package(
    (qtarget, req): (Query<Target>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_channel_package(&req, &qtarget, &ident, &channel) {
        Ok(json_body) => HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .header(http::header::CACHE_CONTROL, headers::cache(false))
            .body(json_body),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_latest_package_for_origin_channel_package_version(
    (qtarget, req): (Query<Target>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg, version) = Path::<(String, String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let ident = PackageIdent::new(origin, pkg, Some(version), None);

    match do_get_channel_package(&req, &qtarget, &ident, &channel) {
        Ok(json_body) => HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .header(http::header::CACHE_CONTROL, headers::cache(false))
            .body(json_body),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_package_fully_qualified(
    (qtarget, req): (Query<Target>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, channel, pkg, version, release) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

    let ident = PackageIdent::new(origin, pkg, Some(version), Some(release));

    match do_get_channel_package(&req, &qtarget, &ident, &channel) {
        Ok(json_body) => HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .header(http::header::CACHE_CONTROL, headers::cache(false))
            .body(json_body),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

//
// Internal - these functions should return Result<..>
//

fn do_get_channel_packages(
    req: &HttpRequest<AppState>,
    pagination: &Query<Pagination>,
    ident: &PackageIdent,
    channel: &str,
) -> Result<(Vec<BuilderPackageIdent>, i64)> {
    let opt_session_id = match authorize_session(&req, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };
    let (page, per_page) = helpers::extract_pagination_in_pages(pagination);

    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    Channel::list_packages(
        &ListChannelPackages {
            ident: &BuilderPackageIdent(ident.clone()),
            visibility: &helpers::visibility_for_optional_session(
                &req,
                opt_session_id,
                &ident.origin,
            ),
            origin: &ident.origin,
            channel,
            page: page as i64,
            limit: per_page as i64,
        },
        &*conn,
    )
    .map_err(Error::DieselError)
}

fn do_get_channel_package(
    req: &HttpRequest<AppState>,
    qtarget: &Query<Target>,
    ident: &PackageIdent,
    channel: &str,
) -> Result<String> {
    let opt_session_id = match authorize_session(req, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };
    Counter::GetChannelPackage.increment();

    let req_ident = ident.clone();

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            debug!("Query requested target = {}", t);
            PackageTarget::from_str(t)?
        }
        None => helpers::target_from_headers(req),
    };

    // Scope this memcache usage so the reference goes out of
    // scope before the visibility_for_optional_session call
    // below
    {
        let mut memcache = req.state().memcache.borrow_mut();
        match memcache.get_package(&req_ident, channel, &target, opt_session_id) {
            (true, Some(pkg_json)) => {
                trace!(
                    "Channel package {} {} {} {:?} - cache hit with pkg json",
                    channel,
                    ident,
                    target,
                    opt_session_id
                );
                // Note: the Package specifier is needed even though the variable is un-used
                let _p: Package = match serde_json::from_str(&pkg_json) {
                    Ok(p) => p,
                    Err(e) => {
                        debug!("Unable to deserialize package json, err={:?}", e);
                        return Err(Error::SerdeJson(e));
                    }
                };
                return Ok(pkg_json);
            }
            (true, None) => {
                trace!(
                    "Channel package {} {} {} {:?} - cache hit with 404",
                    channel,
                    ident,
                    target,
                    opt_session_id
                );
                return Err(Error::NotFound);
            }
            (false, _) => {
                trace!(
                    "Channel package {} {} {} {:?} - cache miss",
                    channel,
                    ident,
                    target,
                    opt_session_id
                );
            }
        };
    }

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(e) => return Err(e.into()),
    };

    let pkg = match Channel::get_latest_package(
        &GetLatestPackage {
            ident: &BuilderPackageIdent(ident.clone()),
            channel,
            target: &target,
            visibility: &helpers::visibility_for_optional_session(
                req,
                opt_session_id,
                &ident.origin,
            ),
        },
        &*conn,
    ) {
        Ok(pkg) => pkg,
        Err(NotFound) => {
            let mut memcache = req.state().memcache.borrow_mut();
            memcache.set_package(&req_ident, None, channel, &target, opt_session_id);
            return Err(Error::NotFound);
        }
        Err(err) => return Err(err.into()),
    };

    let mut pkg_json = serde_json::to_value(pkg.clone()).unwrap();
    let channels = channels_for_package_ident(req, &pkg.ident.clone(), &*conn)?;

    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(pkg.is_a_service());

    let json_body = serde_json::to_string(&pkg_json).unwrap();

    {
        let mut memcache = req.state().memcache.borrow_mut();
        memcache.set_package(
            &req_ident,
            Some(&json_body),
            channel,
            &target,
            opt_session_id,
        );
    }

    Ok(json_body)
}

pub fn channels_for_package_ident(
    req: &HttpRequest<AppState>,
    package: &BuilderPackageIdent,
    conn: &PgConnection,
) -> Result<Option<Vec<String>>> {
    let opt_session_id = match authorize_session(req, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    match Package::list_package_channels(
        package,
        visibility_for_optional_session(req, opt_session_id, &package.clone().origin),
        &*conn,
    )
    .map_err(Error::DieselError)
    {
        Ok(channels) => {
            let list: Vec<String> = channels
                .iter()
                .map(|channel| channel.name.to_string())
                .collect();

            Ok(Some(list))
        }
        Err(err) => Err(err),
    }
}

// Helper

fn postprocess_channel_package_list(
    _req: &HttpRequest<AppState>,
    packages: &[BuilderPackageIdent],
    count: i64,
    pagination: &Query<Pagination>,
) -> HttpResponse {
    let (start, _) = helpers::extract_pagination(pagination);
    let pkg_count = packages.len() as isize;
    let stop = match pkg_count {
        0 => count,
        _ => (start + pkg_count - 1) as i64,
    };

    debug!(
        "postprocessing channel package list, start: {}, stop: {}, total_count: {}",
        start, stop, count
    );

    let body =
        helpers::package_results_json(packages, count as isize, start as isize, stop as isize);

    let mut response = if count as isize > (stop as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response
        .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
        .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
        .body(body)
}
