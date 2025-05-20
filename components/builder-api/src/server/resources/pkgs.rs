// Copyright (c) 2018-2022 Chef Software Inc. and/or applicable contributors
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

use super::reverse_dependencies::{self};
use crate::{bldr_core::metrics::CounterMetric,
            db::models::{channel::{Channel,
                                   ChannelWithPromotion},
                         license_keys::*,
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
            server::{authorize::authorize_session,
                     error::{Error,
                             Result},
                     feat,
                     framework::headers,
                     helpers::{self,
                               fetch_license_expiration,
                               req_state,
                               Pagination,
                               Target},
                     resources::channels::channels_for_package_ident,
                     services::metrics::Counter,
                     AppState}};
use actix_web::{body::BoxBody,
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
use serde::ser::Serialize;
use std::{convert::Infallible,
          fs::{self,
               remove_file,
               File},
          io::{BufReader,
               BufWriter,
               Read,
               Write},
          path::{self,
                 PathBuf},
          str::FromStr};
use tempfile::tempdir_in;
use uuid::Uuid;

// Query param containers
#[derive(Debug, Deserialize)]
pub struct Upload {
    #[serde(default)]
    target:   Option<String>,
    #[serde(default)]
    checksum: String,
    #[serde(default)]
    forced:   bool,
}

pub struct Packages {}

impl Packages {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/depot/pkgs/{origin}",
                  web::get().to(get_packages_for_origin))
           .route("/depot/pkgs/search/{query}", web::get().to(search_packages))
           .route("/depot/pkgs/{origin}/{pkg}",
                  web::get().to(get_packages_for_origin_package))
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
async fn get_packages_for_origin(req: HttpRequest,
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
async fn get_packages_for_origin_package(req: HttpRequest,
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
async fn get_packages_for_origin_package_version(req: HttpRequest,
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
async fn get_latest_package_for_origin_package(req: HttpRequest,
                                               path: Path<(String, String)>,
                                               qtarget: Query<Target>)
                                               -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_package(&req, &qtarget, &ident).await {
        Ok(json_body) => {
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_latest_package_for_origin_package_version(req: HttpRequest,
                                                       path: Path<(String, String, String)>,
                                                       qtarget: Query<Target>)
                                                       -> HttpResponse {
    let (origin, pkg, version) = path.into_inner();

    let ident = PackageIdent::new(origin, pkg, Some(version), None);

    match do_get_package(&req, &qtarget, &ident).await {
        Ok(json_body) => {
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_package(req: HttpRequest,
                     path: Path<(String, String, String, String)>,
                     qtarget: Query<Target>)
                     -> HttpResponse {
    let (origin, pkg, version, release) = path.into_inner();

    let ident = PackageIdent::new(origin, pkg, Some(version), Some(release));

    match do_get_package(&req, &qtarget, &ident).await {
        Ok(json_body) => {
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::default().to_string()))
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

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Member)) {
        return err.into();
    }

    let ident = PackageIdent::new(origin.clone(), pkg.clone(), Some(version), Some(release));

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => {
                    debug!("Invalid target requested: {}, err = {:?}", t, err);
                    let body = Bytes::from(format!("Invalid package target '{}'", t).into_bytes());
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                                   BoxBody::new(body));
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
                                         PackageVisibility::all(),
                                         &conn)
    {
        Ok(channels) => {
            if channels.iter()
                       .any(|c| c.name == ChannelIdent::stable().to_string())
            {
                debug!("Deleting package in stable channel not allowed: {}", ident);
                let body = Bytes::from(format!("Deleting package in stable channel not allowed \
                                                '{}'",
                                               ident).into_bytes());
                return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                               BoxBody::new(body));
            }
        }
        Err(err) => {
            debug!("{}", err);
            return Error::DieselError(err).into();
        }
    }

    match reverse_dependencies::get_rdeps(&conn, &origin, &pkg, &target).await {
        Ok(reverse_depenencies) => {
            if !reverse_depenencies.rdeps.is_empty() {
                let body = Bytes::from(format!("Deleting package with rdeps not allowed '{}'",
                                               ident).into_bytes());
                return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                               BoxBody::new(body));
            }
        }
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    }

    // TODO (SA): Wrap in transaction, or better yet, eliminate need to do
    // channel package deletion
    let pkg = match Package::get(GetPackage { ident:      BuilderPackageIdent(ident.clone()),
                                              visibility: PackageVisibility::all(),
                                              target:     BuilderPackageTarget(target), },
                                 &conn).map_err(Error::DieselError)
    {
        Ok(pkg) => pkg,
        Err(err) => return err.into(),
    };

    if let Err(err) = Channel::delete_channel_package(pkg.id, &conn).map_err(Error::DieselError) {
        debug!("{}", err);
        return err.into();
    }

    match Package::delete(DeletePackage { ident:  BuilderPackageIdent(ident.clone()),
                                          target: BuilderPackageTarget(target), },
                          &conn).map_err(Error::DieselError)
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
                    let body = Bytes::from(format!("Invalid package target '{}'", t).into_bytes());
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                                   BoxBody::new(body));
                }
            }
        }
        None => helpers::target_from_headers(&req),
    };

    if !state.config.api.targets.contains(&target) {
        let body = Bytes::from(format!("Invalid package target '{}'", target).into_bytes());
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
    }

    match Package::get(GetPackage { ident:      BuilderPackageIdent(ident.clone()),
                                    visibility: vis,
                                    target:     BuilderPackageTarget(target), },
                       &conn)
    {
        Ok(package) => {
            let channels = match channels_for_package_ident(&req, &package.ident, target, &conn) {
                Ok(channels) => channels,
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Failed to determine package channels: {}", err));
                }
            };

            let should_restrict = if let Some(chs_vec) = channels.as_ref() {
                let in_unrestricted_channels = state.config
                                                    .api
                                                    .unrestricted_channels
                                                    .iter()
                                                    .any(|c| chs_vec.contains(c));

                if in_unrestricted_channels || state.config.api.restricted_if_present.is_empty() {
                    false
                } else {
                    let in_partially_unrestricted_channels = state.config
                                                                  .api
                                                                  .partially_unrestricted_channels
                                                                  .iter()
                                                                  .any(|c| chs_vec.contains(c));

                    let in_restricted_if_present = state.config
                                                        .api
                                                        .restricted_if_present
                                                        .iter()
                                                        .any(|c| chs_vec.contains(c));

                    !in_partially_unrestricted_channels || in_restricted_if_present
                }
            } else {
                true
            };

            if should_restrict {
                match opt_session_id {
                    Some(account_id) => {
                        match LicenseKey::get_by_account_id(account_id as i64, &conn) {
                            Ok(Some(license)) => {
                                let today = chrono::Utc::now().date_naive();
                                if license.expiration_date < today {
                                    match fetch_license_expiration(&license.license_key,
                                                                   &state.config
                                                                         .api
                                                                         .license_server_url)
                                    {
                                        Ok(new_expiration) => {
                                            let update =
                                                NewLicenseKey { account_id:      account_id as i64,
                                                                license_key:
                                                                    &license.license_key,
                                                                expiration_date: new_expiration, };

                                            if let Err(err) = LicenseKey::create(&update, &conn) {
                                                debug!("Failed to update license in DB: {}", err);
                                                return HttpResponse::InternalServerError()
                                                    .body("License update failed.");
                                            }
                                        }
                                        Err(err_msg) => {
                                            return err_msg;
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                return HttpResponse::Forbidden().body("No valid license key \
                                                                       found.");
                            }
                            Err(err) => {
                                debug!("License DB error: {}", err);
                                return HttpResponse::InternalServerError()
                                    .body("License validation error.");
                            }
                        }
                    }
                    None => {
                        return HttpResponse::Unauthorized().body("Authentication required.");
                    }
                }
            }

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
        let body = Bytes::from(format!("Invalid or not fully qualified package identifier '{}'",
                                       ident).into_bytes());
        return Ok(HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body)));
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

#[allow(clippy::needless_pass_by_value)]
async fn get_package_channels(req: HttpRequest,
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
        let body = Bytes::from(
            format!("Required fully qualified package identifier '{}'", ident).into_bytes(),
        );
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
    }

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => {
                    debug!("Invalid target requested: {}, err = {:?}", t, err);
                    let body = Bytes::from(format!("Invalid package target '{}'", t).into_bytes());
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                                   BoxBody::new(body));
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
                                         &conn)
    {
        Ok(channels) => {
            let list: Vec<ChannelWithPromotion> =
                channels.into_iter().map(|channel| channel.into()).collect();
            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                              .json(list)
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn list_package_versions(req: HttpRequest,
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
                                         &conn)
    {
        Ok(packages) => {
            trace!(target: "habitat_builder_api::server::resources::pkgs::versions", "list_package_versions for {} found {} package versions: {:?}", ident, packages.len(), packages);

            let body = serde_json::to_string(&packages).unwrap();
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                              .body(body)
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn search_packages(req: HttpRequest,
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
        // TODO There might be a case where there is a package with the same name as the origin.
        // And the search with the query 'origin/' could end up finding the matches in 'origin' and
        // the package names. Ideally, it should filter the matches to match the 'origin'
        // only in this case.
        Ok(q) => q.to_string().trim_end_matches('/').replace('/', " & "),
        Err(err) => {
            debug!("{}", err);
            let body =
                Bytes::from(format!("Unable to parse query string '{}'", query).into_bytes());
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
        }
    };

    debug!("search_packages called with: {}", decoded_query);

    let search_packages = SearchPackages { query:      decoded_query,
                                           page:       page as i64,
                                           limit:      per_page as i64,
                                           account_id: opt_session_id, };

    if pagination.distinct {
        return match Package::search_distinct(&search_packages, &conn) {
            Ok((packages, count)) => postprocess_package_list(&req, &packages, count, &pagination),
            Err(err) => {
                debug!("{}", err);
                Error::DieselError(err).into()
            }
        };
    }

    match Package::search(&search_packages, &conn) {
        Ok((packages, count)) => postprocess_package_list(&req, &packages, count, &pagination),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn package_privacy_toggle(req: HttpRequest,
                                path: Path<(String, String, String, String, String)>,
                                state: Data<AppState>)
                                -> HttpResponse {
    let (origin, name, version, release, visibility) = path.into_inner();

    let ident = PackageIdent::new(origin.clone(), name, Some(version), Some(release));

    if !ident.valid() {
        debug!("Invalid package identifier: {}", ident);
        let body = Bytes::from(format!("Invalid package identifier '{}'", ident).into_bytes());
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
    }

    let pv: PackageVisibility = match visibility.parse() {
        Ok(o) => o,
        Err(err) => {
            debug!("{:?}", err);
            let body =
                Bytes::from(format!("Invalid package visibility '{}'", visibility).into_bytes());
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
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
        let body = Bytes::from("Not allowed to set packages to 'hidden'");
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
    }

    match Package::update_visibility(pv, BuilderPackageIdent(ident.clone()), &conn) {
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

    let body = helpers::package_results_json(packages, count as isize, start, stop as isize);

    let mut response = if count as isize > (stop as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response.append_header((http::header::CONTENT_TYPE, headers::APPLICATION_JSON))
            .append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
            .body(body)
}

pub fn postprocess_extended_package_list(_req: &HttpRequest,
                                         packages: &[PackageIdentWithChannelPlatform],
                                         count: i64,
                                         pagination: &Query<Pagination>)
                                         -> HttpResponse {
    let start = if pagination.range < 0 {
        0
    } else {
        let (start, _) = helpers::extract_pagination(pagination);
        start
    };
    let pkg_count = packages.len() as isize;
    let stop = match pkg_count {
        0 => count,
        _ => (start + pkg_count - 1) as i64,
    };

    debug!("postprocessing extended package list, start: {}, stop: {}, total_count: {}",
           start, stop, count);

    let body = helpers::package_results_json(packages, count as isize, start, stop as isize);

    let mut response = if count as isize > (stop as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response.append_header((http::header::CONTENT_TYPE, headers::APPLICATION_JSON))
            .append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
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
    let limit = if pagination.range < 0 { -1 } else { per_page };

    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    let lpr = ListPackages { ident:      BuilderPackageIdent(ident.clone()),
                             visibility: helpers::visibility_for_optional_session(req,
                                                                                  opt_session_id,
                                                                                  &ident.origin),
                             page:       page as i64,
                             limit:      limit as i64, };

    if pagination.distinct {
        match Package::list_distinct(lpr, &conn).map_err(Error::DieselError) {
            Ok((packages, count)) => {
                let ident_pkgs: Vec<PackageIdentWithChannelPlatform> =
                    packages.into_iter().map(|p| p.into()).collect();
                return Ok((ident_pkgs, count));
            }
            Err(e) => return Err(e),
        }
    }

    match Package::list(lpr, &conn).map_err(Error::DieselError) {
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
    authorize_session(req, Some(&ident.origin), Some(OriginMemberRole::Member))?;

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
                visibility: PackageVisibility::all(),
                target: BuilderPackageTarget(PackageTarget::from_str(&target).unwrap()), // Unwrap OK
            },
            &conn,
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
                                  temp_path: &path::Path)
                                  -> HttpResponse {
    let mut archive = match PackageArchive::new(temp_path) {
        Ok(archive) => archive,
        Err(e) => {
            info!("Could not read the package at {:#?}: {:#?}", temp_path, e);
            let body = Bytes::from(format!("ds:up:0, err={:?}", e).into_bytes());
            let body = BoxBody::new(body);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
        }
    };

    debug!("Package Archive: {:#?}", archive);

    let package_type = match archive.package_type() {
        Ok(pkg_type) => pkg_type,
        Err(e) => {
            info!("Could not read the package type for {:#?}: {:#?}",
                  archive, e);
            let body = Bytes::from(format!("ds:up:0, err={:?}", e).into_bytes());
            let body = BoxBody::new(body);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
        }
    };

    let target_from_artifact = match archive.target() {
        Ok(target) => target,
        Err(e) => {
            info!("Could not read the target for {:#?}: {:#?}", archive, e);
            let body = Bytes::from(format!("ds:up:1, err={:?}", e).into_bytes());
            let body = BoxBody::new(body);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
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
            let body = Bytes::from_static(b"ds:up:2");
            let body = BoxBody::new(body);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
        }
    };

    if qupload.checksum != checksum_from_artifact {
        debug!("Checksums did not match: from_param={:?}, from_artifact={:?}",
               qupload.checksum, checksum_from_artifact);
        let body = Bytes::from_static(b"ds:up:3");
        let body = BoxBody::new(body);
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
    }

    let conn = match req_state(req).db.get_conn().map_err(Error::DbError) {
        Ok(conn) => conn,
        Err(err) => return err.into(),
    };

    // Check If previously uploaded package exists in DB
    // and discard the upload if package_type mismatch occurs.
    let pkg_ident = PackageIdent::new(ident.origin.clone(), ident.name.clone(), None, None);
    match Package::get_latest(
        GetLatestPackage {
            ident: BuilderPackageIdent(pkg_ident),
            target: BuilderPackageTarget(PackageTarget::from_str(&target_from_artifact).unwrap()),
            visibility: PackageVisibility::all(),
        },
        &conn,
    ) {
        Ok(pkg) => {
            if package_type != *pkg.package_type {
                debug!(
                    "Package Type did not match: from_param={:?}, from_database={:?}",
                    package_type, pkg.package_type
                );
                let body = Bytes::from(
                    format!(
                        "Package type mismatch; expected '{}', found '{}'",
                        *pkg.package_type, package_type
                    )
                    .into_bytes(),
                );
                return HttpResponse::with_body(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    BoxBody::new(body),
                );
            }
        }
        Err(NotFound) => {
            debug!("Package does not already exist in the Database.");
        }
        Err(err) => return Error::DieselError(err).into(),
    }

    // If upload was forced, and a previously uploaded package exists in DB
    // make sure the checksums match the original (idempotency)
    if qupload.forced {
        match Package::get(
            GetPackage {
                ident: BuilderPackageIdent(ident.clone()),
                visibility: PackageVisibility::all(),
                target: BuilderPackageTarget(
                    PackageTarget::from_str(&target_from_artifact).unwrap(),
                ), // Unwrap OK
            },
            &conn,
        ) {
            Ok(pkg) => {
                if qupload.checksum != pkg.checksum {
                    debug!(
                        "Checksums did not match: from_param={:?}, from_database={:?}",
                        qupload.checksum, pkg.checksum
                    );
                    let body = Bytes::from_static(b"ds:up:4");
                    let body = BoxBody::new(body);
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
                }
            }
            Err(NotFound) => {}
            Err(err) => return Error::DieselError(err).into(),
        }
    }

    let file_path = &req_state(req).config.api.data_path;
    let filename = file_path.join(archive_name(ident, target_from_artifact));
    let temp_ident = ident.to_owned();

    match fs::rename(temp_path, &filename) {
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

    let mut archive = match PackageArchive::new(filename.clone()) {
        Ok(archive) => archive,
        Err(e) => {
            debug!("Could not read the package at {:#?}: {:#?}", filename, e);
            return Error::HabitatCore(e).into();
        }
    };

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
        let body = Bytes::from_static(b"ds:up:6");
        let body = BoxBody::new(body);
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
    }

    let session = authorize_session(req, None, None).unwrap(); // Unwrap Ok

    package.owner_id = session.get_id() as i64;
    package.origin = ident.clone().origin;

    package.visibility = match OriginPackageSettings::get(
        &GetOriginPackageSettings {
            origin: &package.origin,
            name: &package.name,
        },
        &conn,
    ) {
        // TED if this is in-fact optional in the db it should be an option in the model
        Ok(pkg) => pkg.visibility,
        Err(_) => match Origin::get(&ident.origin, &conn) {
            Ok(o) => {
                match OriginPackageSettings::create(
                    &NewOriginPackageSettings {
                        origin: &ident.origin,
                        name: &ident.name,
                        visibility: &o.default_package_visibility,
                        owner_id: package.owner_id,
                    },
                    &conn,
                ) {
                    Ok(pkg_settings) => pkg_settings.visibility,
                    Err(err) => return Error::DieselError(err).into(),
                }
            }
            Err(err) => return Error::DieselError(err).into(),
        },
    };

    // Re-create origin package as needed (eg, checksum update)
    match Package::create(&package, &conn) {
        Ok(_) => {}
        Err(NotFound) => {
            debug!("Package::create returned NotFound (DB conflict handled)");
        }
        Err(err) => {
            debug!("Failed to create package in DB, err: {:?}", err);
            return Error::DieselError(err).into();
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

    HttpResponse::Created().append_header((http::header::LOCATION, format!("{}", req.uri())))
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
        writer = web::block(move || writer.write(&chunk).map(|_| writer)).await??;
    }

    match writer.into_inner() {
        Ok(f) => {
            f.sync_all()?;
            Ok(do_upload_package_finish(&req, &qupload, &ident, &temp_path).await)
        }
        Err(err) => Err(Error::InnerError(err)),
    }
}

async fn do_get_package(req: &HttpRequest,
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
            PackageTarget::from_str(t)?
        }
        None => helpers::target_from_headers(req),
    };

    // Scope this memcache usage so the reference goes out of
    // scope before the visibility_for_optional_session call
    // below
    {
        let mut memcache = req_state(req).memcache.borrow_mut();
        match memcache.get_package(ident, &ChannelIdent::unstable(), &target, opt_session_id) {
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
                                          &conn)
        {
            Ok(pkg) => pkg,
            Err(NotFound) => {
                let mut memcache = req_state(req).memcache.borrow_mut();
                memcache.set_package(ident,
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
            &conn,
        ) {
            Ok(pkg) => pkg.into(),
            Err(NotFound) => {
                let mut memcache = req_state(req).memcache.borrow_mut();
                memcache.set_package(
                    ident,
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
    let channels = channels_for_package_ident(req, &pkg.ident, *pkg.target, &conn)?;

    pkg_json["manifest"] = json!("");
    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(pkg.is_a_service());
    if let Some(obj) = pkg_json.as_object_mut() {
        obj.insert("deps".to_string(), json!([]));
        obj.insert("tdeps".to_string(), json!([]));
        obj.insert("build_deps".to_string(), json!([]));
        obj.insert("build_tdeps".to_string(), json!([]));
    }
    let size = match req_state(req).packages
                                   .size_of(&pkg.ident, *pkg.target)
                                   .await
    {
        Ok(size) => size,
        Err(err) => {
            debug!("Could not get size for {:?}, {:?}", pkg.ident, err);
            0
        }
    };

    pkg_json["size"] = json!(size);

    let json_body = serde_json::to_string(&pkg_json).unwrap();

    {
        let mut memcache = req_state(req).memcache.borrow_mut();
        memcache.set_package(ident,
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
                                 file_path: &path::Path,
                                 is_private: bool,
                                 state: &Data<AppState>)
                                 -> HttpResponse {
    let filename = archive.file_name();
    let file = match File::open(file_path) {
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
    HttpResponse::Ok()
        .append_header((
            http::header::CONTENT_DISPOSITION,
            ContentDisposition {
                disposition: DispositionType::Attachment,
                parameters: vec![DispositionParam::Filename(filename)],
            },
        ))
        .append_header((
            http::header::HeaderName::from_static(headers::XFILENAME),
            archive.file_name(),
        ))
        .insert_header(ContentType::octet_stream())
        .append_header((http::header::CACHE_CONTROL, cache_hdr))
        .streaming(rx_body.map(|s| Ok::<_, Infallible>(s)))
}
