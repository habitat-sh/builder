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

use crate::{bldr_core::{error::Error::RpcError,
                        metrics::CounterMetric},
            db::models::{channel::Channel,
                         origin::*,
                         package::{BuilderPackageIdent,
                                   BuilderPackageTarget,
                                   DeletePackage,
                                   GetLatestPackage,
                                   GetPackage,
                                   ListPackages,
                                   NewPackage,
                                   Package,
                                   PackageIdentWithChannelPlatform,
                                   PackageVisibility,
                                   SearchPackages},
                         settings::{GetOriginPackageSettings,
                                    NewOriginPackageSettings,
                                    OriginPackageSettings}},
            hab_core::{package::{FromArchive,
                                 Identifiable,
                                 PackageArchive,
                                 PackageIdent,
                                 PackageTarget},
                       ChannelIdent},
            protocol::{jobsrv,
                       net::NetOk,
                       originsrv},
            server::{authorize::authorize_session,
                     error::{Error,
                             Result},
                     feat,
                     framework::{headers,
                                 middleware::route_message},
                     helpers::{self,
                               req_state,
                               Pagination,
                               Target},
                     resources::channels::channels_for_package_ident,
                     services::metrics::Counter,
                     AppState}};
use actix_web::{body::Body,
                http::{self,
                       header::{ContentDisposition,
                                ContentType,
                                DispositionParam,
                                DispositionType},
                       StatusCode},
                web::{self,
                      Data,
                      Path,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};
use bytes::Bytes;
use diesel::result::Error::NotFound;
use futures::{channel::mpsc,
              StreamExt};
use percent_encoding;
use protobuf;
use serde::ser::Serialize;
use serde_json;
use std::{fs::{self,
               remove_file,
               File},
          io::{BufReader,
               BufWriter,
               Read,
               Write},
          path::PathBuf,
          str::FromStr};
use tempfile::tempdir_in;
use uuid::Uuid;

// Query param containers
#[derive(Debug, Deserialize)]
pub struct Upload {
    #[serde(default)]
    target: Option<String>,
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

fn default_target() -> String { "x86_64-linux".to_string() }

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
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/depot/pkgs/{origin}",
                  web::get().to(get_packages_for_origin))
           .route("/depot/pkgs/search/{query}", web::get().to(search_packages))
           .route("/depot/pkgs/schedule/{groupid}",
                  web::get().to(get_schedule))
           .route("/depot/pkgs/{origin}/{pkg}",
                  web::get().to(get_packages_for_origin_package))
           .route("/depot/pkgs/schedule/{origin}/status",
                  web::get().to(get_origin_schedule_status))
           .route("/depot/pkgs/schedule/{origin}/{pkg}",
                  web::post().to(schedule_job_group))
           .route("/depot/pkgs/{origin}/{pkg}/latest",
                  web::get().to(get_latest_package_for_origin_package))
           .route("/depot/pkgs/{origin}/{pkg}/versions",
                  web::get().to(list_package_versions))
           .route("/depot/pkgs/{origin}/{pkg}/{version}",
                  web::get().to(get_packages_for_origin_package_version))
           .route("/depot/pkgs/{origin}/{pkg}/{version}/latest",
                  web::get().to(get_latest_package_for_origin_package_version))
           .route("/depot/pkgs/{origin}/{pkg}/{version}/{release}",
                  web::post().to(upload_package))
           .route("/depot/pkgs/{origin}/{pkg}/{version}/{release}",
                  web::get().to(get_package))
           .route("/depot/pkgs/{origin}/{pkg}/{version}/{release}",
                  web::delete().to(delete_package))
           .route("/depot/pkgs/{origin}/{pkg}/{version}/{release}/download",
                  web::get().to(download_package))
           .route("/depot/pkgs/{origin}/{pkg}/{version}/{release}/channels",
                  web::get().to(get_package_channels))
           .route("/depot/pkgs/{origin}/{pkg}/{version}/{release}/{visibility}",
                  web::patch().to(package_privacy_toggle));
    }
}

// Route handlers - these functions can return any Responder trait
//

