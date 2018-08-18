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

use actix_web::http::{self, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Json, Path};

use protocol::jobsrv::*;
use protocol::originsrv::*;

use hab_core::channel::{STABLE_CHANNEL, UNSTABLE_CHANNEL};
use hab_core::package::ident;
use hab_net::{ErrCode, NetError, NetOk};

use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers;
use server::AppState;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GroupPromoteReq {
    pub idents: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GroupDemoteReq {
    pub idents: Vec<String>,
}

pub struct Jobs;

impl Jobs {
    // Internal - these functions should return Result<..>
    fn do_group_promotion_or_demotion(
        req: &HttpRequest<AppState>,
        channel: &str,
        projects: Vec<&JobGroupProject>,
        origin: &str,
        promote: bool,
    ) -> Result<Vec<u64>> {
        helpers::check_origin_access(req, &origin)?;

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
        group_id: u64,
        idents: Option<Vec<String>>,
        channel: &str,
        promote: bool,
    ) -> Result<()> {
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
        let has_idents = if idents.is_some() {
            for ident in idents.unwrap().iter() {
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
            match Self::do_group_promotion_or_demotion(
                req,
                channel,
                projects.to_vec(),
                &origin,
                promote,
            ) {
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

                    let (session_id, session_name) = helpers::get_session_id_and_name(req);

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

    // Route handlers - these functions should return HttpResponse

    // Route registration
    pub fn register(app: App<AppState>) -> App<AppState> {
        app
    }
}

// JOBS HANDLERS - "/v1/jobs/..."

/*

        if feat::is_enabled(feat::Jobsrv) {
            r.post(
                "/jobs/group/:id/promote/:channel",
                XHandler::new(job_group_promote).before(basic.clone()),
                "job_group_promote",
            );
            r.post(
                "/jobs/group/:id/demote/:channel",
                XHandler::new(job_group_demote).before(basic.clone()),
                "job_group_demote",
            );
            r.post(
                "/jobs/group/:id/cancel",
                XHandler::new(job_group_cancel).before(basic.clone()),
                "job_group_cancel",
            );
            r.get("/rdeps/:origin/:name", rdeps_show, "rdeps");
            r.get(
                "/jobs/:id",
                XHandler::new(job_show).before(basic.clone()),
                "job",
            );
            r.get(
                "/jobs/:id/log",
                XHandler::new(job_log).before(basic.clone()),
                "job_log",
            );
*/

/*

// This route is only available if jobsrv_enabled is true
pub fn rdeps_show(req: &mut Request) -> IronResult<Response> {
    let mut rdeps_get = JobGraphPackageReverseDependenciesGet::new();
    match get_param(req, "origin") {
        Some(origin) => rdeps_get.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "name") {
        Some(name) => rdeps_get.set_name(name),
        None => return Ok(Response::with(status::BadRequest)),
    }

    // TODO (SA): The rdeps API needs to be extended to support a target param.
    // For now, hard code a default value
    rdeps_get.set_target("x86_64-linux".to_string());

    match route_message::<JobGraphPackageReverseDependenciesGet, JobGraphPackageReverseDependencies>(
        req, &rdeps_get,
    ) {
        Ok(rdeps) => Ok(render_json(status::Ok, &rdeps)),
        Err(err) => return Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn job_show(req: &mut Request) -> IronResult<Response> {
    let mut request = JobGet::new();
    match get_param(req, "id") {
        Some(id) => match id.parse::<u64>() {
            Ok(i) => request.set_id(i),
            Err(e) => {
                debug!("Error finding id. e = {:?}", e);
                return Ok(Response::with(status::BadRequest));
            }
        },
        None => return Ok(Response::with(status::BadRequest)),
    }

    match route_message::<JobGet, Job>(req, &request) {
        Ok(job) => {
            debug!("job = {:?}", &job);

            if !check_origin_access(req, job.get_project().get_origin_name()).unwrap_or(false) {
                return Ok(Response::with(status::Forbidden));
            }

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

                Ok(render_json(status::Ok, &job_json))
            } else {
                Ok(render_json(status::Ok, &job))
            }
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn job_log(req: &mut Request) -> IronResult<Response> {
    let start = req
        .get_ref::<Params>()
        .unwrap()
        .find(&["start"])
        .and_then(FromValue::from_value)
        .unwrap_or(0);

    let include_color = req
        .get_ref::<Params>()
        .unwrap()
        .find(&["color"])
        .and_then(FromValue::from_value)
        .unwrap_or(false);

    let mut job_get = JobGet::new();
    let mut request = JobLogGet::new();
    request.set_start(start);

    match get_param(req, "id") {
        Some(id) => match id.parse::<u64>() {
            Ok(i) => {
                request.set_id(i);
                job_get.set_id(i);
            }
            Err(e) => {
                debug!("Error parsing id. e = {:?}", e);
                return Ok(Response::with(status::BadRequest));
            }
        },
        None => return Ok(Response::with(status::BadRequest)),
    }

    // Before fetching the logs, we need to check and see if the logs we want to fetch are for
    // a job that's building a private package, and if so, do we have the right to see said
    // package.
    match route_message::<JobGet, Job>(req, &job_get) {
        Ok(job) => {
            // It's not sufficient to check the project that's on the job itself, since that
            // project is reconstructed from information available in the jobsrv database and does
            // not contain things like visibility settings. We need to fetch the project from
            // originsrv.
            let mut project_get = OriginProjectGet::new();
            project_get.set_name(job.get_project().get_name().to_string());

            let project = match route_message::<OriginProjectGet, OriginProject>(req, &project_get)
            {
                Ok(p) => p,
                Err(err) => return Ok(render_net_error(&err)),
            };

            if vec![
                OriginPackageVisibility::Private,
                OriginPackageVisibility::Hidden,
            ].contains(&project.get_visibility())
            {
                if !check_origin_access(req, project.get_origin_name()).unwrap_or(false) {
                    return Ok(Response::with(status::Forbidden));
                }
            }

            match route_message::<JobLogGet, JobLog>(req, &request) {
                Ok(mut log) => {
                    if !include_color {
                        log.strip_ansi();
                    }
                    Ok(render_json(status::Ok, &log))
                }
                Err(err) => Ok(render_net_error(&err)),
            }
        }
        Err(e) => return Ok(render_net_error(&e)),
    }
}


// This route is only available if jobsrv_enabled is true
pub fn job_group_promote(req: &mut Request) -> IronResult<Response> {
    job_group_promote_or_demote(req, true)
}

// This route is only available if jobsrv_enabled is true
pub fn job_group_demote(req: &mut Request) -> IronResult<Response> {
    job_group_promote_or_demote(req, false)
}

// This route is only available if jobsrv_enabled is true
fn job_group_promote_or_demote(req: &mut Request, promote: bool) -> IronResult<Response> {
    let group_id = match get_param(req, "id") {
        Some(id) => match id.parse::<u64>() {
            Ok(g) => g,
            Err(e) => {
                debug!("Error finding group. e = {:?}", e);
                return Ok(Response::with(status::BadRequest));
            }
        },
        None => return Ok(Response::with(status::BadRequest)),
    };

    let channel = match get_param(req, "channel") {
        Some(c) => c,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let idents = if promote {
        match req.get::<bodyparser::Struct<GroupPromoteReq>>() {
            Ok(Some(gpr)) => Some(gpr.idents),
            Ok(None) => None,
            Err(err) => {
                debug!("Error decoding json struct: {:?}", err);
                return Ok(Response::with(status::BadRequest));
            }
        }
    } else {
        match req.get::<bodyparser::Struct<GroupDemoteReq>>() {
            Ok(Some(gpr)) => Some(gpr.idents),
            Ok(None) => None,
            Err(err) => {
                debug!("Error decoding json struct: {:?}", err);
                return Ok(Response::with(status::BadRequest));
            }
        }
    };

    match helpers::promote_or_demote_job_group(req, group_id, idents, &channel, promote) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn job_group_cancel(req: &mut Request) -> IronResult<Response> {
    let (session_id, mut session_name) = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        (session.get_id(), session.get_name().to_string())
    };

    // Sessions created via Personal Access Tokens only have ids, so we may need
    // to get the username explicitly.
    if session_name.is_empty() {
        session_name = get_session_user_name(req, session_id)
    }

    let group_id = match get_param(req, "id") {
        Some(id) => match id.parse::<u64>() {
            Ok(g) => g,
            Err(e) => {
                debug!("Error finding group. e = {:?}", e);
                return Ok(Response::with(status::BadRequest));
            }
        },
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut jgg = JobGroupGet::new();
    jgg.set_group_id(group_id);
    jgg.set_include_projects(true);

    let group = match route_message::<JobGroupGet, JobGroup>(req, &jgg) {
        Ok(group) => group,
        Err(err) => return Ok(render_net_error(&err)),
    };

    let name_split: Vec<&str> = group.get_project_name().split("/").collect();
    assert!(name_split.len() == 2);

    if !check_origin_access(req, &name_split[0]).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut jgc = JobGroupCancel::new();
    jgc.set_group_id(group_id);
    jgc.set_trigger(trigger_from_request(req));
    jgc.set_requester_id(session_id);
    jgc.set_requester_name(session_name);

    match route_message::<JobGroupCancel, NetOk>(req, &jgc) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

*/
