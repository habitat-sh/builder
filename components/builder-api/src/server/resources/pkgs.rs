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
use std::path::{self, PathBuf};
use std::str::FromStr;

use actix_web::http::header::{Charset, ContentDisposition, DispositionParam, DispositionType};
use actix_web::http::{self, StatusCode};
use actix_web::FromRequest;
use actix_web::{error, App, AsyncResponder, HttpMessage, HttpRequest, HttpResponse, Path, Query};
use bytes::Bytes;
use futures::sync::mpsc;
use futures::{future::ok as fut_ok, Future, Stream};
use protobuf;
use serde_json;
use tempfile::tempdir_in;
use url;
use uuid::Uuid;

use bldr_core::metrics::CounterMetric;
use hab_core::package::{FromArchive, Identifiable, PackageArchive, PackageIdent, PackageTarget};
use hab_net::{ErrCode, NetError, NetOk, NetResult};

use protocol::jobsrv::*;
use protocol::originsrv::*;

use server::error::{Error, Result};
use server::feat;
use server::framework::headers;
use server::framework::middleware::{route_message, Authenticated, Optional};
use server::helpers::{self, Pagination, Target};
use server::services::metrics::Counter;
use server::services::route_broker::RouteBroker;
use server::AppState;

// Query param containers
#[derive(Debug, Deserialize)]
pub struct Upload {
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
    // Internal - these functions should return Result<..>
    //
    fn do_get_stats(req: &HttpRequest<AppState>, origin: String) -> Result<JobGraphPackageStats> {
        let mut request = JobGraphPackageStatsGet::new();
        request.set_origin(origin);

        route_message::<JobGraphPackageStatsGet, JobGraphPackageStats>(req, &request)
    }

    fn do_get_packages(
        req: &HttpRequest<AppState>,
        ident: OriginPackageIdent,
        pagination: &Query<Pagination>,
    ) -> Result<OriginPackageListResponse> {
        let opt_session_id = helpers::get_optional_session_id(&req);

        let (start, stop) = helpers::extract_pagination(pagination);

        let mut request = OriginPackageListRequest::new();
        request.set_start(start as u64);
        request.set_stop(stop as u64);
        request.set_visibilities(helpers::visibility_for_optional_session(
            &req,
            opt_session_id,
            &ident.get_origin(),
        ));
        request.set_distinct(pagination.distinct);
        request.set_ident(ident);

        route_message::<OriginPackageListRequest, OriginPackageListResponse>(&req, &request)
    }

    // TODO : this needs to be re-designed to not fan out
    // Common functionality for pkgs and channel routes
    pub fn postprocess_package_list(
        req: &HttpRequest<AppState>,
        oplr: &OriginPackageListResponse,
        distinct: bool,
    ) -> HttpResponse {
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

    //
    //  Async helpers
    //
    fn do_upload_package_start(
        req: &HttpRequest<AppState>,
        qupload: &Query<Upload>,
        ident: &OriginPackageIdent,
    ) -> Result<(PathBuf, BufWriter<File>)> {
        helpers::check_origin_access(req, &ident.get_origin())?;

        // Return if we find existing package metadata (unless forced flag specified)
        let mut ident_req = OriginPackageGet::new();
        ident_req.set_ident(ident.clone());
        ident_req.set_visibilities(helpers::all_visibilities());

        if qupload.forced {
            debug!(
                "Upload was forced (bypassing existing package check) for: {}",
                ident
            );
        } else {
            match route_message::<OriginPackageGet, OriginPackage>(&req, &ident_req) {
                Ok(_) => {
                    return Err(Error::NetError(NetError::new(
                        ErrCode::ENTITY_CONFLICT,
                        "ds:up:0",
                    )))
                }
                Err(Error::NetError(ref err)) if err.get_code() == ErrCode::ENTITY_NOT_FOUND => (),
                Err(err) => return Err(err),
            }
        }

        debug!("UPLOADING {}, params={:?}", ident, qupload);

        // Find the path to folder where archive should be created, and
        // create the folder if necessary
        let parent_path = packages_path(&req.state().config.api.data_path);
        fs::create_dir_all(parent_path.clone())?;

        // Create a temp file at the archive location
        let file_path = packages_path(&req.state().config.api.data_path);

        let temp_name = format!("{}.tmp", Uuid::new_v4());
        let temp_path = parent_path.join(file_path).join(temp_name);

        let file = File::create(&temp_path)?;
        let writer = BufWriter::new(file);

        Ok((temp_path, writer))
    }

    // TODO: Break this up further, convert S3 upload to async
    fn do_upload_package_finish(
        req: HttpRequest<AppState>,
        qupload: Query<Upload>,
        ident: OriginPackageIdent,
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
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        };

        let mut ident_req = OriginPackageGet::new();
        ident_req.set_ident(ident.clone());
        ident_req.set_visibilities(helpers::all_visibilities());

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

        // Check with scheduler to ensure we don't have circular deps, if configured
        if feat::is_enabled(feat::Jobsrv) {
            if let Err(e) = check_circular_deps(&req, &ident, &target_from_artifact, &mut archive) {
                debug!("Failed circular dependency check, err={:?}", e);
                return e.into();
            }
        }

        let file_path = packages_path(&req.state().config.api.data_path);
        let filename = file_path.join(archive_name(&(&ident).into(), &target_from_artifact));
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
        let mut package = match OriginPackageCreate::from_archive(&mut archive) {
            Ok(package) => package,
            Err(e) => {
                debug!("Error building package from archive: {:#?}", e);
                return Error::HabitatCore(e).into();
            }
        };

        if !ident.satisfies(package.get_ident()) {
            debug!(
                "Ident mismatch, expected={:?}, got={:?}",
                ident,
                package.get_ident()
            );

            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "ds:up:6");
        }

