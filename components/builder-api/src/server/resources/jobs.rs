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

use std::collections::HashMap;
use std::str::FromStr;

use actix_web::http::{self, Method, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Json, Path, Query};
use serde_json;

use protocol::jobsrv;
use protocol::originsrv::OriginPackageIdent;

use hab_core::channel::{STABLE_CHANNEL, UNSTABLE_CHANNEL};
use hab_core::package::{Identifiable, PackageIdent, PackageTarget};
use hab_net::{ErrCode, NetError, NetOk};

use db::models::channel::*;
use db::models::package::*;
use db::models::projects::*;
use diesel::result::Error::NotFound;

use server::authorize::authorize_session;
use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers::{self, Target};
use server::resources::channels::channels_for_package_ident;
use server::resources::pkgs::platforms_for_package_ident;
use server::AppState;

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
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route(
            "/jobs/group/{id}/promote/{channel}",
            Method::POST,
            promote_job_group,
        ).route(
            "/jobs/group/{id}/demote/{channel}",
            Method::POST,
            demote_job_group,
        ).route("/jobs/group/{id}/cancel", Method::POST, cancel_job_group)
        .route("/rdeps/{origin}/{name}", Method::GET, get_rdeps)
        .route("/jobs/{id}", Method::GET, get_job)
        .route("/jobs/{id}/log", Method::GET, get_job_log)
    }
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_rdeps((qtarget, req): (Query<Target>, HttpRequest<AppState>)) -> HttpResponse {
    let (origin, name) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    // TODO: Deprecate target from headers
    let target = match qtarget.target.clone() {
        Some(t) => {
            debug!("Query requested target = {}", t);
            match PackageTarget::from_str(&t) {
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
    {
        Ok(rdeps) => HttpResponse::Ok().json(rdeps),
        Err(err) => err.into(),
    }
}

fn get_job(req: HttpRequest<AppState>) -> HttpResponse {
    let id_str = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

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
        Err(err) => err.into(),
    }
}

fn get_job_log(
    (pagination, req): (Query<JobLogPagination>, HttpRequest<AppState>),
) -> HttpResponse {
    let id_str = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let job_id = match id_str.parse::<u64>() {
        Ok(id) => id,
        Err(e) => {
            debug!("Error finding id. e = {:?}", e);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    match do_get_job_log(&req, job_id, pagination.start) {
        Ok(mut job_log) => {
            if !pagination.color {
                job_log.strip_ansi();
            }
            HttpResponse::Ok().json(job_log)
        }
        Err(err) => err.into(),
    }
}

fn promote_job_group((req, body): (HttpRequest<AppState>, Json<GroupPromoteReq>)) -> HttpResponse {
    let (group_id, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    match promote_or_demote_job_group(&req, group_id, &body.idents, &channel, true) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn demote_job_group((req, body): (HttpRequest<AppState>, Json<GroupDemoteReq>)) -> HttpResponse {
    let (group_id, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    match promote_or_demote_job_group(&req, group_id, &body.idents, &channel, false) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn cancel_job_group(req: HttpRequest<AppState>) -> HttpResponse {
    let id_str = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let group_id = match id_str.parse::<u64>() {
        Ok(id) => id,
        Err(e) => {
            debug!("Error finding id. e = {:?}", e);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    match do_cancel_job_group(&req, group_id) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

//
// Internal - these functions should return Result<..>
//
fn do_group_promotion_or_demotion(
    req: &HttpRequest<AppState>,
    channel: &str,
    projects: Vec<&jobsrv::JobGroupProject>,
    origin: &str,
    promote: bool,
) -> Result<Vec<i64>> {
    let session = authorize_session(req, Some(&origin))?;

    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    let channel = match Channel::get(origin, channel, &*conn) {
        Ok(channel) => channel,
        Err(NotFound) => {
            if channel != STABLE_CHANNEL || channel != UNSTABLE_CHANNEL {
                Channel::create(
                    CreateChannel {
                        name: channel,
                        origin: origin,
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
        req.state().memcache.borrow_mut().clear_cache_for_package(
            OriginPackageIdent::from_str(project.get_ident())
                .unwrap()
                .into(),
        );

        // TODO (SA): Expand build targets - currently Builder only supports x86_64-linux
        let op = Package::get(
            GetPackage {
                ident: BuilderPackageIdent(
                    PackageIdent::from_str(project.get_ident().clone()).unwrap(),
                ),
                visibility: helpers::all_visibilities(),
                target: BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap()), // Unwrap OK
            },
            &*conn,
        )?;

        package_ids.push(op.id);
    }

    if promote {
        Channel::promote_packages(channel.id, package_ids.clone(), &*conn)?;
    } else {
        Channel::demote_packages(channel.id, package_ids.clone(), &*conn)?;
    }

    Ok(package_ids)
}

fn promote_or_demote_job_group(
    req: &HttpRequest<AppState>,
    group_id_str: String,
    idents: &Vec<String>,
    channel: &str,
    promote: bool,
) -> Result<()> {
    authorize_session(&req, None)?;

    let group_id = match group_id_str.parse::<u64>() {
        Ok(g) => g,
        Err(err) => {
            debug!("Error parsing group id: '{}': {:?}", group_id_str, err);
            return Err(Error::ParseIntError(err));
        }
    };

    let mut group_get = jobsrv::JobGroupGet::new();
    group_get.set_group_id(group_id);
    group_get.set_include_projects(true);
    let group = route_message::<jobsrv::JobGroupGet, jobsrv::JobGroup>(req, &group_get)?;

    // This only makes sense if the group is complete. If the group isn't complete, return now and
    // let the user know. Check the completion state by checking the individual project states,
    // as if this is called by the scheduler it needs to promote/demote the group before marking it
    // Complete.
    if group.get_projects().iter().any(|&ref p| {
        p.get_state() == jobsrv::JobGroupProjectState::NotStarted
            || p.get_state() == jobsrv::JobGroupProjectState::InProgress
    }) {
        return Err(Error::NetError(NetError::new(
            ErrCode::GROUP_NOT_COMPLETE,
            "hg:promote-or-demote-job-group:0",
        )));
    }

    let mut origin_map = HashMap::new();

    let mut ident_map = HashMap::new();
    let has_idents = if idents.len() > 0 {
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
    for project in group.get_projects().into_iter() {
        if project.get_state() == jobsrv::JobGroupProjectState::Success {
            let ident_str = project.get_ident();
            if has_idents && !ident_map.contains_key(ident_str) {
                continue;
            }

            let ident = OriginPackageIdent::from_str(ident_str).unwrap();
            let project_list = origin_map
                .entry(ident.get_origin().to_string())
                .or_insert(Vec::new());
            project_list.push(project);
        }
    }

    let jgt = helpers::trigger_from_request(req);
    let trigger = PackageChannelTrigger::from(jgt);
    let conn = req.state().db.get_conn().map_err(Error::DbError)?;

    for (origin, projects) in origin_map.iter() {
        match do_group_promotion_or_demotion(req, channel, projects.to_vec(), &origin, promote) {
            Ok(package_ids) => {
                let pco = if promote {
                    PackageChannelOperation::Promote
                } else {
                    PackageChannelOperation::Demote
                };

                let session = authorize_session(req, None).unwrap(); // Unwrap ok

                PackageGroupChannelAudit::audit(
                    PackageGroupChannelAudit {
                        origin: &origin,
                        channel: &channel,
                        package_ids: package_ids,
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
                warn!("Failed to promote or demote group, err: {:?}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

// TODO: this should be redesigned to not have fan-out, and also to return
// a Job instead of a String
fn do_get_job(req: &HttpRequest<AppState>, job_id: u64) -> Result<String> {
    let mut request = jobsrv::JobGet::new();
    request.set_id(job_id);

    match route_message::<jobsrv::JobGet, jobsrv::Job>(req, &request) {
        Ok(job) => {
            debug!("job = {:?}", &job);

            authorize_session(req, Some(&job.get_project().get_origin_name()))?;

            if job.get_package_ident().fully_qualified() {
                let builder_package_ident = BuilderPackageIdent(job.get_package_ident().into());
                let channels = channels_for_package_ident(req, &builder_package_ident)?;
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
        Err(err) => Err(err),
    }
}

fn do_get_job_log(req: &HttpRequest<AppState>, job_id: u64, start: u64) -> Result<jobsrv::JobLog> {
    let mut job_get = jobsrv::JobGet::new();
    let mut request = jobsrv::JobLogGet::new();
    request.set_start(start);
    request.set_id(job_id);
    job_get.set_id(job_id);

    // Before fetching the logs, we need to check and see if the logs we want to fetch are for
    // a job that's building a private package, and if so, do we have the right to see said
    // package.
    match route_message::<jobsrv::JobGet, jobsrv::Job>(&req, &job_get) {
        Ok(job) => {
            // It's not sufficient to check the project that's on the job itself, since that
            // project is reconstructed from information available in the database and does
            // not contain things like visibility settings. We need to fetch the project from
            // database.
            // TODO (SA): Update the project information in the job to match the DB
            let conn = req.state().db.get_conn().map_err(Error::DbError)?;
            let project = Project::get(job.get_project().get_name(), &*conn)?;

            if vec![PackageVisibility::Private, PackageVisibility::Hidden]
                .contains(&project.visibility)
            {
                authorize_session(req, Some(&project.origin))?;
            }

            route_message::<jobsrv::JobLogGet, jobsrv::JobLog>(req, &request)
        }
        Err(err) => Err(err),
    }
}

fn do_cancel_job_group(req: &HttpRequest<AppState>, group_id: u64) -> Result<NetOk> {
    let mut jgg = jobsrv::JobGroupGet::new();
    jgg.set_group_id(group_id);
    jgg.set_include_projects(true);

    let group = route_message::<jobsrv::JobGroupGet, jobsrv::JobGroup>(req, &jgg)?;

    let name_split: Vec<&str> = group.get_project_name().split("/").collect();
    assert!(name_split.len() == 2);

    let session = authorize_session(req, Some(&name_split[0]))?;

    let mut jgc = jobsrv::JobGroupCancel::new();
    jgc.set_group_id(group_id);
    jgc.set_trigger(helpers::trigger_from_request(req));
    jgc.set_requester_id(session.get_id());
    jgc.set_requester_name(session.get_name().to_string());

    route_message::<jobsrv::JobGroupCancel, NetOk>(req, &jgc)
}