#[allow(clippy::needless_pass_by_value)]
fn get_packages_for_origin(req: HttpRequest,
                           path: Path<String>,
                           pagination: Query<Pagination>)
                           -> HttpResponse {
    let origin = path.into_inner();
    let ident = PackageIdent::new(origin, String::from(""), None, None);

    match do_get_packages(&req, &ident, &pagination) {
        Ok((packages, count)) => {
            postprocess_extended_package_list(&req, &packages, count, &pagination)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_packages_for_origin_package(req: HttpRequest,
                                   path: Path<(String, String)>,
                                   pagination: Query<Pagination>)
                                   -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_packages(&req, &ident, &pagination) {
        Ok((packages, count)) => {
            postprocess_extended_package_list(&req, &packages, count, &pagination)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_packages_for_origin_package_version(req: HttpRequest,
                                           path: Path<(String, String, String)>,
                                           pagination: Query<Pagination>)
                                           -> HttpResponse {
    let (origin, pkg, version) = path.into_inner();

    let ident = PackageIdent::new(origin, pkg, Some(version), None);

    match do_get_packages(&req, &ident, &pagination) {
        Ok((packages, count)) => {
            postprocess_extended_package_list(&req, &packages, count, &pagination)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_latest_package_for_origin_package(req: HttpRequest,
                                         path: Path<(String, String)>,
                                         qtarget: Query<Target>)
                                         -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_package(&req, &qtarget, &ident) {
        Ok(json_body) => {
            HttpResponse::Ok().header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                              .header(http::header::CACHE_CONTROL,
                                      headers::Cache::NoCache.to_string())
                              .body(json_body)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_latest_package_for_origin_package_version(req: HttpRequest,
                                                 path: Path<(String, String, String)>,
                                                 qtarget: Query<Target>)
                                                 -> HttpResponse {
    let (origin, pkg, version) = path.into_inner();

    let ident = PackageIdent::new(origin, pkg, Some(version), None);

    match do_get_package(&req, &qtarget, &ident) {
        Ok(json_body) => {
            HttpResponse::Ok().header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                              .header(http::header::CACHE_CONTROL,
                                      headers::Cache::NoCache.to_string())
                              .body(json_body)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_package(req: HttpRequest,
               path: Path<(String, String, String, String)>,
               qtarget: Query<Target>)
               -> HttpResponse {
    let (origin, pkg, version, release) = path.into_inner();

    let ident = PackageIdent::new(origin, pkg, Some(version), Some(release));

    match do_get_package(&req, &qtarget, &ident) {
        Ok(json_body) => {
            HttpResponse::Ok().header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                              .header(http::header::CACHE_CONTROL,
                                      headers::Cache::default().to_string())
                              .body(json_body)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn delete_package(req: HttpRequest,
                        path: Path<(String, String, String, String)>,
                        qtarget: Query<Target>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let (origin, pkg, version, release) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return err.into();
    }

    let ident = PackageIdent::new(origin, pkg, Some(version), Some(release));

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => {
                    debug!("Invalid target requested: {}, err = {:?}", t, err);
                    return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                }
            }
        }
        None => helpers::target_from_headers(&req),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Check whether package is in stable channel
    match Package::list_package_channels(&BuilderPackageIdent(ident.clone()),
                                         target,
                                         helpers::all_visibilities(),
                                         &*conn)
    {
        Ok(channels) => {
            if channels.iter()
                       .any(|c| c.name == ChannelIdent::stable().to_string())
            {
                debug!("Deleting package in stable channel not allowed: {}", ident);
                return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
            }
        }
        Err(err) => {
            debug!("{}", err);
            return Error::DieselError(err).into();
        }
    }

    // Check whether package project has any rdeps
    if feat::is_enabled(feat::Jobsrv) {
        let mut rdeps_get = jobsrv::JobGraphPackageReverseDependenciesGet::new();
        rdeps_get.set_origin(ident.origin().to_string());
        rdeps_get.set_name(ident.name().to_string());
        rdeps_get.set_target(target.to_string());

        match route_message::<jobsrv::JobGraphPackageReverseDependenciesGet,
                            jobsrv::JobGraphPackageReverseDependencies>(&req, &rdeps_get).await
        {
            Ok(rdeps) => {
                if !rdeps.get_rdeps().is_empty() {
                    debug!("Deleting package with rdeps not allowed: {}", ident);
                    return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                }
            }
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        }
    }

    // TODO (SA): Wrap in transaction, or better yet, eliminate need to do
    // channel package deletion
    let pkg = match Package::get(GetPackage { ident:      BuilderPackageIdent(ident.clone()),
                                              visibility: helpers::all_visibilities(),
                                              target:     BuilderPackageTarget(target), },
                                 &*conn).map_err(Error::DieselError)
    {
        Ok(pkg) => pkg,
        Err(err) => return err.into(),
    };

    if let Err(err) = Channel::delete_channel_package(pkg.id, &*conn).map_err(Error::DieselError) {
        debug!("{}", err);
        return err.into();
    }

    match Package::delete(DeletePackage { ident:  BuilderPackageIdent(ident.clone()),
                                          target: BuilderPackageTarget(target), },
                          &*conn).map_err(Error::DieselError)
    {
        Ok(_) => {
            state.memcache.borrow_mut().clear_cache_for_package(&ident);
            HttpResponse::NoContent().finish()
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// TODO : Convert to async
#[allow(clippy::needless_pass_by_value)]
async fn download_package(req: HttpRequest,
                          path: Path<(String, String, String, String)>,
                          qtarget: Query<Target>,
                          state: Data<AppState>)
                          -> HttpResponse {
    let (origin, name, version, release) = path.into_inner();

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let opt_session_id = match authorize_session(&req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    let ident = PackageIdent::new(origin, name, Some(version), Some(release));

    let mut vis = helpers::visibility_for_optional_session(&req, opt_session_id, &ident.origin);
    vis.push(PackageVisibility::Hidden);

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => {
                    debug!("Invalid target requested: {}, err = {:?}", t, err);
                    return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                }
            }
        }
        None => helpers::target_from_headers(&req),
    };

    if !state.config.api.targets.contains(&target) {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    match Package::get(GetPackage { ident:      BuilderPackageIdent(ident.clone()),
                                    visibility: vis,
                                    target:     BuilderPackageTarget(target), },
                       &*conn)
    {
        Ok(package) => {
            let dir = tempdir_in(&state.config.api.data_path).expect("Unable to create a tempdir!");
            let file_path = dir.path().join(archive_name(&package.ident, target));
            let temp_ident = ident;
            let is_private = package.visibility != PackageVisibility::Public;

            // TODO: Aggregate Artifactory/S3 into a provider model
            if feat::is_enabled(feat::Artifactory) {
                match state.artifactory
                           .download(&file_path, &temp_ident, target)
                           .await
                {
                    Ok(archive) => {
                        download_response_for_archive(&archive, &file_path, is_private, &state)
                    }
                    Err(e) => {
                        warn!("Failed to download package, ident={}, err={:?}",
                              temp_ident, e);
                        HttpResponse::new(StatusCode::NOT_FOUND)
                    }
                }
            } else {
                match state.packages
                           .download(&file_path, &temp_ident, target)
                           .await
                {
                    Ok(archive) => {
                        download_response_for_archive(&archive, &file_path, is_private, &state)
                    }
                    Err(e) => {
                        warn!("Failed to download package, ident={}, err={:?}",
                              temp_ident, e);
                        HttpResponse::new(StatusCode::NOT_FOUND)
                    }
                }
            }
        }
        Err(err) => Error::DieselError(err).into(),
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn upload_package(req: HttpRequest,
                        path: Path<(String, String, String, String)>,
                        qupload: Query<Upload>,
                        stream: web::Payload,
                        state: Data<AppState>)
                        -> Result<HttpResponse> {
    let (origin, name, version, release) = path.into_inner();

    let ident = PackageIdent::new(origin, name, Some(version), Some(release));

    if !ident.valid() || !ident.fully_qualified() {
        info!("Invalid or not fully qualified package identifier: {}",
              ident);
        return Ok(HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY));
    }

    match do_upload_package_start(&req, &qupload, &ident) {
        Ok((temp_path, writer)) => {
            state.memcache.borrow_mut().clear_cache_for_package(&ident);
            do_upload_package_async(req, stream, qupload, ident, temp_path, writer).await
        }
        Err(Error::Conflict) => {
            debug!("Failed to upload package {}, metadata already exists",
                   &ident);
            Ok(HttpResponse::new(StatusCode::CONFLICT))
        }
        Err(err) => {
            warn!("Failed to upload package {}, err={:?}", &ident, err);
            Ok(err.into())
        }
    }
}

// TODO REVIEW: should this path be under jobs instead?
#[allow(clippy::needless_pass_by_value)]
async fn schedule_job_group(req: HttpRequest,
                            path: Path<(String, String)>,
                            qschedule: Query<Schedule>,
                            state: Data<AppState>)
                            -> HttpResponse {
    let (origin_name, package) = path.into_inner();

    let session =
        match authorize_session(&req, Some(&origin_name), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session,
            Err(err) => return err.into(),
        };

    let target = match PackageTarget::from_str(&qschedule.target) {
        Ok(t) => t,
        Err(_) => {
            debug!("Invalid target received: {}", qschedule.target);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    if !state.config.api.build_targets.contains(&target) {
        debug!("Rejecting build with target: {}", qschedule.target);
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    let mut request = jobsrv::JobGroupSpec::new();
    request.set_origin(origin_name);
    request.set_package(package);
    request.set_target(qschedule.target.clone());
    request.set_deps_only(qschedule.deps_only
                                   .clone()
                                   .unwrap_or_else(|| "false".to_string())
                                   .parse()
                                   .unwrap_or(false));
    request.set_origin_only(qschedule.origin_only
                                     .clone()
                                     .unwrap_or_else(|| "false".to_string())
                                     .parse()
                                     .unwrap_or(false));
    request.set_package_only(qschedule.package_only
                                      .clone()
                                      .unwrap_or_else(|| "false".to_string())
                                      .parse()
                                      .unwrap_or(false));
    request.set_trigger(helpers::trigger_from_request(&req));
    request.set_requester_id(session.get_id());
    request.set_requester_name(session.get_name().to_string());

    match route_message::<jobsrv::JobGroupSpec, jobsrv::JobGroup>(&req, &request).await {
        Ok(group) => {
            HttpResponse::Created().header(http::header::CACHE_CONTROL,
                                           headers::Cache::NoCache.to_string())
                                   .json(group)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_schedule(req: HttpRequest,
                      path: Path<String>,
                      qgetschedule: Query<GetSchedule>)
                      -> HttpResponse {
    let group_id_str = path.into_inner();
    let group_id = match group_id_str.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::new(StatusCode::BAD_REQUEST),
    };

    let mut request = jobsrv::JobGroupGet::new();
    request.set_group_id(group_id);
    request.set_include_projects(qgetschedule.include_projects);

    match route_message::<jobsrv::JobGroupGet, jobsrv::JobGroup>(&req, &request).await {
        Ok(group) => {
            HttpResponse::Ok().header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                              .json(group)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_origin_schedule_status(req: HttpRequest,
                                    path: Path<String>,
                                    qoss: Query<OriginScheduleStatus>)
                                    -> HttpResponse {
    let origin = path.into_inner();
    let limit = qoss.limit.parse::<u32>().unwrap_or(10);

    let mut request = jobsrv::JobGroupOriginGet::new();
    request.set_origin(origin);
    request.set_limit(limit);

    match route_message::<jobsrv::JobGroupOriginGet, jobsrv::JobGroupOriginResponse>(&req, &request).await
    {
        Ok(jgor) => {
            HttpResponse::Ok().header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                              .json(jgor.get_job_groups())
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_package_channels(req: HttpRequest,
                        path: Path<(String, String, String, String)>,
                        qtarget: Query<Target>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let (origin, name, version, release) = path.into_inner();

    let opt_session_id = match authorize_session(&req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let ident = PackageIdent::new(origin, name, Some(version), Some(release));

    if !ident.fully_qualified() {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => {
                    debug!("Invalid target requested: {}, err = {:?}", t, err);
                    return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                }
            }
        }
        None => helpers::target_from_headers(&req),
    };

    match Package::list_package_channels(&BuilderPackageIdent(ident.clone()),
                                         target,
                                         helpers::visibility_for_optional_session(&req,
                                                                                  opt_session_id,
                                                                                  &ident.origin),
                                         &*conn)
    {
        Ok(channels) => {
            let list: Vec<String> = channels.iter()
                                            .map(|channel| channel.name.to_string())
                                            .collect();
            HttpResponse::Ok().header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                              .json(list)
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn list_package_versions(req: HttpRequest,
                         path: Path<(String, String)>,
                         state: Data<AppState>)
                         -> HttpResponse {
    let (origin, name) = path.into_inner();

    let opt_session_id = match authorize_session(&req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let ident = PackageIdent::new(origin.to_string(), name, None, None);

    match Package::list_package_versions(&BuilderPackageIdent(ident.clone()),
                                         helpers::visibility_for_optional_session(&req,
                                                                                  opt_session_id,
                                                                                  &origin),
                                         &*conn)
    {
        Ok(packages) => {
            trace!(target: "habitat_builder_api::server::resources::pkgs::versions", "list_package_versions for {} found {} package versions: {:?}", ident, packages.len(), packages);

            let body = serde_json::to_string(&packages).unwrap();
            HttpResponse::Ok().header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                              .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                              .body(body)
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn search_packages(req: HttpRequest,
                   path: Path<String>,
                   pagination: Query<Pagination>,
                   state: Data<AppState>)
                   -> HttpResponse {
    Counter::SearchPackages.increment();

    let query = path.into_inner();

    let opt_session_id = match authorize_session(&req, None, None) {
        Ok(session) => Some(session.get_id() as i64),
        Err(_) => None,
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (page, per_page) = helpers::extract_pagination_in_pages(&pagination);

    // First, try to parse the query like it's a PackageIdent, since it seems reasonable to expect
    // that many people will try searching using that kind of string, e.g. core/redis.  If that
    // works, set the origin appropriately and do a regular search.  If that doesn't work, do a
    // search across all origins, similar to how the "distinct" search works now, but returning all
    // the details instead of just names.
    let decoded_query = match percent_encoding::percent_decode(query.as_bytes()).decode_utf8() {
        Ok(q) => q.to_string().trim_end_matches('/').replace("/", " & "),
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    debug!("search_packages called with: {}", decoded_query);

    if pagination.distinct {
        return match Package::search_distinct(SearchPackages { query:      decoded_query,
                                                               page:       page as i64,
                                                               limit:      per_page as i64,
                                                               account_id: opt_session_id, },
                                              &*conn)
        {
            Ok((packages, count)) => postprocess_package_list(&req, &packages, count, &pagination),
            Err(err) => {
                debug!("{}", err);
                Error::DieselError(err).into()
            }
        };
    }

    match Package::search(SearchPackages { query:      decoded_query,
                                           page:       page as i64,
                                           limit:      per_page as i64,
                                           account_id: opt_session_id, },
                          &*conn)
    {
        Ok((packages, count)) => postprocess_package_list(&req, &packages, count, &pagination),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn package_privacy_toggle(req: HttpRequest,
                          path: Path<(String, String, String, String, String)>,
                          state: Data<AppState>)
                          -> HttpResponse {
    let (origin, name, version, release, visibility) = path.into_inner();

    let ident = PackageIdent::new(origin.clone(), name, Some(version), Some(release));

    if !ident.valid() {
        debug!("Invalid package identifier: {}", ident);
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let pv: PackageVisibility = match visibility.parse() {
        Ok(o) => o,
        Err(err) => {
            debug!("{:?}", err);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // users aren't allowed to set packages to hidden manually
    if visibility.to_lowercase() == "hidden" {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    match Package::update_visibility(pv, BuilderPackageIdent(ident.clone()), &*conn) {
        Ok(_) => {
            trace!("Clearing cache for {}", ident);
            state.memcache.borrow_mut().clear_cache_for_package(&ident);
            HttpResponse::Ok().finish()
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

// Public helpers
//

pub fn postprocess_package_list<T: Serialize>(_req: &HttpRequest,
                                              packages: &[T],
                                              count: i64,
                                              pagination: &Query<Pagination>)
                                              -> HttpResponse {
    let (start, _) = helpers::extract_pagination(pagination);
    let pkg_count = packages.len() as isize;
    let stop = match pkg_count {
        0 => count,
        _ => (start + pkg_count - 1) as i64,
    };

    debug!("postprocessing package list, start: {}, stop: {}, total_count: {}",
           start, stop, count);

    let body =
        helpers::package_results_json(&packages, count as isize, start as isize, stop as isize);

    let mut response = if count as isize > (stop as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response.header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .body(body)
}

pub fn postprocess_extended_package_list(_req: &HttpRequest,
                                         packages: &[PackageIdentWithChannelPlatform],
                                         count: i64,
                                         pagination: &Query<Pagination>)
                                         -> HttpResponse {
    let (start, _) = helpers::extract_pagination(pagination);
    let pkg_count = packages.len() as isize;
    let stop = match pkg_count {
        0 => count,
        _ => (start + pkg_count - 1) as i64,
    };

    debug!("postprocessing extended package list, start: {}, stop: {}, total_count: {}",
           start, stop, count);

    let body =
        helpers::package_results_json(&packages, count as isize, start as isize, stop as isize);

    let mut response = if count as isize > (stop as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response.header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .body(body)
}

// Internal - these functions should return Result<..>
//
fn do_get_packages(req: &HttpRequest,
                   ident: &PackageIdent,
                   pagination: &Query<Pagination>)
                   -> Result<(Vec<PackageIdentWithChannelPlatform>, i64)> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    let (page, per_page) = helpers::extract_pagination_in_pages(pagination);

    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    let lpr = ListPackages { ident:      BuilderPackageIdent(ident.clone()),
                             visibility: helpers::visibility_for_optional_session(&req,
                                                                                  opt_session_id,
                                                                                  &ident.origin),
                             page:       page as i64,
                             limit:      per_page as i64, };

    if pagination.distinct {
        match Package::list_distinct(lpr, &*conn).map_err(Error::DieselError) {
            Ok((packages, count)) => {
                let ident_pkgs: Vec<PackageIdentWithChannelPlatform> =
                    packages.into_iter().map(|p| p.into()).collect();
                return Ok((ident_pkgs, count));
            }
            Err(e) => return Err(e),
        }
    }

    match Package::list(lpr, &*conn).map_err(Error::DieselError) {
        Ok((packages, count)) => {
            let ident_pkgs: Vec<PackageIdentWithChannelPlatform> =
                packages.into_iter().map(|p| p.into()).collect();

            trace!(target: "habitat_builder_api::server::resources::pkgs::versions", "do_get_packages for {}, got {} packages, idents: {:?}", ident, count, ident_pkgs);

            Ok((ident_pkgs, count))
        }
        Err(e) => Err(e),
    }
}

//  Async helpers
//
fn do_upload_package_start(req: &HttpRequest,
                           qupload: &Query<Upload>,
                           ident: &PackageIdent)
                           -> Result<(PathBuf, BufWriter<File>)> {
    authorize_session(req, Some(&ident.origin), Some(OriginMemberRole::Maintainer))?;

    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    if qupload.forced {
        debug!("Upload was forced (bypassing existing package check) for: {}",
               ident);
    } else {
        let target = match qupload.target {
            Some(ref t) => {
                trace!("Query requested target = {}", t);
                PackageTarget::from_str(t)?
            }
            None => helpers::target_from_headers(req),
        };

        match Package::get(
            GetPackage {
                ident: BuilderPackageIdent(ident.clone()),
                visibility: helpers::all_visibilities(),
                target: BuilderPackageTarget(PackageTarget::from_str(&target).unwrap()), // Unwrap OK
            },
            &*conn,
        ) {
            Ok(_) => return Err(Error::Conflict),
            Err(NotFound) => {}
            Err(err) => return Err(err.into()),
        }
    }

    debug!("UPLOADING {}, params={:?}", ident, qupload);

    // Create a temp file at the data path
    let temp_name = format!("{}.tmp", Uuid::new_v4());
    let temp_path = req_state(req).config.api.data_path.join(temp_name);

    let file = File::create(&temp_path)?;
    let writer = BufWriter::new(file);

    Ok((temp_path, writer))
}

// TODO: Break this up further, convert S3 upload to async
#[allow(clippy::cognitive_complexity)]
async fn do_upload_package_finish(req: &HttpRequest,
                                  qupload: &Query<Upload>,
                                  ident: &PackageIdent,
                                  temp_path: &PathBuf)
                                  -> HttpResponse {
    let mut archive = PackageArchive::new(&temp_path);

    debug!("Package Archive: {:#?}", archive);

    let target_from_artifact = match archive.target() {
        Ok(target) => target,
        Err(e) => {
            info!("Could not read the target for {:#?}: {:#?}", archive, e);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                           Body::from_message(format!("ds:up:1, err={:?}", e)));
        }
    };

    if !req_state(req).config
                      .api
                      .targets
                      .contains(&target_from_artifact)
    {
        debug!("Unsupported package platform or architecture {}.",
               target_from_artifact);
        return HttpResponse::new(StatusCode::NOT_IMPLEMENTED);
    };

    let checksum_from_artifact = match archive.checksum() {
        Ok(cksum) => cksum,
        Err(e) => {
            debug!("Could not compute a checksum for {:#?}: {:#?}", archive, e);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                           Body::from_message("ds:up:2"));
        }
    };

    if qupload.checksum != checksum_from_artifact {
        debug!("Checksums did not match: from_param={:?}, from_artifact={:?}",
               qupload.checksum, checksum_from_artifact);
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                       Body::from_message("ds:up:3"));
    }

    let conn = match req_state(req).db.get_conn().map_err(Error::DbError) {
        Ok(conn) => conn,
        Err(err) => return err.into(),
    };

    // If upload was forced, and a previously uploaded package exists in DB
    // make sure the checksums match the original (idempotency)
    if qupload.forced {
        match Package::get(
            GetPackage {
                ident: BuilderPackageIdent(ident.clone()),
                visibility: helpers::all_visibilities(),
                target: BuilderPackageTarget(PackageTarget::from_str(&target_from_artifact).unwrap()), // Unwrap OK
            },
            &*conn,
        ) {
            Ok(pkg) => {
                if qupload.checksum != pkg.checksum {
                    debug!("Checksums did not match: from_param={:?}, from_database={:?}",
                           qupload.checksum, pkg.checksum);
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                                   Body::from_message("ds:up:4"));
                }
            }
            Err(NotFound) => {}
            Err(err) => return Error::DieselError(err).into(),
        }
    }

    // Check with scheduler to ensure we don't have circular deps, if configured
    if feat::is_enabled(feat::Jobsrv) {
        match has_circular_deps(&req, ident, target_from_artifact, &mut archive).await {
            Ok(val) if val => return HttpResponse::new(StatusCode::FAILED_DEPENDENCY),
            Err(err) => return err.into(),
            _ => (),
        }
    }

    let file_path = &req_state(req).config.api.data_path;
    let filename = file_path.join(archive_name(&ident, target_from_artifact));
    let temp_ident = ident.to_owned();

    match fs::rename(&temp_path, &filename) {
        Ok(_) => {}
        Err(e) => {
            warn!("Unable to rename temp archive {:?} to {:?}, err={:?}",
                  temp_path, filename, e);
            return Error::IO(e).into();
        }
    }

    // TODO: Make upload async
    // TODO: Aggregate Artifactory/S3 into a provider model
    if feat::is_enabled(feat::Artifactory) {
        if let Err(err) = req_state(req).artifactory
                                        .upload(&filename, &temp_ident, target_from_artifact)
                                        .await
                                        .map_err(Error::Artifactory)
        {
            warn!("Unable to upload archive to artifactory!");
            return err.into();
        }
    } else if let Err(err) = req_state(req).packages
                                           .upload(&filename, &temp_ident, target_from_artifact)
                                           .await
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
        debug!("Ident mismatch, expected={:?}, got={:?}",
               ident, package.ident);

        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                       Body::from_message("ds:up:6"));
    }

    let session = authorize_session(&req, None, None).unwrap(); // Unwrap Ok

    package.owner_id = session.get_id() as i64;
    package.origin = ident.clone().origin;

    package.visibility =
        match OriginPackageSettings::get(&GetOriginPackageSettings { origin: &package.origin,
                                                                     name:   &package.name, },
                                         &*conn)
        {
            // TED if this is in-fact optional in the db it should be an option in the model
            Ok(pkg) => pkg.visibility,
            Err(_) => {
                match Origin::get(&ident.origin, &*conn) {
                    Ok(o) => {
                        match OriginPackageSettings::create(&NewOriginPackageSettings {
                            origin: &ident.origin,
                            name: &ident.name,
                            visibility: &o.default_package_visibility,
                            owner_id: package.owner_id,
                        }, &*conn) {
                            Ok(pkg_settings) => pkg_settings.visibility,
                            Err(err) => return Error::DieselError(err).into(),
                        }
                    },
                    Err(err) => return Error::DieselError(err).into(),
                }
            }
        };

    // Re-create origin package as needed (eg, checksum update)
    match Package::create(&package, &*conn) {
        Ok(pkg) => {
            if feat::is_enabled(feat::Jobsrv) {
                let mut job_graph_package = jobsrv::JobGraphPackageCreate::new();
                job_graph_package.set_package(pkg.into());

                match route_message::<jobsrv::JobGraphPackageCreate, originsrv::OriginPackage>(
                    &req,
                    &job_graph_package,
                ).await {
                    Ok(_) => (),
                    Err(Error::BuilderCore(RpcError(code, _)))
                        if StatusCode::from_u16(code).unwrap() == StatusCode::NOT_FOUND =>
                    {
                        debug!(
                            "Graph not found for package target: {}",
                            target_from_artifact
                        );
                    }
                    Err(err) => {
                        warn!("Failed to create job graph package, err={:?}", err);
                        return err.into();
                    }
                }
            }
        }
        Err(NotFound) => {
            debug!("Package::create returned NotFound (DB conflict handled)");
        }
        Err(err) => {
            debug!("Failed to create package in DB, err: {:?}", err);
            return Error::DieselError(err).into();
        }
    }

    // Schedule re-build of dependent packages (if requested)
    // Don't schedule builds if the upload is being done by the builder
    if qupload.builder.is_none()
       && feat::is_enabled(feat::Jobsrv)
       && req_state(req).config.api.build_on_upload
    {
        let mut request = jobsrv::JobGroupSpec::new();
        request.set_origin(ident.origin.to_string());
        request.set_package(ident.name.to_string());
        request.set_target(target_from_artifact.to_string());
        request.set_deps_only(true);
        request.set_origin_only(false);
        request.set_package_only(false);
        request.set_trigger(jobsrv::JobGroupTrigger::Upload);
        request.set_requester_id(session.get_id());
        request.set_requester_name(session.get_name().to_string());

        match route_message::<jobsrv::JobGroupSpec, jobsrv::JobGroup>(&req, &request).await {
            Ok(group) => {
                debug!("Scheduled reverse dependecy build for {}, group id: {}",
                       ident,
                       group.get_id())
            }
            Err(Error::BuilderCore(RpcError(code, _)))
                if StatusCode::from_u16(code).unwrap() == StatusCode::NOT_FOUND =>
            {
                debug!("Unable to schedule build for {} (not found)", ident)
            }
            Err(err) => warn!("Unable to schedule build for {}, err: {:?}", ident, err),
        }
    }

    match remove_file(&filename) {
        Ok(_) => {
            debug!("Successfully removed cached file after upload. {:?}",
                   &filename)
        }
        Err(e) => {
            warn!("Failed to remove cached file after upload: {:?}, {}",
                  &filename, e)
        }
    }

    HttpResponse::Created().header(http::header::LOCATION, format!("{}", req.uri()))
                           .body(format!("/pkgs/{}/download", *package.ident))
}

async fn do_upload_package_async(req: HttpRequest,
                                 mut stream: web::Payload,
                                 qupload: Query<Upload>,
                                 ident: PackageIdent,
                                 temp_path: PathBuf,
                                 mut writer: BufWriter<File>)
                                 -> Result<HttpResponse> {
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        debug!("Writing file upload chunk, size: {}", chunk.len());
        writer = web::block(move || writer.write(&chunk).map(|_| writer)).await?;
    }

    match writer.into_inner() {
        Ok(f) => {
            f.sync_all()?;
            Ok(do_upload_package_finish(&req, &qupload, &ident, &temp_path).await)
        }
        Err(err) => Err(Error::InnerError(err)),
    }
}

fn do_get_package(req: &HttpRequest,
                  qtarget: &Query<Target>,
                  ident: &PackageIdent)
                  -> Result<String> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };
    Counter::GetPackage.increment();

    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            PackageTarget::from_str(&t)?
        }
        None => helpers::target_from_headers(req),
    };

    // Scope this memcache usage so the reference goes out of
    // scope before the visibility_for_optional_session call
    // below
    {
        let mut memcache = req_state(req).memcache.borrow_mut();
        match memcache.get_package(&ident, &ChannelIdent::unstable(), &target, opt_session_id) {
            (true, Some(pkg_json)) => {
                trace!("Package {} {} {:?} - cache hit with pkg json",
                       ident,
                       target,
                       opt_session_id);
                // Note: the Package specifier is needed even though the variable is un-used
                let _p: Package = match serde_json::from_str(&pkg_json) {
                    Ok(p) => p,
                    Err(e) => {
                        debug!("Unable to deserialize package json, err={:?}", e);
                        return Err(Error::SerdeJson(e));
                    }
                };
                Counter::MemcachePackageHit.increment();
                return Ok(pkg_json);
            }
            (true, None) => {
                trace!("Channel package {} {} {:?} - cache hit with 404",
                       ident,
                       target,
                       opt_session_id);
                Counter::MemcachePackage404.increment();
                return Err(Error::NotFound);
            }
            (false, _) => {
                trace!("Channel package {} {} {:?} - cache miss",
                       ident,
                       target,
                       opt_session_id);
                Counter::MemcachePackageMiss.increment();
            }
        };
    }

    let pkg = if ident.fully_qualified() {
        match Package::get_without_target(BuilderPackageIdent(ident.clone()),
                                          helpers::visibility_for_optional_session(req,
                                                                                   opt_session_id,
                                                                                   &ident.origin),
                                          &*conn)
        {
            Ok(pkg) => pkg,
            Err(NotFound) => {
                let mut memcache = req_state(req).memcache.borrow_mut();
                memcache.set_package(&ident,
                                     None,
                                     &ChannelIdent::unstable(),
                                     &target,
                                     opt_session_id);
                return Err(Error::NotFound);
            }

            Err(err) => {
                debug!("{:?}", err);
                return Err(err.into());
            }
        }
    } else {
        match Package::get_latest(
            GetLatestPackage {
                ident: BuilderPackageIdent(ident.clone()),
                target: BuilderPackageTarget(target),
                visibility: helpers::visibility_for_optional_session(
                    req,
                    opt_session_id,
                    &ident.origin,
                ),
            },
            &*conn,
        ) {
            Ok(pkg) => pkg.into(),
            Err(NotFound) => {
                let mut memcache = req_state(req).memcache.borrow_mut();
                memcache.set_package(
                    &ident,
                    None,
                    &ChannelIdent::unstable(),
                    &target,
                    opt_session_id,
                );
                return Err(Error::NotFound);
            }
            Err(err) => {
                debug!("{:?}", err);
                return Err(err.into());
            }
        }
    };

    let mut pkg_json = serde_json::to_value(pkg.clone()).unwrap();
    let channels = channels_for_package_ident(req, &pkg.ident.clone(), target, &*conn)?;

    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(pkg.is_a_service());

    let json_body = serde_json::to_string(&pkg_json).unwrap();

    {
        let mut memcache = req_state(req).memcache.borrow_mut();
        memcache.set_package(&ident,
                             Some(&json_body),
                             &ChannelIdent::unstable(),
                             &target,
                             opt_session_id);
    }

    Ok(json_body)
}

// Internal helpers
//

// Return a formatted string representing the filename of an archive for the given package
// identifier pieces.
fn archive_name(ident: &PackageIdent, target: PackageTarget) -> PathBuf {
    PathBuf::from(ident.archive_name_with_target(target).unwrap_or_else(|_| {
                                                            panic!("Package ident should be fully \
                                                                    qualified, ident={}",
                                                                   &ident)
                                                        }))
}

fn download_response_for_archive(archive: &PackageArchive,
                                 file_path: &PathBuf,
                                 is_private: bool,
                                 state: &Data<AppState>)
                                 -> HttpResponse {
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
    let cache_hdr = if is_private {
        headers::Cache::MaxAge(state.config.api.private_max_age).to_string()
    } else {
        headers::Cache::default().to_string()
    };

    #[allow(clippy::redundant_closure)] //  Ok::<_, ()>
    HttpResponse::Ok().header(http::header::CONTENT_DISPOSITION,
            ContentDisposition { disposition: DispositionType::Attachment,
                                 parameters:  vec![DispositionParam::Filename(filename)], })
    .header(http::header::HeaderName::from_static(headers::XFILENAME),
            archive.file_name())
    .set(ContentType::octet_stream())
    .header(http::header::CACHE_CONTROL, cache_hdr)
    .streaming(rx_body.map(|s| Ok::<_, ()>(s)))
}

async fn has_circular_deps(req: &HttpRequest,
                           ident: &PackageIdent,
                           target: PackageTarget,
                           archive: &mut PackageArchive)
                           -> Result<bool> {
    let mut pcr_req = jobsrv::JobGraphPackagePreCreate::new();
    pcr_req.set_ident(format!("{}", ident));
    pcr_req.set_target(target.to_string());

    let mut pcr_deps = protobuf::RepeatedField::new();
    let mut pcr_build_deps = protobuf::RepeatedField::new();

    let build_deps_from_artifact = match archive.build_deps() {
        Ok(build_deps) => build_deps,
        Err(e) => {
            debug!("Could not get build deps from {:#?}: {:#?}", archive, e);
            return Err(Error::HabitatCore(e));
        }
    };

    let deps_from_artifact = match archive.deps() {
        Ok(deps) => deps,
        Err(e) => {
            debug!("Could not get deps from {:#?}: {:#?}", archive, e);
            return Err(Error::HabitatCore(e));
        }
    };
    for ident in build_deps_from_artifact {
        let dep_str = format!("{}", ident);
        pcr_build_deps.push(dep_str);
    }
    pcr_req.set_build_deps(pcr_build_deps);

    for ident in deps_from_artifact {
        let dep_str = format!("{}", ident);
        pcr_deps.push(dep_str);
    }
    pcr_req.set_deps(pcr_deps);

    match route_message::<jobsrv::JobGraphPackagePreCreate, NetOk>(req, &pcr_req).await {
        Ok(_) => Ok(false),
        Err(Error::BuilderCore(RpcError(code, _)))
            if StatusCode::from_u16(code).unwrap() == StatusCode::CONFLICT =>
        {
            debug!("Failed package circular dependency check for {}", ident);
            Ok(true)
        }
        Err(Error::BuilderCore(RpcError(code, _)))
            if StatusCode::from_u16(code).unwrap() == StatusCode::NOT_FOUND =>
        {
            debug!("Graph not found for package target: {}", target);
            Ok(false)
        }
        Err(err) => Err(err),
    }
}

pub fn platforms_for_package_ident(req: &HttpRequest,
                                   package: &BuilderPackageIdent)
                                   -> Result<Option<Vec<String>>> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    let conn = req_state(req).db.get_conn()?;

    match Package::list_package_platforms(&package,
                                          helpers::visibility_for_optional_session(req,
                                                                                   opt_session_id,
                                                                                   &package.origin),
                                          &*conn)
    {
        Ok(list) => Ok(Some(list.iter().map(|p| p.to_string()).collect())),
        Err(NotFound) => Ok(None),
        Err(err) => Err(Error::DieselError(err)),
    }
}
