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

use actix_web::{http::{self,
                       StatusCode},
                web::{self,
                      Data,
                      Path,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};
use diesel::{pg::PgConnection,
             result::{DatabaseErrorKind,
                      Error::{DatabaseError,
                              NotFound}}};

use crate::{bldr_core::metrics::CounterMetric,
            bldr_events::event::{AffinityKey::*,
                                 BuilderEvent,
                                 EventType},
            hab_core::{package::{PackageIdent,
                                 PackageTarget},
                       ChannelIdent}};

use crate::db::models::{channel::*,
                        origin::*,
                        package::{BuilderPackageIdent,
                                  GetPackageGroup,
                                  Package,
                                  PackageVisibility}};

use crate::server::{authorize::authorize_session,
                    error::{Error,
                            Result},
                    framework::headers,
                    helpers::{self,
                              req_state,
                              visibility_for_optional_session,
                              Pagination,
                              Target,
                              ToChannel},
                    services::metrics::Counter,
                    AppState};

// Query param containers
#[derive(Debug, Default, Clone, Deserialize)]
struct SandboxBool {
    #[serde(default)]
    sandbox: bool,
}

pub struct Channels;

impl Channels {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/depot/channels/{origin}", web::get().to(get_channels))
           .route("/depot/channels/{origin}/{channel}",
                  web::post().to(create_channel))
           .route("/depot/channels/{origin}/{channel}",
                  web::delete().to(delete_channel))
           .route("/depot/channels/{origin}/{channel}/pkgs",
                  web::get().to(get_packages_for_origin_channel))
           .route("/depot/channels/{origin}/{channel}/pkgs/_latest",
                  web::get().to(get_latest_packages_for_origin_channel))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}",
                  web::get().to(get_packages_for_origin_channel_package))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/latest",
                  web::get().to(get_latest_package_for_origin_channel_package))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}",
                  web::get().to(get_packages_for_origin_channel_package_version))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/latest",
                  web::get().to(get_latest_package_for_origin_channel_package_version))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}",
                  web::get().to(get_package_fully_qualified))
           .route("/depot/channels/{origin}/{channel}/pkgs/promote",
                  web::put().to(promote_channel_packages))
           .route("/depot/channels/{origin}/{channel}/pkgs/demote",
                  web::put().to(demote_channel_packages))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}/promote",
                  web::put().to(promote_package))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}/demote",
                  web::put().to(demote_package));
    }
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
fn get_channels(path: Path<String>,
                sandbox: Query<SandboxBool>,
                state: Data<AppState>)
                -> HttpResponse {
    let origin = path.into_inner();

    let conn = match state.db.get_conn().map_err(Error::DbError) {
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
            let ident_list: Vec<Temp> = list.iter()
                                            .map(|channel| Temp { name: channel.name.clone(), })
                                            .collect();
            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .json(ident_list)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get channels, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_channel(req: HttpRequest,
                  path: Path<(String, String)>,
                  state: Data<AppState>)
                  -> HttpResponse {
    let (origin, channel) = path.into_inner();

    let session_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.get_id(),
            Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
        };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Channel::create(&CreateChannel { name:     &channel,
                                           origin:   &origin,
                                           owner_id: session_id as i64, },
                          &*conn)
    {
        Ok(channel) => HttpResponse::Created().json(channel),
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            HttpResponse::Conflict().into()
        }
        Err(err) => {
            debug!("Failed to create channel, err={}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_channel(req: HttpRequest,
                  path: Path<(String, String)>,
                  state: Data<AppState>)
                  -> HttpResponse {
    let (origin, channel) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    if let Err(_err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return HttpResponse::new(StatusCode::UNAUTHORIZED);
    }

    if channel == ChannelIdent::stable() || channel == ChannelIdent::unstable() {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    state.memcache
         .borrow_mut()
         .clear_cache_for_channel(&origin, &channel);

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Channel::delete(&origin, &channel, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => {
            debug!("Failed to delete channel, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn promote_channel_packages(req: HttpRequest,
                            path: Path<(String, String)>,
                            state: Data<AppState>,
                            to_channel: Query<ToChannel>)
                            -> HttpResponse {
    let (origin, channel) = path.into_inner();

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let ch_source = ChannelIdent::from(channel);
    let ch_target = ChannelIdent::from(to_channel.channel.as_ref());

    match do_promote_or_demote_channel_packages(&req,
                                                &ch_source,
                                                &ch_target,
                                                &origin,
                                                true,
                                                session.get_id() as i64)
    {
        Ok(pkg_ids) => {
            match PackageGroupChannelAudit::audit(
                PackageGroupChannelAudit {
                    origin: &origin,
                    channel: ch_target.as_str(),
                    package_ids: pkg_ids,
                    operation: PackageChannelOperation::Promote,
                    trigger: helpers::trigger_from_request_model(&req),
                    requester_id: session.get_id() as i64,
                    requester_name: session.get_name(),
                    group_id: 0_i64,
                },
                &*conn,
            ) {
                Ok(_) => {}
                Err(e) => debug!("Failed to save rank change to audit log: {}", e),
            };
            HttpResponse::new(StatusCode::OK)
        }
        Err(e) => {
            debug!("Failed to promote channel packages, err={}", e);
            e.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn demote_channel_packages(req: HttpRequest,
                           path: Path<(String, String)>,
                           state: Data<AppState>,
                           to_channel: Query<ToChannel>)
                           -> HttpResponse {
    let (origin, channel) = path.into_inner();
    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let ch_source = ChannelIdent::from(channel);
    let ch_target = ChannelIdent::from(to_channel.channel.as_ref());

    match do_promote_or_demote_channel_packages(&req,
                                                &ch_source,
                                                &ch_target,
                                                &origin,
                                                false,
                                                session.get_id() as i64)
    {
        Ok(pkg_ids) => {
            match PackageGroupChannelAudit::audit(
                PackageGroupChannelAudit {
                    origin: &origin,
                    channel: ch_target.as_str(),
                    package_ids: pkg_ids,
                    operation: PackageChannelOperation::Demote,
                    trigger: helpers::trigger_from_request_model(&req),
                    requester_id: session.get_id() as i64,
                    requester_name: session.get_name(),
                    group_id: 0_i64,
                },
                &*conn,
            ) {
                Ok(_) => {}
                Err(e) => debug!("Failed to save rank change to audit log: {}", e),
            };
            HttpResponse::new(StatusCode::OK)
        }
        Err(e) => {
            debug!("Failed to demote channel packages, err={}", e);
            e.into()
        }
    }
}

fn do_promote_or_demote_channel_packages(req: &HttpRequest,
                                         ch_source: &ChannelIdent,
                                         ch_target: &ChannelIdent,
                                         origin: &str,
                                         promote: bool,
                                         session_id: i64)
                                         -> Result<Vec<i64>> {
    Counter::AtomicChannelRequests.increment();
    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;
    let mut pkg_ids = Vec::new();

    // Simple guards to protect users from bad decisioning
    if !promote
       && (*ch_target == ChannelIdent::unstable() || *ch_source == ChannelIdent::unstable())
    {
        return Err(Error::BadRequest);
    }

    if *ch_target == *ch_source {
        return Err(Error::BadRequest);
    }

    if promote && *ch_target == ChannelIdent::unstable() {
        return Err(Error::BadRequest);
    }

    let pkgs = do_get_all_channel_packages(req, origin, ch_source)?;

    #[rustfmt::skip]
    let channel = match Channel::get(origin, ch_target, &*conn) {
        Ok(channel) => channel,
        Err(NotFound) => {
            if (ch_target != &ChannelIdent::stable()) && (ch_target != &ChannelIdent::unstable()) {
                Channel::create(
                    &CreateChannel {
                        name:     ch_target.as_str(),
                        origin,
                        owner_id: session_id,
                    },
                &*conn)?
            } else {
                warn!("Unable to retrieve target channel: {}", &ch_target);
                return Err(Error::DieselError(NotFound));
            }
        }
        Err(e) => {
            info!("Unable to retrieve channel, err: {:?}", e);
            return Err(Error::DieselError(e));
        }
    };

    #[rustfmt::skip]
    let op = Package::get_group(
        GetPackageGroup {
            pkgs,
            visibility: PackageVisibility::all()
        },
    &*conn)?;

    let mut ids: Vec<i64> = op.iter().map(|x| x.id).collect();

    pkg_ids.append(&mut ids);

    if promote {
        debug!("Bulk promoting Pkg IDs: {:?}", &pkg_ids);
        Channel::promote_packages(channel.id, &pkg_ids, &*conn)?;
    } else {
        debug!("Bulk demoting Pkg IDs: {:?}", &pkg_ids);
        Channel::demote_packages(channel.id, &pkg_ids, &*conn)?;
    }
    Ok(pkg_ids)
}

#[allow(clippy::needless_pass_by_value)]
async fn promote_package(req: HttpRequest,
                         path: Path<(String, String, String, String, String)>,
                         qtarget: Query<Target>,
                         state: Data<AppState>)
                         -> HttpResponse {
    let (origin, channel, pkg, version, release) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let ident = PackageIdent::new(origin.clone(), pkg, Some(version), Some(release));

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

    let auditevent = PackageChannelAudit { package_ident:  BuilderPackageIdent(ident.clone()),
                                           channel:        channel.as_str(),
                                           operation:      PackageChannelOperation::Promote,
                                           trigger:
                                               helpers::trigger_from_request_model(&req),
                                           requester_id:   session.get_id() as i64,
                                           requester_name: session.get_name(),
                                           origin:         &origin, };

    match OriginChannelPackage::promote(
        OriginChannelPromote {
            ident: BuilderPackageIdent(ident.clone()),
            target,
            origin: origin.clone(),
            channel: channel.clone(),
        },
        &*conn
    )
    .map_err(Error::DieselError)
    {
        Ok(promoted_count) => {
            // Note: promoted_count is 0 when attempting to promote a package to a channel where it already exists
            if promoted_count != 0 {
                if let Err(e) = PackageChannelAudit::audit(&auditevent, &*conn) {
                     debug!("Failed to save rank change to audit log: {}", e);
                };

                BuilderEvent::new(EventType::PackageChannelMotion, NoAffinity, "builder_events".to_string(), &auditevent)
                    .publish(&state.event_producer).await;
            }

            state
                .memcache
                .borrow_mut()
                .clear_cache_for_package(&ident);
            HttpResponse::new(StatusCode::OK)
        }
        Err(err) => {
            debug!("Failed to promote package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn demote_package(req: HttpRequest,
                  path: Path<(String, String, String, String, String)>,
                  qtarget: Query<Target>,
                  state: Data<AppState>)
                  -> HttpResponse {
    let (origin, channel, pkg, version, release) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    if channel == ChannelIdent::unstable() {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let ident = PackageIdent::new(origin.clone(), pkg, Some(version), Some(release));

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

    match OriginChannelPackage::demote(OriginChannelDemote { ident:
                                                                 BuilderPackageIdent(ident.clone()),
                                                             target,
                                                             origin: origin.clone(),
                                                             channel: channel.clone() },
                                       &*conn).map_err(Error::DieselError)
    {
        Ok(0) => {
            debug!("Requested package {} for target {} not present in channel {}",
                   ident, target, channel);
            HttpResponse::new(StatusCode::BAD_REQUEST)
        }
        Ok(_) => {
            match PackageChannelAudit::audit(
                &PackageChannelAudit {
                    package_ident: BuilderPackageIdent(ident.clone()),
                    channel: channel.as_str(),
                    operation: PackageChannelOperation::Demote,
                    trigger: helpers::trigger_from_request_model(&req),
                    requester_id: session.get_id() as i64,
                    requester_name: session.get_name(),
                    origin: &origin,
                },
                &*conn,
            ) {
                Ok(_) => {}
                Err(err) => debug!("Failed to save rank change to audit log: {}", err),
            };
            state.memcache.borrow_mut().clear_cache_for_package(&ident);
            HttpResponse::new(StatusCode::OK)
        }
        Err(err) => {
            debug!("Failed to demote package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_packages_for_origin_channel_package_version(req: HttpRequest,
                                                   path: Path<(String, String, String, String)>,
                                                   pagination: Query<Pagination>)
                                                   -> HttpResponse {
    let (origin, channel, pkg, version) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, Some(version), None);

    match do_get_channel_packages(&req, &pagination, &ident, &channel) {
        Ok((packages, count)) => {
            postprocess_channel_package_list(&req, &packages, count, &pagination)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get packages, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_packages_for_origin_channel_package(req: HttpRequest,
                                           path: Path<(String, String, String)>,
                                           pagination: Query<Pagination>)
                                           -> HttpResponse {
    let (origin, channel, pkg) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_channel_packages(&req, &pagination, &ident, &channel) {
        Ok((packages, count)) => {
            postprocess_channel_package_list(&req, &packages, count, &pagination)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get packages, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_packages_for_origin_channel(req: HttpRequest,
                                   path: Path<(String, String)>,
                                   pagination: Query<Pagination>)
                                   -> HttpResponse {
    let (origin, channel) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    // It feels 1000x wrong to set the package name to ""
    let ident = PackageIdent::new(origin, String::from(""), None, None);

    match do_get_channel_packages(&req, &pagination, &ident, &channel) {
        Ok((packages, count)) => {
            postprocess_channel_package_list(&req, &packages, count, &pagination)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get packages, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_latest_package_for_origin_channel_package(req: HttpRequest,
                                                 path: Path<(String, String, String)>,
                                                 qtarget: Query<Target>)
                                                 -> HttpResponse {
    let (origin, channel, pkg) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_channel_package(&req, &qtarget, &ident, &channel) {
        Ok(json_body) => {
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get latest package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_latest_package_for_origin_channel_package_version(req: HttpRequest,
                                                         path: Path<(String,
                                                               String,
                                                               String,
                                                               String)>,
                                                         qtarget: Query<Target>)
                                                         -> HttpResponse {
    let (origin, channel, pkg, version) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, Some(version), None);

    match do_get_channel_package(&req, &qtarget, &ident, &channel) {
        Ok(json_body) => {
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get latest package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_package_fully_qualified(req: HttpRequest,
                               path: Path<(String, String, String, String, String)>,
                               qtarget: Query<Target>)
                               -> HttpResponse {
    let (origin, channel, pkg, version, release) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, Some(version), Some(release));

    match do_get_channel_package(&req, &qtarget, &ident, &channel) {
        Ok(json_body) => {
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_latest_packages_for_origin_channel(req: HttpRequest,
                                          path: Path<(String, String)>,
                                          qtarget: Query<Target>)
                                          -> HttpResponse {
    let (origin, channel) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    match do_get_latest_channel_packages(&req, &qtarget, &origin, &channel) {
        Ok((channel, target, data)) => {
            let json_body = helpers::channel_listing_results_json(&channel, &target, &data);
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(Error::BadRequest) => HttpResponse::new(StatusCode::BAD_REQUEST),
        Err(err) => {
            debug!("Failed to get package, err={}", err);
            err.into()
        }
    }
}

// Internal - these functions should return Result<..>
//

fn do_get_latest_channel_packages(req: &HttpRequest,
                                  qtarget: &Query<Target>,
                                  origin: &str,
                                  channel: &ChannelIdent)
                                  -> Result<(String, String, Vec<BuilderPackageIdent>)> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    // This is a new API, so we only look at the query string not the headers.
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            t
        }
        None => return Err(Error::BadRequest),
    };

    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    Channel::list_latest_packages(
        &ListAllChannelPackagesForTarget {
            visibility: &helpers::visibility_for_optional_session(req, opt_session_id, origin),
            channel,
            origin,
            target,
        },
        &*conn,
    )
    .map_err(Error::DieselError)
}

fn do_get_channel_packages(req: &HttpRequest,
                           pagination: &Query<Pagination>,
                           ident: &PackageIdent,
                           channel: &ChannelIdent)
                           -> Result<(Vec<BuilderPackageIdent>, i64)> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };
    let (page, per_page) = helpers::extract_pagination_in_pages(pagination);

    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    Channel::list_packages(
        &ListChannelPackages {
            ident: &BuilderPackageIdent(ident.clone()),
            visibility: &helpers::visibility_for_optional_session(
                req,
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

fn do_get_all_channel_packages(req: &HttpRequest,
                               origin: &str,
                               channel: &ChannelIdent)
                               -> Result<Vec<BuilderPackageIdent>> {
    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    Channel::list_all_packages(&ListAllChannelPackages { visibility: &PackageVisibility::all(),
                                                         origin,
                                                         channel },
                               &*conn).map_err(Error::DieselError)
}

fn do_get_channel_package(req: &HttpRequest,
                          qtarget: &Query<Target>,
                          ident: &PackageIdent,
                          channel: &ChannelIdent)
                          -> Result<String> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };
    Counter::GetChannelPackage.increment();

    let req_ident = ident.clone();

    // TODO: Deprecate target from headers
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
        match memcache.get_package(&req_ident, channel, &target, opt_session_id) {
            (true, Some(pkg_json)) => {
                trace!("Channel package {} {} {} {:?} - cache hit with pkg json",
                       channel,
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
                Counter::MemcacheChannelPackageHit.increment();
                return Ok(pkg_json);
            }
            (true, None) => {
                trace!("Channel package {} {} {} {:?} - cache hit with 404",
                       channel,
                       ident,
                       target,
                       opt_session_id);
                Counter::MemcacheChannelPackage404.increment();
                return Err(Error::NotFound);
            }
            (false, _) => {
                trace!("Channel package {} {} {} {:?} - cache miss",
                       channel,
                       ident,
                       target,
                       opt_session_id);
                Counter::MemcacheChannelPackageMiss.increment();
            }
        };
    }

    let conn = match req_state(req).db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(e) => return Err(e.into()),
    };

    let pkg: Package = match Channel::get_latest_package(
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
        Ok(pkg) => pkg.into(),
        Err(NotFound) => {
            let mut memcache = req_state(req).memcache.borrow_mut();
            memcache.set_package(&req_ident, None, channel, &target, opt_session_id);
            return Err(Error::NotFound);
        }
        Err(err) => return Err(err.into()),
    };

    let mut pkg_json = serde_json::to_value(pkg.clone()).unwrap();
    let channels = channels_for_package_ident(req, &pkg.ident, target, &*conn)?;

    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(pkg.is_a_service());

    let json_body = serde_json::to_string(&pkg_json).unwrap();

    {
        let mut memcache = req_state(req).memcache.borrow_mut();
        memcache.set_package(&req_ident,
                             Some(&json_body),
                             channel,
                             &target,
                             opt_session_id);
    }

    Ok(json_body)
}

pub fn channels_for_package_ident(req: &HttpRequest,
                                  package: &BuilderPackageIdent,
                                  target: PackageTarget,
                                  conn: &PgConnection)
                                  -> Result<Option<Vec<String>>> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    match Package::list_package_channels(package,
                                         target,
                                         visibility_for_optional_session(req,
                                                                         opt_session_id,
                                                                         &package.clone().origin),
                                         &*conn).map_err(Error::DieselError)
    {
        Ok(channels) => {
            let list: Vec<String> = channels.iter()
                                            .map(|channel| channel.name.to_string())
                                            .collect();

            Ok(Some(list))
        }
        Err(err) => Err(err),
    }
}

// Helper

fn postprocess_channel_package_list(_req: &HttpRequest,
                                    packages: &[BuilderPackageIdent],
                                    count: i64,
                                    pagination: &Query<Pagination>)
                                    -> HttpResponse {
    let (start, _) = helpers::extract_pagination(pagination);
    let pkg_count = packages.len() as isize;
    let stop = match pkg_count {
        0 => count,
        _ => (start + pkg_count - 1) as i64,
    };

    debug!("postprocessing channel package list, start: {}, stop: {}, total_count: {}",
           start, stop, count);

    let body =
        helpers::package_results_json(packages, count as isize, start as isize, stop as isize);

    let mut response = if count as isize > (stop as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response.append_header((http::header::CONTENT_TYPE, headers::APPLICATION_JSON))
            .append_header((http::header::CACHE_CONTROL, headers::Cache::NoCache.to_string()))
            .body(body)
}
