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

use std::{collections::HashMap, str::FromStr};

use protobuf::RepeatedField;

use actix_web::{
    http::{self, StatusCode},
    web::{self, Json, Path, Query, ServiceConfig},
    HttpRequest, HttpResponse,
};
use serde_json;

use crate::protocol::{jobsrv, net::NetOk, originsrv::OriginPackageIdent};

use crate::hab_core::{
    package::{Identifiable, PackageIdent, PackageTarget},
    ChannelIdent,
};

use crate::db::models::{channel::*, jobs::*, origin::*, package::*, projects::*, settings::*};
use diesel::result::Error::NotFound;

use crate::server::{
    authorize::authorize_session,
    error::{Error, Result},
    feat,
    framework::{headers, middleware::route_message},
    helpers::{self, req_state, Target},
    resources::{channels::channels_for_package_ident, pkgs::platforms_for_package_ident},
};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GroupPromoteReq {
    #[serde(default)]
    pub idents: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GroupDemoteReq {
    #[serde(default)]
    pub idents: Vec<String>,
}

#[derive(Deserialize)]
pub struct JobLogPagination {
    #[serde(default)]
    start: u64,
    #[serde(default)]
    color: bool,
}

pub struct Jobs;

impl Jobs {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route(
            "/jobs/group/{id}/promote/{channel}",
            web::post().to(promote_job_group),
        )
        .route(
            "/jobs/group/{id}/demote/{channel}",
            web::post().to(demote_job_group),
        )
        .route("/jobs/group/{id}/cancel", web::post().to(cancel_job_group))
        .route("/rdeps/{origin}/{name}", web::get().to(get_rdeps))
        .route(
            "/rdeps/{origin}/{name}/group",
            web::get().to(get_rdeps_group),
        )
        .route("/jobs/{id}", web::get().to(get_job))
        .route("/jobs/{id}/log", web::get().to(get_job_log));
    }
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
async fn get_rdeps(
    req: HttpRequest,
    path: Path<(String, String)>,
    qtarget: Query<Target>,
) -> HttpResponse {
    let (origin, name) = path.into_inner();

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => return Error::HabitatCore(err).into(),
            }
        }
        None => helpers::target_from_headers(&req),
    };

    let mut rdeps_get = jobsrv::JobGraphPackageReverseDependenciesGet::new();
    rdeps_get.set_origin(origin);
    rdeps_get.set_name(name);
    rdeps_get.set_target(target.to_string());

    match route_message::<
        jobsrv::JobGraphPackageReverseDependenciesGet,
        jobsrv::JobGraphPackageReverseDependencies,
    >(&req, &rdeps_get)
    .await
    {
        Ok(rdeps) => {
            let filtered = match filtered_rdeps(&req, &rdeps) {
                Ok(f) => f,
                Err(err) => return err.into(),
            };
            HttpResponse::Ok().json(filtered)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

fn filtered_rdeps(
    req: &HttpRequest,
    rdeps: &jobsrv::JobGraphPackageReverseDependencies,
) -> Result<jobsrv::JobGraphPackageReverseDependencies> {
    let mut new_rdeps = jobsrv::JobGraphPackageReverseDependencies::new();
    new_rdeps.set_origin(rdeps.get_origin().to_string());
    new_rdeps.set_name(rdeps.get_name().to_string());

    let mut origin_map = HashMap::new();
    let mut short_deps = RepeatedField::new();

    for rdep in rdeps.get_rdeps() {
        let ident = OriginPackageIdent::from_str(rdep)?;
        let origin_name = ident.get_origin();
        let pv = if !origin_map.contains_key(origin_name) {
            let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;
            let origin = Origin::get(&origin_name, &*conn)?;
            origin_map.insert(
                origin_name.to_owned(),
                origin.default_package_visibility.clone(),
            );
            origin.default_package_visibility
        } else {
            origin_map[origin_name].clone()
        };
        if pv != PackageVisibility::Public
            && authorize_session(req, Some(&origin_name), Some(OriginMemberRole::Member)).is_err()
        {
            debug!("Skipping unauthorized non-public origin package: {}", rdep);
            continue; // Skip any unauthorized origin packages
        }

        short_deps.push(rdep.to_string())
    }

    new_rdeps.set_rdeps(short_deps);
    Ok(new_rdeps)
}

#[allow(clippy::needless_pass_by_value)]
async fn get_rdeps_group(
    req: HttpRequest,
    path: Path<(String, String)>,
    qtarget: Query<Target>,
) -> HttpResponse {
    let (origin, name) = path.into_inner();

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => return Error::HabitatCore(err).into(),
            }
        }
        None => helpers::target_from_headers(&req),
    };

    let mut rdeps_get = jobsrv::JobGraphPackageReverseDependenciesGroupedGet::new();
    rdeps_get.set_origin(origin);
    rdeps_get.set_name(name);
    rdeps_get.set_target(target.to_string());

    match route_message::<
        jobsrv::JobGraphPackageReverseDependenciesGroupedGet,
        jobsrv::JobGraphPackageReverseDependenciesGrouped,
    >(&req, &rdeps_get)
    .await
    {
        Ok(rdeps) => {
            let filtered = match filtered_group_rdeps(&req, &rdeps) {
                Ok(f) => f,
                Err(err) => return err.into(),
            };
            HttpResponse::Ok().json(filtered)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

fn filtered_group_rdeps(
    req: &HttpRequest,
    rdeps: &jobsrv::JobGraphPackageReverseDependenciesGrouped,
) -> Result<jobsrv::JobGraphPackageReverseDependenciesGrouped> {
    let mut new_rdeps = jobsrv::JobGraphPackageReverseDependenciesGrouped::new();
    new_rdeps.set_origin(rdeps.get_origin().to_string());
    new_rdeps.set_name(rdeps.get_name().to_string());

    let mut origin_map = HashMap::new();
    let mut new_groups = RepeatedField::new();

    for group in rdeps.get_rdeps() {
        let mut ident_list = Vec::new();
        for ident_str in group.get_idents() {
            let ident = OriginPackageIdent::from_str(ident_str)?;
            let origin_name = ident.get_origin();
            let pv = if !origin_map.contains_key(origin_name) {
                let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;
                let origin = Origin::get(&origin_name, &*conn)?;
                origin_map.insert(
                    origin_name.to_owned(),
                    origin.default_package_visibility.clone(),
                );
                origin.default_package_visibility
            } else {
                origin_map[origin_name].clone()
            };
            if pv != PackageVisibility::Public
                && authorize_session(req, Some(&origin_name), None).is_err()
            {
                debug!(
                    "Skipping unauthorized non-public origin package: {}",
                    ident_str
                );
                continue; // Skip any unauthorized origin packages
            }
            ident_list.push(ident_str.to_owned())
        }

        let mut new_group = jobsrv::JobGraphPackageReverseDependencyGroup::new();
        new_group.set_group_id(group.get_group_id());
        new_group.set_idents(RepeatedField::from_vec(ident_list));
        new_groups.push(new_group)
    }

    new_rdeps.set_rdeps(new_groups);
    Ok(new_rdeps)
}

#[allow(clippy::needless_pass_by_value)]
fn get_job(req: HttpRequest, path: Path<String>) -> HttpResponse {
    let id_str = path.into_inner();

    let job_id = match id_str.parse::<u64>() {
        Ok(id) => id,
        Err(e) => {
            debug!("Error finding id. e = {:?}", e);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    match do_get_job(&req, job_id) {
        Ok(body) => HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .body(body),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_job_log(
    req: HttpRequest,
    path: Path<String>,
    pagination: Query<JobLogPagination>,
) -> HttpResponse {
    let id_str = path.into_inner();

    let job_id = match id_str.parse::<u64>() {
        Ok(id) => id,
        Err(e) => {
            debug!("Error finding id. e = {:?}", e);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    match do_get_job_log(&req, job_id, pagination.start).await {
        Ok(mut job_log) => {
            if !pagination.color {
                job_log.strip_ansi();
            }
            HttpResponse::Ok().json(job_log)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn promote_job_group(
    req: HttpRequest,
    path: Path<(String, String)>,
    body: Json<GroupPromoteReq>,
) -> HttpResponse {
    let (group_id, channel) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    match promote_or_demote_job_group(&req, &group_id, &body.idents, &channel, true).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn demote_job_group(
    req: HttpRequest,
    path: Path<(String, String)>,
    body: Json<GroupDemoteReq>,
) -> HttpResponse {
    let (group_id, channel) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    match promote_or_demote_job_group(&req, &group_id, &body.idents, &channel, false).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn cancel_job_group(req: HttpRequest, path: Path<String>) -> HttpResponse {
    let id_str = path.into_inner();

    let group_id = match id_str.parse::<u64>() {
        Ok(id) => id,
        Err(e) => {
            debug!("Error finding id. e = {:?}", e);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    match do_cancel_job_group(&req, group_id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// Internal - these functions should return Result<..>
//
fn do_group_promotion_or_demotion(
    req: &HttpRequest,
    channel: &ChannelIdent,
    projects: Vec<&jobsrv::JobGroupProject>,
    origin: &str,
    target: PackageTarget,
    promote: bool,
) -> Result<Vec<i64>> {
    let session = authorize_session(req, Some(&origin), Some(OriginMemberRole::Maintainer))?;

    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    let channel = match Channel::get(origin, channel, &*conn) {
        Ok(channel) => channel,
        Err(NotFound) => {
            if (channel != &ChannelIdent::stable()) && (channel != &ChannelIdent::unstable()) {
                Channel::create(
                    &CreateChannel {
                        name: channel.as_str(),
                        origin,
                        owner_id: session.get_id() as i64,
                    },
                    &*conn,
                )?
            } else {
                warn!("Unable to retrieve default channel: {}", channel);
                return Err(Error::DieselError(NotFound));
            }
        }
        Err(e) => {
            info!("Unable to retrieve channel, err: {:?}", e);
            return Err(Error::DieselError(e));
        }
    };

    let mut package_ids = Vec::new();

    for project in projects {
        req_state(req)
            .memcache
            .borrow_mut()
            .clear_cache_for_package(
                &OriginPackageIdent::from_str(project.get_ident())
                    .unwrap()
                    .into(),
            );

        let op = Package::get(
            GetPackage {
                ident: BuilderPackageIdent(PackageIdent::from_str(project.get_ident()).unwrap()),
                visibility: helpers::all_visibilities(),
                target: BuilderPackageTarget(target),
            },
            &*conn,
        )?;

        package_ids.push(op.id);
    }

    if promote {
        Channel::promote_packages(channel.id, &package_ids, &*conn)?;
    } else {
        Channel::demote_packages(channel.id, &package_ids, &*conn)?;
    }

    Ok(package_ids)
}

async fn promote_or_demote_job_group(
    req: &HttpRequest,
    group_id_str: &str,
    idents: &[String],
    channel: &ChannelIdent,
    promote: bool,
) -> Result<()> {
    authorize_session(&req, None, Some(OriginMemberRole::Maintainer))?;

    let group_id = match group_id_str.parse::<u64>() {
        Ok(g) => g,
        Err(err) => {
            debug!("Error parsing group id: '{}': {:?}", group_id_str, err);
            return Err(Error::BadRequest);
        }
    };

    let mut group_get = jobsrv::JobGroupGet::new();
    group_get.set_group_id(group_id);
    group_get.set_include_projects(true);
    let group = route_message::<jobsrv::JobGroupGet, jobsrv::JobGroup>(req, &group_get).await?;
    let target = PackageTarget::from_str(group.get_target()).unwrap();

    let mut origin_map = HashMap::new();
    let mut ident_map = HashMap::new();

    let has_idents = if !idents.is_empty() {
        for ident in idents.iter() {
            ident_map.insert(ident.clone(), 1);
        }
        true
    } else {
        false
    };

    // We can't assume that every project in the group belongs to the same origin. It's entirely
    // possible that there are multiple origins present within the group. Because of this, there's
    // no way to atomically commit the entire promotion/demotion at once. It's possible origin
    // shards can be on different machines, so for now, the best we can do is partition the projects
    // by origin, and commit each origin at once. Ultimately, it'd be nice to have a way to
    // atomically commit the entire promotion/demotion at once, but that would require a cross-shard
    // tool that we don't currently have.
    for project in group.get_projects().iter() {
        if project.get_state() == jobsrv::JobGroupProjectState::Success {
            let ident_str = project.get_ident();
            if has_idents && !ident_map.contains_key(ident_str) {
                continue;
            }

            let ident = OriginPackageIdent::from_str(ident_str).unwrap();
            let project_list = origin_map
                .entry(ident.get_origin().to_string())
                .or_insert_with(Vec::new);
            project_list.push(project);
        }
    }

    let jgt = helpers::trigger_from_request(req);
    let trigger = PackageChannelTrigger::from(jgt);
    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    for (origin, projects) in origin_map.iter() {
        match do_group_promotion_or_demotion(
            req,
            channel,
            projects.to_vec(),
            &origin,
            target,
            promote,
        ) {
            Ok(package_ids) => {
                let pco = if promote {
                    PackageChannelOperation::Promote
                } else {
                    PackageChannelOperation::Demote
                };

                // TODO: This feels like with the added weight of calls to authorize
                // session, this might not be the best way to handle passing the session
                // info in. We probably should consider a refactor here.

                let session =
                    authorize_session(req, None, Some(OriginMemberRole::Maintainer)).unwrap(); // Unwrap ok

                PackageGroupChannelAudit::audit(
                    PackageGroupChannelAudit {
                        origin: &origin,
                        channel: channel.as_str(),
                        package_ids,
                        operation: pco,
                        trigger: trigger.clone(),
                        requester_id: session.get_id() as i64,
                        requester_name: session.get_name(),
                        group_id: group_id as i64,
                    },
                    &*conn,
                )?;
            }
            Err(e) => {
                debug!("Failed to promote or demote group, err: {:?}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

// TODO: this should be redesigned to not have fan-out, and also to return
// a Job instead of a String
fn do_get_job(req: &HttpRequest, job_id: u64) -> Result<String> {
    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    match Job::get(job_id as i64, &*conn) {
        Ok(job) => {
            let job: jobsrv::Job = job.into();
            authorize_session(
                req,
                Some(&job.get_project().get_origin_name()),
                Some(OriginMemberRole::Member),
            )?;

            if job.get_package_ident().fully_qualified() {
                let builder_package_ident = BuilderPackageIdent(job.get_package_ident().into());
                let channels = channels_for_package_ident(
                    req,
                    &builder_package_ident,
                    PackageTarget::from_str(job.get_target()).unwrap(),
                    &*conn,
                )?;
                let platforms = platforms_for_package_ident(req, &builder_package_ident)?;
                let mut job_json = serde_json::to_value(job).unwrap();

                if channels.is_some() {
                    job_json["channels"] = json!(channels);
                }

                if platforms.is_some() {
                    job_json["platforms"] = json!(platforms);
                }

                Ok(serde_json::to_string(&job_json).unwrap())
            } else {
                Ok(serde_json::to_string(&job).unwrap())
            }
        }
        Err(err) => Err(Error::DieselError(err)),
    }
}

async fn do_get_job_log(req: &HttpRequest, job_id: u64, start: u64) -> Result<jobsrv::JobLog> {
    let mut job_get = jobsrv::JobGet::new();
    let mut request = jobsrv::JobLogGet::new();
    request.set_start(start);
    request.set_id(job_id);
    job_get.set_id(job_id);

    // Before fetching the logs, we need to check and see if the logs we want to fetch are for
    // a job that's building a private package, and if so, do we have the right to see said
    // package.
    match route_message::<jobsrv::JobGet, jobsrv::Job>(&req, &job_get).await {
        Ok(job) => {
            // It's not sufficient to check the project that's on the job itself, since that
            // project is reconstructed from information available in the database and does
            // not contain things like visibility settings. We need to fetch the project from
            // database.
            // TODO (SA): Update the project information in the job to match the DB
            let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;
            let target = if feat::is_enabled(feat::LegacyProject) {
                "x86_64-linux"
            } else {
                job.get_target()
            };
            let project = Project::get(job.get_project().get_name(), target, &*conn)?;
            let settings = OriginPackageSettings::get(
                &GetOriginPackageSettings {
                    origin: job.get_project().get_origin_name(),
                    name: job.get_project().get_package_name(),
                },
                &*conn,
            )?;

            if vec![PackageVisibility::Private, PackageVisibility::Hidden]
                .contains(&settings.visibility)
            {
                authorize_session(req, Some(&project.origin), Some(OriginMemberRole::Member))?;
            }

            route_message::<jobsrv::JobLogGet, jobsrv::JobLog>(req, &request).await
        }
        Err(err) => Err(err),
    }
}

async fn do_cancel_job_group(req: &HttpRequest, group_id: u64) -> Result<NetOk> {
    let mut jgg = jobsrv::JobGroupGet::new();
    jgg.set_group_id(group_id);
    jgg.set_include_projects(true);

    let group = route_message::<jobsrv::JobGroupGet, jobsrv::JobGroup>(req, &jgg).await?;

    let name_split: Vec<&str> = group.get_project_name().split('/').collect();
    assert!(name_split.len() == 2);

    let session = authorize_session(
        req,
        Some(&name_split[0]),
        Some(OriginMemberRole::Maintainer),
    )?;

    let mut jgc = jobsrv::JobGroupCancel::new();
    jgc.set_group_id(group_id);
    jgc.set_trigger(helpers::trigger_from_request(req));
    jgc.set_requester_id(session.get_id());
    jgc.set_requester_name(session.get_name().to_string());

    route_message::<jobsrv::JobGroupCancel, NetOk>(req, &jgc).await
}
