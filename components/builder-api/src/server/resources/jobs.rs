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

use protocol::jobsrv::*;
use protocol::originsrv::*;

use hab_core::channel::{STABLE_CHANNEL, UNSTABLE_CHANNEL};
use hab_core::package::{Identifiable, PackageTarget};
use hab_net::{ErrCode, NetError, NetOk};

use server::authorize::{authorize_session, get_session_user_name};
use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers::{self, Target};
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

    let mut rdeps_get = JobGraphPackageReverseDependenciesGet::new();
    rdeps_get.set_origin(origin);
    rdeps_get.set_name(name);
    rdeps_get.set_target(target.to_string());

    match route_message::<JobGraphPackageReverseDependenciesGet, JobGraphPackageReverseDependencies>(
        &req, &rdeps_get,
    ) {
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
    let job_id = Path::<u64>::extract(&req).unwrap().into_inner(); // Unwrap Ok ?

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
    let group_id = Path::<u64>::extract(&req).unwrap().into_inner(); // Unwrap Ok ?

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
    projects: Vec<&JobGroupProject>,
    origin: &str,
    promote: bool,
) -> Result<Vec<u64>> {
    authorize_session(req, Some(&origin))?;

    let mut ocg = OriginChannelGet::new();
    ocg.set_origin_name(origin.to_string());
    ocg.set_name(channel.to_string());

    let channel = match route_message::<OriginChannelGet, OriginChannel>(req, &ocg) {
        Ok(channel) => channel,
        Err(Error::NetError(e)) => {
            if e.get_code() == ErrCode::ENTITY_NOT_FOUND {
                if channel != STABLE_CHANNEL || channel != UNSTABLE_CHANNEL {
                    helpers::create_channel(req, &origin, channel)?
                } else {
                    info!("Unable to retrieve default channel, err: {:?}", e);
                    return Err(Error::NetError(e));
                }
            } else {
                info!("Unable to retrieve channel, err: {:?}", e);
                return Err(Error::NetError(e));
            }
        }
        Err(e) => {
            info!("Unable to retrieve channel, err: {:?}", e);
            return Err(e);
        }
    };

    let mut package_ids = Vec::new();

    for project in projects {
        let opi = OriginPackageIdent::from_str(project.get_ident()).unwrap();
        let mut opg = OriginPackageGet::new();
        opg.set_ident(opi);
        opg.set_visibilities(helpers::all_visibilities());

        req.state().memcache.borrow_mut().clear_cache_for_package(
            OriginPackageIdent::from_str(project.get_ident())
                .unwrap()
                .into(),
        );

        let op = route_message::<OriginPackageGet, OriginPackage>(req, &opg)?;
        package_ids.push(op.get_id());
    }

    if promote {
        let mut opgp = OriginPackageGroupPromote::new();
        opgp.set_channel_id(channel.get_id());
        opgp.set_package_ids(package_ids.clone());
        opgp.set_origin(origin.to_string());

        route_message::<OriginPackageGroupPromote, NetOk>(req, &opgp)?;
    } else {
        let mut opgp = OriginPackageGroupDemote::new();
        opgp.set_channel_id(channel.get_id());
        opgp.set_package_ids(package_ids.clone());
        opgp.set_origin(origin.to_string());

        route_message::<OriginPackageGroupDemote, NetOk>(req, &opgp)?;
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

    let mut group_get = JobGroupGet::new();
    group_get.set_group_id(group_id);
    group_get.set_include_projects(true);
    let group = route_message::<JobGroupGet, JobGroup>(req, &group_get)?;

    // This only makes sense if the group is complete. If the group isn't complete, return now and
    // let the user know. Check the completion state by checking the individual project states,
    // as if this is called by the scheduler it needs to promote/demote the group before marking it
    // Complete.
    if group.get_projects().iter().any(|&ref p| {
        p.get_state() == JobGroupProjectState::NotStarted
            || p.get_state() == JobGroupProjectState::InProgress
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
        if project.get_state() == JobGroupProjectState::Success {
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

    for (origin, projects) in origin_map.iter() {
        match do_group_promotion_or_demotion(req, channel, projects.to_vec(), &origin, promote) {
            Ok(package_ids) => {
                let mut pgca = PackageGroupChannelAudit::new();

                let mut channel_get = OriginChannelGet::new();
                channel_get.set_origin_name(origin.clone());
                channel_get.set_name(channel.to_string());
                match route_message::<OriginChannelGet, OriginChannel>(req, &channel_get) {
                    Ok(origin_channel) => pgca.set_channel_id(origin_channel.get_id()),
                    Err(err) => return Err(err),
                }

                let mut origin_get = OriginGet::new();
                origin_get.set_name(origin.clone());
                match route_message::<OriginGet, Origin>(req, &origin_get) {
                    Ok(origin_origin) => pgca.set_origin_id(origin_origin.get_id()),
                    Err(err) => return Err(err),
                }

                pgca.set_package_ids(package_ids);

                if promote {
                    pgca.set_operation(PackageChannelOperation::Promote);
                } else {
                    pgca.set_operation(PackageChannelOperation::Demote);
                }

                let session_id = authorize_session(req, None).unwrap(); // Unwrap ok
                let session_name = get_session_user_name(req, session_id);

                pgca.set_trigger(trigger);
                pgca.set_requester_id(session_id);
                pgca.set_requester_name(session_name);
                pgca.set_group_id(group_id);

                route_message::<PackageGroupChannelAudit, NetOk>(req, &pgca)?;
            }
            Err(Error::NetError(e)) => {
                if e.get_code() != ErrCode::ACCESS_DENIED {
                    warn!("Failed to promote or demote group, err: {:?}", e);
                    return Err(Error::NetError(e));
                }
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
    let mut request = JobGet::new();
    request.set_id(job_id);

    match route_message::<JobGet, Job>(req, &request) {
        Ok(job) => {
            debug!("job = {:?}", &job);

            authorize_session(req, Some(&job.get_project().get_origin_name()))?;

            if job.get_package_ident().fully_qualified() {
                let channels = helpers::channels_for_package_ident(req, job.get_package_ident());
                let platforms = helpers::platforms_for_package_ident(req, job.get_package_ident());
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

fn do_get_job_log(req: &HttpRequest<AppState>, job_id: u64, start: u64) -> Result<JobLog> {
    let mut job_get = JobGet::new();
    let mut request = JobLogGet::new();
    request.set_start(start);
    request.set_id(job_id);
    job_get.set_id(job_id);

    // Before fetching the logs, we need to check and see if the logs we want to fetch are for
    // a job that's building a private package, and if so, do we have the right to see said
    // package.
    match route_message::<JobGet, Job>(&req, &job_get) {
        Ok(job) => {
            // It's not sufficient to check the project that's on the job itself, since that
            // project is reconstructed from information available in the jobsrv database and does
            // not contain things like visibility settings. We need to fetch the project from
            // originsrv.
            let mut project_get = OriginProjectGet::new();
            project_get.set_name(job.get_project().get_name().to_string());

            let project = route_message::<OriginProjectGet, OriginProject>(&req, &project_get)?;

            if vec![
                OriginPackageVisibility::Private,
                OriginPackageVisibility::Hidden,
            ].contains(&project.get_visibility())
            {
                authorize_session(req, Some(&project.get_origin_name()))?;
            }

            route_message::<JobLogGet, JobLog>(req, &request)
        }
        Err(err) => Err(err),
    }
}

fn do_cancel_job_group(req: &HttpRequest<AppState>, group_id: u64) -> Result<NetOk> {
    let mut jgg = JobGroupGet::new();
    jgg.set_group_id(group_id);
    jgg.set_include_projects(true);

    let group = route_message::<JobGroupGet, JobGroup>(req, &jgg)?;

    let name_split: Vec<&str> = group.get_project_name().split("/").collect();
    assert!(name_split.len() == 2);

    let session_id = authorize_session(req, Some(&name_split[0]))?;
    let session_name = get_session_user_name(req, session_id);

    let mut jgc = JobGroupCancel::new();
    jgc.set_group_id(group_id);
    jgc.set_trigger(helpers::trigger_from_request(req));
    jgc.set_requester_id(session_id);
    jgc.set_requester_name(session_name);

    route_message::<JobGroupCancel, NetOk>(req, &jgc)
}
