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

use std::fs::{self, remove_file, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::str::FromStr;

use actix_web::http::header::{ContentDisposition, ContentType, DispositionParam, DispositionType};
use actix_web::http::{self, Method, StatusCode};
use actix_web::FromRequest;
use actix_web::{error, App, AsyncResponder, HttpMessage, HttpRequest, HttpResponse, Path, Query};
use bytes::Bytes;
use diesel::result::Error::NotFound;
use futures::sync::mpsc;
use futures::{future::ok as fut_ok, Future, Stream};
use protobuf;
use serde_json;
use tempfile::tempdir_in;
use url;
use uuid::Uuid;

use bldr_core::helpers::transition_visibility;
use bldr_core::metrics::CounterMetric;
use hab_core::package::{FromArchive, Identifiable, PackageArchive, PackageIdent, PackageTarget};
use hab_net::{ErrCode, NetError, NetOk};

use protocol::jobsrv::*;
use protocol::originsrv::*;

use db::models::package::{
    BuilderPackageIdent, BuilderPackageTarget, GetLatestPackage, GetPackage, ListPackages,
    NewPackage, Package, PackageVisibility,
};
use server::authorize::{authorize_session, get_session_user_name};
use server::error::{Error, Result};
use server::feat;
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers::{self, Pagination, Target};
use server::services::metrics::Counter;
use server::AppState;

// Query param containers
#[derive(Debug, Deserialize)]
pub struct Upload {
    #[serde(default = "default_target")]
    target: String,
    #[serde(default)]
    checksum: String,
    #[serde(default)]
    builder: Option<String>,
    #[serde(default)]
    forced: bool,
}

#[derive(Debug, Deserialize)]
pub struct Schedule {
    #[serde(default = "default_target")]
    target: String,
    #[serde(default)]
    deps_only: Option<String>,
    #[serde(default)]
    origin_only: Option<String>,
    #[serde(default)]
    package_only: Option<String>,
}

fn default_target() -> String {
    "x86_64-linux".to_string()
}

#[derive(Debug, Deserialize)]
pub struct GetSchedule {
    #[serde(default)]
    include_projects: bool,
}

#[derive(Debug, Deserialize)]
pub struct OriginScheduleStatus {
    #[serde(default)]
    limit: String,
}

pub struct Packages {}

impl Packages {
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route("/depot/pkgs/{origin}", Method::GET, get_packages_for_origin)
            .route("/depot/pkgs/search/{query}", Method::GET, search_packages)
            .route("/depot/pkgs/schedule/{groupid}", Method::GET, get_schedule)
            .route(
                "/depot/pkgs/{origin}/{pkg}",
                Method::GET,
                get_packages_for_origin_package,
            ).route(
                "/depot/pkgs/schedule/{origin}/status",
                Method::GET,
                get_origin_schedule_status,
            ).route(
                "/depot/pkgs/schedule/{origin}/{pkg}",
                Method::POST,
                schedule_job_group,
            ).route(
                "/depot/pkgs/{origin}/{pkg}/latest",
                Method::GET,
                get_latest_package_for_origin_package,
            ).route(
                "/depot/pkgs/{origin}/{pkg}/versions",
                Method::GET,
                list_package_versions,
            ).route(
                "/depot/pkgs/{origin}/{pkg}/{version}",
                Method::GET,
                get_packages_for_origin_package_version,
            ).route(
                "/depot/pkgs/{origin}/{pkg}/{version}/latest",
                Method::GET,
                get_latest_package_for_origin_package_version,
            ).route(
                "/depot/pkgs/{origin}/{pkg}/{version}/{release}",
                Method::POST,
                upload_package,
            ).route(
                "/depot/pkgs/{origin}/{pkg}/{version}/{release}",
                Method::GET,
                get_package,
            ).route(
                "/depot/pkgs/{origin}/{pkg}/{version}/{release}/download",
                Method::GET,
                download_package,
            ).route(
                "/depot/pkgs/{origin}/{pkg}/{version}/{release}/channels",
                Method::GET,
                get_package_channels,
            ).route(
                "/depot/pkgs/{origin}/{pkg}/{version}/{release}/{visibility}",
                Method::PATCH,
                package_privacy_toggle,
            )
    }
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_packages_for_origin(
    (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
    let ident = OriginPackageIdent::from_str(origin.as_str()).unwrap();

    match do_get_packages(&req, ident, &pagination) {
        Ok((packages, count)) => postprocess_package_list_model(&req, packages, count, pagination),
        Err(err) => err.into(),
    }
}

fn get_packages_for_origin_package(
    (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, pkg) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(pkg);

    match do_get_packages(&req, ident, &pagination) {
        Ok((packages, count)) => postprocess_package_list_model(&req, packages, count, pagination),
        Err(err) => err.into(),
    }
}

fn get_packages_for_origin_package_version(
    (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, pkg, version) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(pkg);
    ident.set_version(version);

    match do_get_packages(&req, ident, &pagination) {
        Ok((packages, count)) => postprocess_package_list_model(&req, packages, count, pagination),
        Err(err) => err.into(),
    }
}

fn get_latest_package_for_origin_package(
    (qtarget, req): (Query<Target>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, pkg) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_package(&req, &qtarget, ident) {
        Ok(package) => postprocess_package_model(&req, &package, false),
        Err(err) => err.into(),
    }
}

fn get_latest_package_for_origin_package_version(
    (qtarget, req): (Query<Target>, HttpRequest<AppState>),
) -> HttpResponse {
    let (origin, pkg, version) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let ident = PackageIdent::new(origin, pkg, Some(version), None);

    match do_get_package(&req, &qtarget, ident) {
        Ok(package) => postprocess_package_model(&req, &package, false),
        Err(err) => err.into(),
    }
}

fn get_package((qtarget, req): (Query<Target>, HttpRequest<AppState>)) -> HttpResponse {
    let (origin, pkg, version, release) = Path::<(String, String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let ident = PackageIdent::new(origin, pkg, Some(version), Some(release));

    match do_get_package(&req, &qtarget, ident) {
        Ok(package) => postprocess_package_model(&req, &package, true),
        Err(err) => err.into(),
    }
}

// TODO : Convert to async
fn download_package((qtarget, req): (Query<Target>, HttpRequest<AppState>)) -> HttpResponse {
    let (origin, name, version, release) = Path::<(String, String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let conn = match req.state().db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let opt_session_id = match authorize_session(&req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let ident = PackageIdent::new(origin, name, Some(version), Some(release));

    let mut vis =
        helpers::visibility_for_optional_session_model(&req, opt_session_id, &ident.origin);
    vis.push(PackageVisibility::Hidden);

    // TODO: Deprecate target from headers
    let target = match qtarget.target.clone() {
        Some(t) => {
            debug!("Query requested target = {}", t);
            PackageTarget::from_str(&t).unwrap() // Unwrap Ok ?
        }
        None => helpers::target_from_headers(&req),
    };

    if !req.state().config.api.targets.contains(&target) {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    match Package::get(
        GetPackage {
            ident: BuilderPackageIdent(ident.clone()),
            visibility: vis,
            target: BuilderPackageTarget(target),
        },
        &*conn,
    ) {
        Ok(package) => {
            let dir =
                tempdir_in(&req.state().config.api.data_path).expect("Unable to create a tempdir!");
            let file_path = dir.path().join(archive_name(&package.ident, &target));
            let temp_ident = ident.to_owned().into();
            match req
                .state()
                .packages
                .download(&file_path, &temp_ident, &target)
            {
                Ok(archive) => download_response_for_archive(archive, file_path),
                Err(e) => {
                    warn!(
                        "Failed to download package, ident={}, err={:?}",
                        temp_ident, e
                    );
                    return HttpResponse::new(StatusCode::NOT_FOUND);
                }
            }
        }
        Err(err) => Error::DieselError(err).into(),
    }
}

fn upload_package(
    (qupload, req): (Query<Upload>, HttpRequest<AppState>),
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    let (origin, name, version, release) = Path::<(String, String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let ident = PackageIdent::new(origin, name, Some(version), Some(release));

    if !ident.valid() || !ident.fully_qualified() {
        info!(
            "Invalid or not fully qualified package identifier: {}",
            ident
        );
        return Box::new(fut_ok(HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY)));
    }

    match do_upload_package_start(&req, &qupload, &ident) {
        Ok((temp_path, writer)) => {
            req.state()
                .memcache
                .borrow_mut()
                .clear_cache_for_package(ident.clone().into());
            do_upload_package_async(req, qupload, ident, temp_path, writer)
        }
        Err(Error::NetError(ref err)) if err.get_code() == ErrCode::ENTITY_CONFLICT => {
            debug!(
                "Failed to upload package {}, metadata already exists",
                &ident
            );
            Box::new(fut_ok(HttpResponse::new(StatusCode::CONFLICT)))
        }
        Err(err) => {
            warn!("Failed to upload package {}, err={:?}", &ident, err);
            Box::new(fut_ok(err.into()))
        }
    }
}

// TODO REVIEW: should this path be under jobs instead?
fn schedule_job_group((qschedule, req): (Query<Schedule>, HttpRequest<AppState>)) -> HttpResponse {
    let (origin_name, package) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin_name)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let account_name = get_session_user_name(&req, account_id);

    // We only support building for Linux x64 only currently
    if qschedule.target != "x86_64-linux" {
        info!("Rejecting build with target: {}", qschedule.target);
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    let mut secret_key_request = OriginPrivateSigningKeyGet::new();
    let origin = match helpers::get_origin(&req, &origin_name) {
        Ok(origin) => {
            secret_key_request.set_owner_id(origin.get_owner_id());
            secret_key_request.set_origin(origin_name.clone());
            origin
        }
        Err(err) => return err.into(),
    };

    let need_keys = match route_message::<OriginPrivateSigningKeyGet, OriginPrivateSigningKey>(
        &req,
        &secret_key_request,
    ) {
        Ok(key) => {
            let mut pub_key_request = OriginPublicSigningKeyGet::new();
            pub_key_request.set_origin(origin_name.clone());
            pub_key_request.set_revision(key.get_revision().to_string());
            route_message::<OriginPublicSigningKeyGet, OriginPublicSigningKey>(
                &req,
                &pub_key_request,
            ).is_err()
        }
        Err(_) => true,
    };

    if need_keys {
        if let Err(err) = helpers::generate_origin_keys(&req, account_id, origin) {
            return err.into();
        }
    }

    let mut request = JobGroupSpec::new();
    request.set_origin(origin_name);
    request.set_package(package);
    request.set_target(qschedule.target.clone());
    request.set_deps_only(qschedule.deps_only.is_some());
    request.set_origin_only(qschedule.origin_only.is_some());
    request.set_package_only(qschedule.package_only.is_some());
    request.set_trigger(helpers::trigger_from_request(&req));
    request.set_requester_id(account_id);
    request.set_requester_name(account_name.clone());

    match route_message::<JobGroupSpec, JobGroup>(&req, &request) {
        Ok(group) => {
            let msg = format!("Scheduled job group for {}", group.get_project_name());

            // We don't really want to abort anything just because a call to segment failed. Let's
            // just log it and move on.
            if let Err(e) = req.state().segment.track(&account_name, &msg) {
                warn!("Error tracking scheduling of job group in segment, {}", e);
            }

            HttpResponse::Created()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(group)
        }
        Err(err) => err.into(),
    }
}

fn get_schedule((qgetschedule, req): (Query<GetSchedule>, HttpRequest<AppState>)) -> HttpResponse {
    let group_id_str = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
    let group_id = match group_id_str.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::new(StatusCode::BAD_REQUEST),
    };

    let mut request = JobGroupGet::new();
    request.set_group_id(group_id);
    request.set_include_projects(qgetschedule.include_projects);

    match route_message::<JobGroupGet, JobGroup>(&req, &request) {
        Ok(group) => HttpResponse::Ok()
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .json(group),
        Err(err) => err.into(),
    }
}

fn get_origin_schedule_status(
    (qoss, req): (Query<OriginScheduleStatus>, HttpRequest<AppState>),
) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
    let limit = qoss.limit.parse::<u32>().unwrap_or(10);

    let mut request = JobGroupOriginGet::new();
    request.set_origin(origin);
    request.set_limit(limit);

    match route_message::<JobGroupOriginGet, JobGroupOriginResponse>(&req, &request) {
        Ok(jgor) => HttpResponse::Ok()
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .json(jgor.get_job_groups()),
        Err(err) => err.into(),
    }
}

fn get_package_channels(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, name, version, release) = Path::<(String, String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let opt_session_id = match authorize_session(&req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let mut request = OriginPackageChannelListRequest::new();

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(name);
    ident.set_version(version);
    ident.set_release(release);

    if !ident.fully_qualified() {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    request.set_visibilities(helpers::visibility_for_optional_session(
        &req,
        opt_session_id,
        &ident.get_origin(),
    ));
    request.set_ident(ident);

    match route_message::<OriginPackageChannelListRequest, OriginPackageChannelListResponse>(
        &req, &request,
    ) {
        Ok(channels) => {
            let list: Vec<String> = channels
                .get_channels()
                .iter()
                .map(|channel| channel.get_name().to_string())
                .collect();
            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(list)
        }
        Err(err) => err.into(),
    }
}

fn list_package_versions(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, name) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let opt_session_id = match authorize_session(&req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let mut request = OriginPackageVersionListRequest::new();
    request.set_visibilities(helpers::visibility_for_optional_session(
        &req,
        opt_session_id,
        &origin,
    ));
    request.set_origin(origin);
    request.set_name(name);

    match route_message::<OriginPackageVersionListRequest, OriginPackageVersionListResponse>(
        &req, &request,
    ) {
        Ok(packages) => {
            let body = serde_json::to_string(&packages.get_versions().to_vec()).unwrap();
            HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .body(body)
        }
        Err(err) => err.into(),
    }
}

fn search_packages((pagination, req): (Query<Pagination>, HttpRequest<AppState>)) -> HttpResponse {
    Counter::SearchPackages.increment();

    let query = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let opt_session_id = match authorize_session(&req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let (start, stop) = helpers::extract_pagination(&pagination);

    let mut request = OriginPackageSearchRequest::new();
    request.set_start(start as u64);
    request.set_stop(stop as u64);

    if let Some(session_id) = opt_session_id {
        let mut my_origins = MyOriginsRequest::new();
        my_origins.set_account_id(session_id);

        match route_message::<MyOriginsRequest, MyOriginsResponse>(&req, &my_origins) {
            Ok(response) => request.set_my_origins(protobuf::RepeatedField::from_vec(
                response.get_origins().to_vec(),
            )),
            Err(e) => {
                debug!(
                    "Error fetching origins for account id {}, {}",
                    session_id, e
                );
                return e.into();
            }
        }
    }

    // First, try to parse the query like it's a PackageIdent, since it seems reasonable to expect
    // that many people will try searching using that kind of string, e.g. core/redis.  If that
    // works, set the origin appropriately and do a regular search.  If that doesn't work, do a
    // search across all origins, similar to how the "distinct" search works now, but returning all
    // the details instead of just names.
    let decoded_query = match url::percent_encoding::percent_decode(query.as_bytes()).decode_utf8()
    {
        Ok(q) => q.to_string(),
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
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
    request.set_distinct(pagination.distinct);

    match route_message::<OriginPackageSearchRequest, OriginPackageListResponse>(&req, &request) {
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

            let status = if packages.get_count() as isize > (packages.get_stop() as isize + 1) {
                StatusCode::PARTIAL_CONTENT
            } else {
                StatusCode::OK
            };

            HttpResponse::build(status)
                .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .body(body)
        }
        Err(err) => err.into(),
    }
}

fn package_privacy_toggle(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, name, version, release, visibility) =
        Path::<(String, String, String, String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    // users aren't allowed to set packages to hidden manually
    if visibility.to_lowercase() == "hidden" {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let mut ident = OriginPackageIdent::new();
    ident.set_origin(origin);
    ident.set_name(name);
    ident.set_version(version);
    ident.set_release(release);

    if !ident.valid() {
        info!("Invalid package identifier: {}", ident);
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let opv: OriginPackageVisibility = match visibility.parse() {
        Ok(o) => o,
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
    };

    let mut opg = OriginPackageGet::new();
    opg.set_ident(ident);
    opg.set_visibilities(helpers::all_visibilities());

    match route_message::<OriginPackageGet, OriginPackage>(&req, &opg) {
        Ok(mut package) => {
            let real_visibility = transition_visibility(opv, package.get_visibility());
            let mut opu = OriginPackageUpdate::new();
            package.set_visibility(real_visibility);
            opu.set_pkg(package);

            match route_message::<OriginPackageUpdate, NetOk>(&req, &opu) {
                Ok(_) => HttpResponse::Ok().finish(),
                Err(e) => e.into(),
            }
        }
        Err(e) => e.into(),
    }
}

//
// Public helpers
//

// TODO : this needs to be re-designed to not fan out
// Common functionality for pkgs and channel routes
pub fn postprocess_package_list(
    req: &HttpRequest<AppState>,
    oplr: &OriginPackageListResponse,
    distinct: bool,
) -> HttpResponse {
    debug!(
        "postprocessing package list, start: {}, stop: {}, total_count: {}",
        oplr.get_start(),
        oplr.get_stop(),
        oplr.get_count()
    );

    let mut results = Vec::new();

    // The idea here is for every package we get back, pull its channels using the zmq API
    // and accumulate those results. This avoids the N+1 HTTP requests that would be
    // required to fetch channels for a list of packages in the UI. However, if our request
    // has been marked as "distinct" then skip this step because it doesn't make sense in
    // that case. Let's get platforms at the same time.
    for package in oplr.get_idents().to_vec() {
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
        oplr.get_count() as isize,
        oplr.get_start() as isize,
        oplr.get_stop() as isize,
    );

    let mut response = if oplr.get_count() as isize > (oplr.get_stop() as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response
        .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
        .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
        .body(body)
}

// TODO : this needs to be re-designed to not fan out
// Common functionality for pkgs and channel routes
pub fn postprocess_package_list_model(
    req: &HttpRequest<AppState>,
    packages: Vec<BuilderPackageIdent>,
    count: i64,
    pagination: Query<Pagination>,
) -> HttpResponse {
    let (start, _) = helpers::extract_pagination(&pagination);
    let pkg_count = packages.len() as isize;
    let stop = match pkg_count {
        0 => count,
        _ => (start + pkg_count - 1) as i64,
    };

    debug!(
        "postprocessing package list, start: {}, stop: {}, total_count: {}",
        start, stop, count
    );

    let mut results = Vec::new();

    // The idea here is for every package we get back, pull its channels using the zmq API
    // and accumulate those results. This avoids the N+1 HTTP requests that would be
    // required to fetch channels for a list of packages in the UI. However, if our request
    // has been marked as "distinct" then skip this step because it doesn't make sense in
    // that case. Let's get platforms at the same time.
    for ident in packages {
        let mut channels: Option<Vec<String>> = None;
        let mut platforms: Option<Vec<String>> = None;

        if !pagination.distinct {
            channels = helpers::channels_for_package_ident(req, &ident.clone().into());
            platforms = helpers::platforms_for_package_ident(req, &ident.clone().into());
        }

        let mut pkg_json = serde_json::to_value(ident.clone()).unwrap();

        if channels.is_some() {
            pkg_json["channels"] = json!(channels);
        }

        if platforms.is_some() {
            pkg_json["platforms"] = json!(platforms);
        }

        results.push(pkg_json);
    }

    let body =
        helpers::package_results_json(&results, count as isize, start as isize, stop as isize);

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

//
// Internal - these functions should return Result<..>
//
fn do_get_packages(
    req: &HttpRequest<AppState>,
    ident: OriginPackageIdent,
    pagination: &Query<Pagination>,
) -> Result<(Vec<BuilderPackageIdent>, i64)> {
    let opt_session_id = match authorize_session(req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    let (page, per_page) = helpers::extract_pagination_in_pages(pagination);

    let lpr = ListPackages {
        ident: BuilderPackageIdent(ident.clone().into()),
        visibility: helpers::visibility_for_optional_session_model(
            &req,
            opt_session_id,
            &ident.get_origin(),
        ),
        page: page as i64,
        limit: per_page as i64,
    };

    if pagination.distinct {
        return Package::list_distinct(lpr, &*conn).map_err(Error::DieselError);
    }

    Package::list(lpr, &*conn).map_err(Error::DieselError)
}

//
//  Async helpers
//
fn do_upload_package_start(
    req: &HttpRequest<AppState>,
    qupload: &Query<Upload>,
    ident: &PackageIdent,
) -> Result<(PathBuf, BufWriter<File>)> {
    authorize_session(req, Some(&ident.origin))?;

    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    if qupload.forced {
        debug!(
            "Upload was forced (bypassing existing package check) for: {}",
            ident
        );
    } else {
        match Package::get(
            GetPackage {
                ident: BuilderPackageIdent(ident.clone()),
                visibility: helpers::all_visibilities_model(),
                target: BuilderPackageTarget(PackageTarget::from_str(&qupload.target).unwrap()), // Unwrap OK
            },
            &*conn,
        ) {
            Ok(_) => {
                return Err(Error::NetError(NetError::new(
                    ErrCode::ENTITY_CONFLICT,
                    "ds:up:0",
                )))
            }
            Err(NotFound) => {}
            Err(err) => return Err(err.into()),
        }
    }

    debug!("UPLOADING {}, params={:?}", ident, qupload);

    // Create a temp file at the data path
    let temp_name = format!("{}.tmp", Uuid::new_v4());
    let temp_path = req.state().config.api.data_path.join(temp_name);

    let file = File::create(&temp_path)?;
    let writer = BufWriter::new(file);

    Ok((temp_path, writer))
}

// TODO: Break this up further, convert S3 upload to async
fn do_upload_package_finish(
    req: HttpRequest<AppState>,
    qupload: Query<Upload>,
    ident: PackageIdent,
    temp_path: PathBuf,
) -> HttpResponse {
    let mut archive = PackageArchive::new(&temp_path);

    debug!("Package Archive: {:#?}", archive);

    let target_from_artifact = match archive.target() {
        Ok(target) => target,
        Err(e) => {
            info!("Could not read the target for {:#?}: {:#?}", archive, e);
            return HttpResponse::with_body(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("ds:up:1, err={:?}", e),
            );
        }
    };

    if !req
        .state()
        .config
        .api
        .targets
        .contains(&target_from_artifact)
    {
        debug!(
            "Unsupported package platform or architecture {}.",
            target_from_artifact
        );
        return HttpResponse::new(StatusCode::NOT_IMPLEMENTED);
    };

    let checksum_from_artifact = match archive.checksum() {
        Ok(cksum) => cksum,
        Err(e) => {
            debug!("Could not compute a checksum for {:#?}: {:#?}", archive, e);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "ds:up:2");
        }
    };

    if qupload.checksum != checksum_from_artifact {
        debug!(
            "Checksums did not match: from_param={:?}, from_artifact={:?}",
            qupload.checksum, checksum_from_artifact
        );
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "ds:up:3");
    }

    let conn = req.state().db.get_conn().map_err(Error::DbError).unwrap();

    // Check with scheduler to ensure we don't have circular deps, if configured
    if feat::is_enabled(feat::Jobsrv) {
        if let Err(e) = check_circular_deps(&req, &ident, &target_from_artifact, &mut archive) {
            return e.into();
        }
    }

    let file_path = &req.state().config.api.data_path;
    let filename = file_path.join(archive_name(&ident, &target_from_artifact));
    let temp_ident = ident.to_owned().into();

    match fs::rename(&temp_path, &filename) {
        Ok(_) => {}
        Err(e) => {
            warn!(
                "Unable to rename temp archive {:?} to {:?}, err={:?}",
                temp_path, filename, e
            );
            return Error::IO(e).into();
        }
    }

    // TODO: Make S3 upload async
    if let Err(err) = req
        .state()
        .packages
        .upload(&filename, &temp_ident, &target_from_artifact)
    {
        warn!("Unable to upload archive to s3!");
        return err.into();
    }

    debug!("File added to Depot: {:?}", &filename);

    let mut archive = PackageArchive::new(filename.clone());
    let mut package = match NewPackage::from_archive(&mut archive) {
        Ok(package) => package,
        Err(e) => {
            debug!("Error building package from archive: {:#?}", e);
            return Error::HabitatCore(e).into();
        }
    };

    if !ident.satisfies(&*package.ident) {
        debug!(
            "Ident mismatch, expected={:?}, got={:?}",
            ident, package.ident
        );

        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "ds:up:6");
    }

    let session_id = authorize_session(&req, None).unwrap(); // Unwrap Ok
    let session_name = get_session_user_name(&req, session_id);

    package.owner_id = session_id as i64;

    package.origin_id = match helpers::get_origin(&req, ident.origin.clone()) {
        Ok(origin) => origin.get_id() as i64,
        Err(e) => return e.into(),
    };

    // First, try to fetch visibility settings from a project, if one exists
    let mut project_get = OriginProjectGet::new();
    let project_name = format!("{}/{}", ident.origin.clone(), ident.name.clone());
    project_get.set_name(project_name);

    match route_message::<OriginProjectGet, OriginProject>(&req, &project_get) {
        Ok(proj) => {
            if proj.has_visibility() {
                package.visibility = proj.get_visibility().into();
            }
        }
        Err(_) => {
            // There's no project for this package. No worries - we'll check the origin
            let mut origin_get = OriginGet::new();
            origin_get.set_name(ident.origin.to_string());

            match route_message::<OriginGet, Origin>(&req, &origin_get) {
                Ok(o) => {
                    if o.has_default_package_visibility() {
                        package.visibility = o.get_default_package_visibility().into();
                    }
                }
                Err(err) => return err.into(),
            }
        }
    }

    // Re-create origin package as needed (eg, checksum update)
    match Package::create(package.clone(), &*conn) {
        Ok(pkg) => {
            if feat::is_enabled(feat::Jobsrv) {
                let mut job_graph_package = JobGraphPackageCreate::new();
                job_graph_package.set_package(pkg.into());

                if let Err(err) =
                    route_message::<JobGraphPackageCreate, OriginPackage>(&req, &job_graph_package)
                {
                    warn!("Failed to insert package into graph: {:?}", err);
                    return err.into();
                }
            }
        }
        Err(_) => return HttpResponse::InternalServerError().into(),
    }

    // Schedule re-build of dependent packages (if requested)
    // Don't schedule builds if the upload is being done by the builder
    if qupload.builder.is_none() && feat::is_enabled(feat::Jobsrv) {
        let mut request = JobGroupSpec::new();
        request.set_origin(ident.origin.to_string());
        request.set_package(ident.name.to_string());
        request.set_target(target_from_artifact.to_string());
        request.set_deps_only(true);
        request.set_origin_only(false);
        request.set_package_only(false);
        request.set_trigger(JobGroupTrigger::Upload);
        request.set_requester_id(session_id);
        request.set_requester_name(session_name);

        match route_message::<JobGroupSpec, JobGroup>(&req, &request) {
            Ok(group) => debug!(
                "Scheduled reverse dependecy build for {}, group id: {}",
                ident,
                group.get_id()
            ),
            Err(Error::NetError(ref err)) if err.get_code() == ErrCode::ENTITY_NOT_FOUND => {
                debug!("Unable to schedule build for {}, err: {:?}", ident, err)
            }
            Err(err) => warn!("Unable to schedule build for {}, err: {:?}", ident, err),
        }
    }

    match remove_file(&filename) {
        Ok(_) => debug!(
            "Successfully removed cached file after upload. {:?}",
            &filename
        ),
        Err(e) => warn!(
            "Failed to remove cached file after upload: {:?}, {}",
            &filename, e
        ),
    }

    HttpResponse::Created()
        .header(http::header::LOCATION, format!("{}", req.uri()))
        .body(format!("/pkgs/{}/download", *package.ident))
}

fn do_upload_package_async(
    req: HttpRequest<AppState>,
    qupload: Query<Upload>,
    ident: PackageIdent,
    temp_path: PathBuf,
    writer: BufWriter<File>,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.payload()
        // `Future::from_err` acts like `?` in that it coerces the error type from
        // the future into the final error type
        .from_err()
        // `fold` will asynchronously read each chunk of the request body and
        // call supplied closure, then it resolves to result of closure
        .fold(writer, write_archive_async)
        // `Future::and_then` can be used to merge an asynchronous workflow with a
        // synchronous workflow
        .and_then(|writer| {
            match writer.into_inner() {
                Ok(f) => {
                    f.sync_all()?;
                    Ok( do_upload_package_finish(req, qupload, ident, temp_path))
                },
                Err(err) => {
                    Err(Error::InnerError(err))
                }
            }
        })
        .responder()
}

fn do_get_package(
    req: &HttpRequest<AppState>,
    qtarget: &Query<Target>,
    ident: PackageIdent,
) -> Result<Package> {
    let opt_session_id = match authorize_session(req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };
    Counter::GetPackage.increment();

    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    let target = match qtarget.target.clone() {
        Some(t) => {
            debug!("Query requested target = {}", t);
            PackageTarget::from_str(&t)?
        }
        None => helpers::target_from_headers(req),
    };

    if ident.fully_qualified() {
        match Package::get(
            GetPackage {
                ident: BuilderPackageIdent(ident.clone()),
                visibility: helpers::visibility_for_optional_session_model(
                    req,
                    opt_session_id,
                    &ident.origin,
                ),
                target: BuilderPackageTarget(target),
            },
            &*conn,
        ) {
            Ok(pkg) => Ok(pkg),
            Err(NotFound) => {
                Err(Error::NetError(NetError::new(ErrCode::ENTITY_NOT_FOUND, "pkg:get:1")).into())
            }
            Err(err) => {
                debug!("{:?}", err);
                Err(err.into())
            }
        }
    } else {
        match Package::get_latest(
            GetLatestPackage {
                ident: BuilderPackageIdent(ident.clone()),
                target: BuilderPackageTarget(target),
                visibility: helpers::visibility_for_optional_session_model(
                    req,
                    opt_session_id,
                    &ident.origin,
                ),
            },
            &*conn,
        ) {
            Ok(pkg) => Ok(pkg),
            Err(NotFound) => Err(Error::NetError(NetError::new(
                ErrCode::ENTITY_NOT_FOUND,
                "pkg:latest:1",
            )).into()),
            Err(err) => {
                debug!("{:?}", err);
                Err(err.into())
            }
        }
    }
}

// TODO: this needs to be re-designed to not fan out
// Common functionality for pkgs and channel routes
pub fn postprocess_package(
    req: &HttpRequest<AppState>,
    pkg: &OriginPackage,
    should_cache: bool,
) -> HttpResponse {
    let mut pkg_json = serde_json::to_value(pkg.clone()).unwrap();
    let channels = helpers::channels_for_package_ident(req, pkg.get_ident());
    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(is_a_service(pkg));

    let body = serde_json::to_string(&pkg_json).unwrap();

    HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
        .header(http::header::ETAG, pkg.get_checksum().to_string())
        .header(http::header::CACHE_CONTROL, headers::cache(should_cache))
        .body(body)
}

pub fn postprocess_package_model(
    req: &HttpRequest<AppState>,
    pkg: &Package,
    should_cache: bool,
) -> HttpResponse {
    let mut pkg_json = serde_json::to_value(pkg.clone()).unwrap();
    // Some borrow checker workarounds until more channel code is converted in to model code
    let pkg_clone = &*pkg.ident;
    let channels =
        helpers::channels_for_package_ident(req, &OriginPackageIdent::from(pkg_clone.clone()));
    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(is_a_service_model(pkg));

    let body = serde_json::to_string(&pkg_json).unwrap();

    HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
        .header(http::header::ETAG, pkg.checksum.to_string())
        .header(http::header::CACHE_CONTROL, headers::cache(should_cache))
        .body(body)
}

//
// Internal helpers
//

// Return a formatted string representing the filename of an archive for the given package
// identifier pieces.
fn archive_name(ident: &PackageIdent, target: &PackageTarget) -> PathBuf {
    PathBuf::from(ident.archive_name_with_target(target).expect(&format!(
        "Package ident should be fully qualified, ident={}",
        &ident
    )))
}

fn download_response_for_archive(archive: PackageArchive, file_path: PathBuf) -> HttpResponse {
    let filename = archive.file_name();
    let file = match File::open(&file_path) {
        Ok(f) => f,
        Err(err) => {
            warn!("Unable to open file: {:?}", file_path);
            return Error::IO(err).into();
        }
    };
    let reader = BufReader::new(file);
    let bytes: Vec<u8> = reader.bytes().map(|r| r.unwrap()).collect();

    let (tx, rx_body) = mpsc::unbounded();
    let _ = tx.unbounded_send(Bytes::from(bytes));

    HttpResponse::Ok()
        .header(
            http::header::CONTENT_DISPOSITION,
            ContentDisposition {
                disposition: DispositionType::Attachment,
                parameters: vec![DispositionParam::Filename(filename)],
            },
        ).header(
            http::header::HeaderName::from_static(headers::XFILENAME),
            archive.file_name(),
        ).set(ContentType::octet_stream())
        .header(http::header::CACHE_CONTROL, headers::cache(true))
        .streaming(rx_body.map_err(|_| error::ErrorBadRequest("bad request")))
}

fn write_archive_async(mut writer: BufWriter<File>, chunk: Bytes) -> Result<BufWriter<File>> {
    debug!("Writing file upload chunk, size: {}", chunk.len());
    match writer.write(&chunk) {
        Ok(_) => (),
        Err(err) => {
            warn!("Error writing file upload chunk to temp file: {:?}", err);
            return Err(Error::IO(err));
        }
    }
    Ok(writer)
}

fn check_circular_deps(
    req: &HttpRequest<AppState>,
    ident: &PackageIdent,
    target: &PackageTarget,
    archive: &mut PackageArchive,
) -> Result<()> {
    let mut pcr_req = JobGraphPackagePreCreate::new();
    pcr_req.set_ident(format!("{}", ident));
    pcr_req.set_target(target.to_string());

    let mut pcr_deps = protobuf::RepeatedField::new();
    let deps_from_artifact = match archive.deps() {
        Ok(deps) => deps,
        Err(e) => {
            debug!("Could not get deps from {:#?}: {:#?}", archive, e);
            return Err(Error::HabitatCore(e));
        }
    };

    for ident in deps_from_artifact {
        let dep_str = format!("{}", ident);
        pcr_deps.push(dep_str);
    }
    pcr_req.set_deps(pcr_deps);

    match route_message::<JobGraphPackagePreCreate, NetOk>(req, &pcr_req) {
        Ok(_) => Ok(()),
        Err(Error::NetError(err)) => {
            if err.get_code() == ErrCode::ENTITY_CONFLICT {
                debug!(
                    "Failed package circular dependency check: {}, err: {:?}",
                    ident, err
                );
                return Err(Error::CircularDependency(ident.to_string()));
            } else {
                warn!("Error checking circular dependency, err={:?}", err);
            }
            return Err(Error::NetError(err));
        }
        Err(err) => return Err(err),
    }
}

pub fn is_a_service(package: &OriginPackage) -> bool {
    let m = package.get_manifest();

    // TODO: This is a temporary workaround until we plumb in a better solution for
    // determining whether a package is a service from the DB instead of needing
    // to crack the archive file to look for a SVC_USER file
    m.contains("pkg_exposes") || m.contains("pkg_binds") || m.contains("pkg_exports")
}

pub fn is_a_service_model(package: &Package) -> bool {
    let m = package.manifest.clone();
    // TODO: This is a temporary workaround until we plumb in a better solution for
    // determining whether a package is a service from the DB instead of needing
    // to crack the archive file to look for a SVC_USER file
    m.contains("pkg_exposes") || m.contains("pkg_binds") || m.contains("pkg_exports")
}
