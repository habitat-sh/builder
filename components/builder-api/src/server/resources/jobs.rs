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

use actix_web::http::{self, StatusCode};
use actix_web::FromRequest;
use actix_web::{HttpRequest, HttpResponse, Json, Path};
use protocol::originsrv::*;

use hab_core::package::ident;

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