        let (session_id, session_name) = helpers::get_session_id_and_name(&req);

        match process_upload_for_package_archive(&ident, &mut package, session_id) {
            Ok(_) => {
                // Schedule re-build of dependent packages (if requested)
                // Don't schedule builds if the upload is being done by the builder
                if qupload.builder.is_none() && feat::is_enabled(feat::Jobsrv) {
                    let mut request = JobGroupSpec::new();
                    request.set_origin(ident.get_origin().to_string());
                    request.set_package(ident.get_name().to_string());
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
                        Err(err) => warn!("Unable to schedule build, err: {:?}", err),
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

                //let base_url = req.uri();
                HttpResponse::Created()
                    // TODO: Request URI does not have scheme/host - do we need this?
                    // .header(
                    //     http::header::LOCATION,
                    //     format!(
                    //         "{}://{}/{}",
                    //         base_url.scheme().unwrap(),
                    //         base_url.host().unwrap(),
                    //         format!("v1/pkgs/{}/download", package.get_ident())
                    //     ),
                    // )
                    .finish()
            }
            Err(err) => {
                debug!(
                    "Ident mismatch, expected={:?}, got={:?}",
                    ident,
                    package.get_ident()
                );
                Error::NetError(err).into()
            }
        }
    }

    fn do_upload_package_async(
        req: HttpRequest<AppState>,
        qupload: Query<Upload>,
        ident: OriginPackageIdent,
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
                    Ok(Self::do_upload_package_finish(req, qupload, ident, temp_path))
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
        mut ident: OriginPackageIdent,
    ) -> Result<OriginPackage> {
        let session_id = helpers::get_optional_session_id(&req);

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
            let mut request = OriginPackageLatestGet::new();
            request.set_ident(ident.clone());
            request.set_target(target.to_string());
            request.set_visibilities(helpers::visibility_for_optional_session(
                req,
                session_id,
                &ident.get_origin(),
            ));

            ident = match route_message::<OriginPackageLatestGet, OriginPackageIdent>(req, &request)
            {
                Ok(id) => id.into(),
                Err(err) => {
                    // Notify upstream with a non-fully qualified ident to handle checking
                    // of a package that does not exist in the on-premise depot

                    // TODO: notify_upstream(req, &ident, &target);
                    return Err(err);
                }
            }
        }

        let mut request = OriginPackageGet::new();
        request.set_visibilities(helpers::visibility_for_optional_session(
            req,
            session_id,
            &ident.get_origin(),
        ));
        request.set_ident(ident.clone());

        // Notify upstream with a fully qualified ident
        // TODO: notify_upstream(req, &ident, &target);

        route_message::<OriginPackageGet, OriginPackage>(req, &request)
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

    //
    // Route handlers - these functions should return HttpResponse
    //
    fn get_stats(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok

        match Self::do_get_stats(req, origin) {
            Ok(stats) => HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(stats),
            Err(err) => err.into(),
        }
    }

    fn get_packages_for_origin(
        (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
    ) -> HttpResponse {
        let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
        let ident = OriginPackageIdent::from_str(origin.as_str()).unwrap();

        match Self::do_get_packages(&req, ident, &pagination) {
            Ok(olpr) => Self::postprocess_package_list(&req, &olpr, pagination.distinct),
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

        match Self::do_get_packages(&req, ident, &pagination) {
            Ok(olpr) => Self::postprocess_package_list(&req, &olpr, pagination.distinct),
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

        match Self::do_get_packages(&req, ident, &pagination) {
            Ok(olpr) => Self::postprocess_package_list(&req, &olpr, pagination.distinct),
            Err(err) => err.into(),
        }
    }

    fn get_latest_package_for_origin_package(
        (qtarget, req): (Query<Target>, HttpRequest<AppState>),
    ) -> HttpResponse {
        let (origin, pkg) = Path::<(String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        let mut ident = OriginPackageIdent::new();
        ident.set_origin(origin);
        ident.set_name(pkg);

        match Self::do_get_package(&req, &qtarget, ident) {
            Ok(package) => Self::postprocess_package(&req, &package, false),
            Err(err) => err.into(),
        }
    }

    fn get_latest_package_for_origin_package_version(
        (qtarget, req): (Query<Target>, HttpRequest<AppState>),
    ) -> HttpResponse {
        let (origin, pkg, version) = Path::<(String, String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        let mut ident = OriginPackageIdent::new();
        ident.set_origin(origin);
        ident.set_name(pkg);
        ident.set_version(version);

        match Self::do_get_package(&req, &qtarget, ident) {
            Ok(package) => Self::postprocess_package(&req, &package, false),
            Err(err) => err.into(),
        }
    }

    fn get_latest_package((qtarget, req): (Query<Target>, HttpRequest<AppState>)) -> HttpResponse {
        let (origin, pkg, version, release) = Path::<(String, String, String, String)>::extract(
            &req,
        ).unwrap()
            .into_inner(); // Unwrap Ok

        let mut ident = OriginPackageIdent::new();
        ident.set_origin(origin);
        ident.set_name(pkg);
        ident.set_version(version);
        ident.set_release(release);

        match Self::do_get_package(&req, &qtarget, ident) {
            Ok(package) => Self::postprocess_package(&req, &package, true),
            Err(err) => err.into(),
        }
    }

    // TODO : Convert to async
    fn download_package((qtarget, req): (Query<Target>, HttpRequest<AppState>)) -> HttpResponse {
        let (origin, name, version, release) = Path::<(String, String, String, String)>::extract(
            &req,
        ).unwrap()
            .into_inner(); // Unwrap Ok

        let session_id = helpers::get_optional_session_id(&req);
        let mut ident_req = OriginPackageGet::new();

        let mut ident = OriginPackageIdent::new();
        ident.set_origin(origin.to_string());
        ident.set_name(name.to_string());
        ident.set_version(version.to_string());
        ident.set_release(release.to_string());

        let mut vis =
            helpers::visibility_for_optional_session(&req, session_id, &ident.get_origin());
        vis.push(OriginPackageVisibility::Hidden);
        ident_req.set_visibilities(vis);
        ident_req.set_ident(ident.clone());

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

        match route_message::<OriginPackageGet, OriginPackage>(&req, &ident_req) {
            Ok(package) => {
                let dir = tempdir_in(packages_path(&req.state().config.api.data_path))
                    .expect("Unable to create a tempdir!");
                //let dir = packages_path(&req.state().config.api.data_path);
                let file_path = dir.path().join(archive_name(&(&package).into(), &target));
                let temp_ident = ident.to_owned().into();
                match req
                    .state()
                    .packages
                    .download(&file_path, &temp_ident, &target)
                {
                    Ok(archive) => download_response_for_archive(archive, file_path),
                    Err(e) => {
                        warn!("Failed to download package, err={:?}", e);
                        return HttpResponse::new(StatusCode::NOT_FOUND);
                    }
                }
            }
            Err(err) => err.into(),
        }
    }

    fn upload_package(
        (qupload, req): (Query<Upload>, HttpRequest<AppState>),
    ) -> Box<Future<Item = HttpResponse, Error = Error>> {
        let (origin, name, version, release) = Path::<(String, String, String, String)>::extract(
            &req,
        ).unwrap()
            .into_inner(); // Unwrap Ok

        let mut ident = OriginPackageIdent::new();
        ident.set_origin(origin);
        ident.set_name(name);
        ident.set_version(version);
        ident.set_release(release);

        if !ident.valid() || !ident.fully_qualified() {
            info!(
                "Invalid or not fully qualified package identifier: {}",
                ident
            );
            return Box::new(fut_ok(HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY)));
        }

        match Self::do_upload_package_start(&req, &qupload, &ident) {
            Ok((temp_path, writer)) => {
                Self::do_upload_package_async(req, qupload, ident, temp_path, writer)
            }
            Err(err) => {
                warn!("Failed to upload package, err={:?}", err);
                Box::new(fut_ok(err.into()))
            }
        }
    }

    // TODO REVIEW: should this path be under jobs instead?
    fn schedule_job_group(
        (qschedule, req): (Query<Schedule>, HttpRequest<AppState>),
    ) -> HttpResponse {
        let (origin_name, package) = Path::<(String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin_name).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let (session_id, session_name) = helpers::get_session_id_and_name(&req);

        // We only support building for Linux x64 only currently
        if qschedule.target != "x86_64-linux" {
            info!("Rejecting build with target: {}", qschedule.target);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
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

        let account_name = session_name.clone();
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
            if let Err(err) = helpers::generate_origin_keys(&req, session_id, origin) {
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
        request.set_requester_id(session_id);
        request.set_requester_name(session_name);

        match route_message::<JobGroupSpec, JobGroup>(&req, &request) {
            Ok(group) => {
                let msg = format!("Scheduled job group for {}", group.get_project_name());

                // We don't really want to abort anything just because a call to segment failed. Let's
                // just log it and move on.
                if let Err(e) = req.state().segment.track(&account_name, &msg) {
                    warn!("Error tracking scheduling of job group in segment, {}", e);
                }

                HttpResponse::Ok()
                    .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                    .json(group)
            }
            Err(err) => err.into(),
        }
    }

    fn get_schedule(
        (qgetschedule, req): (Query<GetSchedule>, HttpRequest<AppState>),
    ) -> HttpResponse {
        let group_id_str = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
        let group_id = match group_id_str.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
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
                .json(jgor),
            Err(err) => err.into(),
        }
    }

    fn get_package_channels(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, name, version, release) = Path::<(String, String, String, String)>::extract(
            &req,
        ).unwrap()
            .into_inner(); // Unwrap Ok

        let session_id = helpers::get_optional_session_id(req);
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
                HttpResponse::Ok()
                    .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                    .body(body)
            }
            Err(err) => err.into(),
        }
    }

    fn list_package_versions(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, name) = Path::<(String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        let session_id = helpers::get_optional_session_id(req);

        let mut request = OriginPackageVersionListRequest::new();
        request.set_visibilities(helpers::visibility_for_optional_session(
            req, session_id, &origin,
        ));
        request.set_origin(origin);
        request.set_name(name);

        match route_message::<OriginPackageVersionListRequest, OriginPackageVersionListResponse>(
            req, &request,
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

    fn search_packages(
        (pagination, req): (Query<Pagination>, HttpRequest<AppState>),
    ) -> HttpResponse {
        Counter::SearchPackages.increment();

        let query = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
        let session_id = helpers::get_optional_session_id(&req);
        let (start, stop) = helpers::extract_pagination(&pagination);

        let mut request = OriginPackageSearchRequest::new();
        request.set_start(start as u64);
        request.set_stop(stop as u64);

        if session_id.is_some() {
            let mut my_origins = MyOriginsRequest::new();
            my_origins.set_account_id(session_id.unwrap());

            match route_message::<MyOriginsRequest, MyOriginsResponse>(&req, &my_origins) {
                Ok(response) => request.set_my_origins(protobuf::RepeatedField::from_vec(
                    response.get_origins().to_vec(),
                )),
                Err(e) => {
                    debug!(
                        "Error fetching origins for account id {}, {}",
                        session_id.unwrap(),
                        e
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
        let decoded_query =
            match url::percent_encoding::percent_decode(query.as_bytes()).decode_utf8() {
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

        match route_message::<OriginPackageSearchRequest, OriginPackageListResponse>(&req, &request)
        {
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

    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.resource("/depot/pkgs/origins/{origin}/stats", |r| {
            r.get().f(Self::get_stats)
        }).resource("/depot/pkgs/{origin}", |r| {
                r.middleware(Optional);
                r.method(http::Method::GET)
                    .with(Self::get_packages_for_origin);
            })
            .resource("/depot/pkgs/{origin}/{pkg}", |r| {
                r.middleware(Optional);
                r.method(http::Method::GET)
                    .with(Self::get_packages_for_origin_package);
            })
            .resource("/depot/pkgs/{origin}/{pkg}/{version}", |r| {
                r.middleware(Optional);
                r.method(http::Method::GET)
                    .with(Self::get_packages_for_origin_package_version);
            })
            .resource("/depot/pkgs/{origin}/{pkg}/latest", |r| {
                r.middleware(Optional);
                r.method(http::Method::GET)
                    .with(Self::get_latest_package_for_origin_package);
            })
            .resource("/depot/pkgs/{origin}/{pkg}/{version}/latest", |r| {
                r.middleware(Optional);
                r.method(http::Method::GET)
                    .with(Self::get_latest_package_for_origin_package_version);
            })
            .resource("/depot/pkgs/{origin}/{pkg}/{version}/{release}", |r| {
                r.middleware(Optional);
                r.method(http::Method::GET).with(Self::get_latest_package);
            })
            .resource(
                "/depot/pkgs/{origin}/{pkg}/{version}/{release}/download",
                |r| {
                    r.middleware(Optional);
                    r.method(http::Method::GET).with(Self::download_package);
                },
            )
            .resource("/depot/pkgs/{origin}/{pkg}/{version}/{release}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).with(Self::upload_package);
            })
            .resource("/depot/pkgs/{origin}/{pkg}/versions", |r| {
                r.middleware(Optional);
                r.get().f(Self::list_package_versions);
            })
            .resource("/depot/pkgs/search/{query}", |r| {
                r.middleware(Optional);
                r.method(http::Method::POST).with(Self::search_packages);
            })
            .resource("/pkgs/schedule/{origin}/{pkg}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).with(Self::schedule_job_group);
            })
            .resource("/pkgs/schedule/{groupid}", |r| {
                r.method(http::Method::GET).with(Self::get_schedule);
            })
            .resource("/pkgs/schedule/{origin}/status", |r| {
                r.method(http::Method::GET)
                    .with(Self::get_origin_schedule_status);
            })
            .resource("/pkgs/{origin}/{pkg}/{version}/{release}/channels", |r| {
                r.middleware(Optional);
                r.get().f(Self::get_package_channels);
            })
    }
}

// TODO: PACKAGES HANLDERS "/depot/pkgs/..."
/*
    r.patch(
        "/depot/pkgs/:origin/:pkg/:version/:release/:visibility",
        XHandler::new(package_privacy_toggle).before(basic.clone()),
        "package_privacy_toggle",
    );
*/

fn packages_path(data_path: &PathBuf) -> PathBuf {
    path::Path::new(data_path).join("pkgs")
}

// Return a formatted string representing the filename of an archive for the given package
// identifier pieces.
fn archive_name(ident: &PackageIdent, target: &PackageTarget) -> PathBuf {
    PathBuf::from(ident.archive_name_with_target(target).expect(&format!(
        "Package ident should be fully qualified, ident={}",
        &ident
    )))
}

fn download_response_for_archive(archive: PackageArchive, file_path: PathBuf) -> HttpResponse {
    let filename = archive.file_name().as_bytes().to_vec();
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
                parameters: vec![DispositionParam::Filename(
                    Charset::Iso_8859_1, // The character set for the bytes of the filename
                    None,                // The optional language tag (see `language-tag` crate)
                    filename,            // the actual bytes of the filename
                )],
            },
        )
        .header(
            http::header::HeaderName::from_static(headers::XFILENAME),
            archive.file_name(),
        )
        .header(http::header::CACHE_CONTROL, headers::cache(true))
        .streaming(rx_body.map_err(|e| error::ErrorBadRequest("bad request")))
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
    ident: &OriginPackageIdent,
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
                warn!(
                    "Failed package circular dependency check: {}, err: {:?}",
                    ident, err
                );
                return Err(Error::CircularDependency(ident.to_string()));
            }
            return Err(Error::NetError(err));
        }
        Err(err) => return Err(err),
    }
}

fn process_upload_for_package_archive(
    ident: &OriginPackageIdent,
    package: &mut OriginPackageCreate,
    owner_id: u64,
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

    // Re-create origin package as needed (eg, checksum update)
    if let Err(err) = conn.route::<OriginPackageCreate, OriginPackage>(&package) {
        debug!("Failed to create origin package, err: {:?}", err);
        return Err(err);
    }

    Ok(())
}

fn is_a_service(package: &OriginPackage) -> bool {
    let m = package.get_manifest();

    // TODO: This is a temporary workaround until we plumb in a better solution for
    // determining whether a package is a service from the DB instead of needing
    // to crack the archive file to look for a SVC_USER file
    m.contains("pkg_exposes") || m.contains("pkg_binds") || m.contains("pkg_exports")
}
