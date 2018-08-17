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
use actix_web::{App, HttpRequest, HttpResponse, Json, Path};
use protocol::originsrv::*;

use hab_core::package::ident;

use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers;
use server::AppState;

// A default name for per-project integrations. Currently, there
// can only be one.
const DEFAULT_PROJECT_INTEGRATION: &'static str = "default";

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectCreateReq {
    pub origin: String,
    pub plan_path: String,
    pub installation_id: u32,
    pub repo_id: u32,
    pub auto_build: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectUpdateReq {
    pub plan_path: String,
    pub installation_id: u32,
    pub repo_id: u32,
    pub auto_build: bool,
}

pub struct Projects;

impl Projects {
    // Internal - these functions should return Result<..>

    // Route handlers - these functions should return HttpResponse

    // Route registration
    pub fn register(app: App<AppState>) -> App<AppState> {
        app
    }
}

// PROJECTS HANLDERS - "/projects/..."

/*

            r.post(
                "/projects",
                XHandler::new(project_create).before(basic.clone()),
                "projects",
            );
            r.get(
                "/projects/:origin/:name",
                XHandler::new(project_show).before(basic.clone()),
                "project",
            );
            r.get(
                "/projects/:origin",
                XHandler::new(project_list).before(basic.clone()),
                "project_list",
            );
            r.get(
                "/projects/:origin/:name/jobs",
                XHandler::new(project_jobs).before(basic.clone()),
                "project_jobs",
            );
            r.put(
                "/projects/:origin/:name",
                XHandler::new(project_update).before(basic.clone()),
                "edit_project",
            );
            r.delete(
                "/projects/:origin/:name",
                XHandler::new(project_delete).before(basic.clone()),
                "delete_project",
            );
            r.patch(
                "/projects/:origin/:name/:visibility",
                XHandler::new(project_privacy_toggle).before(basic.clone()),
                "project_privacy_toggle",
            );
            r.get(
                "/projects/:origin/:name/integrations/:integration/default",
                XHandler::new(get_project_integration).before(basic.clone()),
                "project_integration_get",
            );
            r.put(
                "/projects/:origin/:name/integrations/:integration/default",
                XHandler::new(create_project_integration).before(basic.clone()),
                "project_integration_put",
            );
            r.delete(
                "/projects/:origin/:name/integrations/:integration/default",
                XHandler::new(delete_project_integration).before(basic.clone()),
                "project_integration_delete",
            );

*/

/*

/// Create a new project as the authenticated user and associated to
/// the given origin.
// This route is only available if jobsrv_enabled is true
pub fn project_create(req: &mut Request) -> IronResult<Response> {
    let mut request = OriginProjectCreate::new();
    let mut project = OriginProject::new();
    let mut origin_get = OriginGet::new();
    let github = req.get::<persistent::Read<GitHubCli>>().unwrap();
    let session = req.extensions.get::<Authenticated>().unwrap().clone();

    let (token, repo_id) = match req.get::<bodyparser::Struct<ProjectCreateReq>>() {
        Ok(Some(body)) => {
            if body.origin.len() <= 0 {
                return Ok(Response::with((
                    status::UnprocessableEntity,
                    "Missing value for field: `origin`",
                )));
            }
            if body.plan_path.len() <= 0 {
                return Ok(Response::with((
                    status::UnprocessableEntity,
                    "Missing value for field: `plan_path`",
                )));
            }

            if !check_origin_access(req, &body.origin).unwrap_or(false) {
                return Ok(Response::with(status::Forbidden));
            }

            debug!(
                "GITHUB-CALL builder_api::server::handlers::project_create: Getting app_installation_token; repo_id={} installation_id={}",
                body.repo_id,
                body.installation_id
            );

            let token = match github.app_installation_token(body.installation_id) {
                Ok(token) => token,
                Err(err) => {
                    warn!("Error authenticating github app installation, {}", err);
                    return Ok(Response::with(status::Forbidden));
                }
            };

            origin_get.set_name(body.origin);
            project.set_plan_path(body.plan_path);
            project.set_vcs_type(String::from("git"));
            project.set_vcs_installation_id(body.installation_id);
            project.set_auto_build(body.auto_build);

            match github.repo(&token, body.repo_id) {
                Ok(Some(repo)) => project.set_vcs_data(repo.clone_url),
                Ok(None) => return Ok(Response::with((status::NotFound, "rg:pc:2"))),
                Err(e) => {
                    warn!("Error finding github repo. e = {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "rg:pc:1")));
                }
            }
            (token, body.repo_id)
        }
        Ok(None) => {
            debug!("Project JSON returned None");
            return Ok(Response::with(status::UnprocessableEntity));
        }
        Err(e) => {
            debug!("Error parsing project JSON: {:?}", e);
            return Ok(Response::with(status::UnprocessableEntity));
        }
    };

    let origin = match route_message::<OriginGet, Origin>(req, &origin_get) {
        Ok(response) => response,
        Err(err) => return Ok(render_net_error(&err)),
    };

    match github.contents(&token, repo_id, &project.get_plan_path()) {
        Ok(Some(contents)) => match contents.decode() {
            Ok(bytes) => match Plan::from_bytes(bytes.as_slice()) {
                Ok(plan) => {
                    project.set_origin_name(String::from(origin.get_name()));
                    project.set_origin_id(origin.get_id());
                    project.set_package_name(String::from(plan.name.trim_matches('"')));
                }
                Err(e) => {
                    debug!("Error matching Plan. e = {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "rg:pc:3")));
                }
            },
            Err(e) => {
                error!("Base64 decode failure: {:?}", e);
                return Ok(Response::with((status::UnprocessableEntity, "rg:pc:4")));
            }
        },
        Ok(None) => return Ok(Response::with((status::NotFound, "rg:pc:5"))),
        Err(e) => {
            warn!("Error fetching contents from GH. e = {:?}", e);
            return Ok(Response::with((status::UnprocessableEntity, "rg:pc:2")));
        }
    }

    project.set_owner_id(session.get_id());
    project.set_visibility(origin.get_default_package_visibility());
    request.set_project(project);
    match route_message::<OriginProjectCreate, OriginProject>(req, &request) {
        Ok(response) => Ok(render_json(status::Created, &response)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Delete the given project
// This route is only available if jobsrv_enabled is true
pub fn project_delete(req: &mut Request) -> IronResult<Response> {
    let mut project_del = OriginProjectDelete::new();

    let session_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let name = match get_param(req, "name") {
        Some(n) => n,
        None => return Ok(Response::with(status::BadRequest)),
    };

    project_del.set_name(format!("{}/{}", &origin, &name));

    if !check_origin_access(req, origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    project_del.set_requestor_id(session_id);
    match route_message::<OriginProjectDelete, NetOk>(req, &project_del) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Update the given project
// This route is only available if jobsrv_enabled is true
pub fn project_update(req: &mut Request) -> IronResult<Response> {
    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let name = match get_param(req, "name") {
        Some(n) => n,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let session_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut project_get = OriginProjectGet::new();
    project_get.set_name(format!("{}/{}", &origin, &name));
    let mut project = match route_message::<OriginProjectGet, OriginProject>(req, &project_get) {
        Ok(project) => project,
        Err(err) => return Ok(render_net_error(&err)),
    };

    let mut request = OriginProjectUpdate::new();
    let github = req.get::<persistent::Read<GitHubCli>>().unwrap();

    let (token, repo_id) = match req.get::<bodyparser::Struct<ProjectUpdateReq>>() {
        Ok(Some(body)) => {
            if body.plan_path.len() <= 0 {
                return Ok(Response::with((
                    status::UnprocessableEntity,
                    "Missing value for field: `plan_path`",
                )));
            }

            debug!(
                "GITHUB-CALL builder_api::server::handlers::project_update: Getting app_installation_token; repo_id={} installation_id={}",
                body.repo_id,
                body.installation_id
            );

            let token = match github.app_installation_token(body.installation_id) {
                Ok(token) => token,
                Err(err) => {
                    warn!("Error authenticating github app installation, {}", err);
                    return Ok(Response::with(status::Forbidden));
                }
            };

            project.set_auto_build(body.auto_build);
            project.set_plan_path(body.plan_path);
            project.set_vcs_installation_id(body.installation_id);
            match github.repo(&token, body.repo_id) {
                Ok(Some(repo)) => project.set_vcs_data(repo.clone_url),
                Ok(None) => return Ok(Response::with((status::NotFound, "rg:pu:2"))),
                Err(e) => {
                    warn!("Error finding GH repo. e = {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "rg:pu:1")));
                }
            }
            (token, body.repo_id)
        }
        _ => return Ok(Response::with(status::UnprocessableEntity)),
    };

    match github.contents(&token, repo_id, &project.get_plan_path()) {
        Ok(Some(contents)) => match contents.decode() {
            Ok(bytes) => match Plan::from_bytes(bytes.as_slice()) {
                Ok(plan) => {
                    debug!("plan = {:?}", &plan);
                    if plan.name != name {
                        return Ok(Response::with((status::UnprocessableEntity, "rg:pu:7")));
                    }
                    project.set_origin_name(String::from(origin));
                    project.set_package_name(String::from(name));
                }
                Err(e) => {
                    debug!("Error matching Plan. e = {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "rg:pu:3")));
                }
            },
            Err(e) => {
                debug!("Error decoding content from b64. e = {:?}", e);
                return Ok(Response::with((status::UnprocessableEntity, "rg:pu:4")));
            }
        },
        Ok(None) => return Ok(Response::with((status::NotFound, "rg:pu:6"))),
        Err(e) => {
            warn!("Erroring fetching contents from GH. e = {:?}", e);
            return Ok(Response::with((status::UnprocessableEntity, "rg:pu:5")));
        }
    }

    request.set_requestor_id(session_id);
    request.set_project(project);
    match route_message::<OriginProjectUpdate, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Display the the given project's details
// This route is only available if jobsrv_enabled is true
pub fn project_show(req: &mut Request) -> IronResult<Response> {
    let mut project_get = OriginProjectGet::new();

    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let name = match get_param(req, "name") {
        Some(n) => n,
        None => return Ok(Response::with(status::BadRequest)),
    };

    project_get.set_name(format!("{}/{}", &origin, &name));

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    match route_message::<OriginProjectGet, OriginProject>(req, &project_get) {
        Ok(project) => Ok(render_json(status::Ok, &project)),
        Err(err) => Ok(render_net_error(&err)),
    }
}


/// Return names of all the projects in the given origin
// This route is only available if jobsrv_enabled is true
pub fn project_list(req: &mut Request) -> IronResult<Response> {
    let mut projects_get = OriginProjectListGet::new();

    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    projects_get.set_origin(origin);

    match route_message::<OriginProjectListGet, OriginProjectList>(req, &projects_get) {
        Ok(projects) => Ok(render_json(status::Ok, &projects.get_names())),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Retrieve the most recent 50 jobs for a project.
// This route is only available if jobsrv_enabled is true
pub fn project_jobs(req: &mut Request) -> IronResult<Response> {
    let mut jobs_get = ProjectJobsGet::new();

    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    match get_param(req, "name") {
        Some(name) => jobs_get.set_name(format!("{}/{}", origin, name)),
        None => return Ok(Response::with(status::BadRequest)),
    }

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    match helpers::extract_pagination(req) {
        Ok((start, stop)) => {
            jobs_get.set_start(start as u64);
            jobs_get.set_stop(stop as u64);
        }
        Err(response) => return Ok(response),
    }
    match route_message::<ProjectJobsGet, ProjectJobsGetResponse>(req, &jobs_get) {
        Ok(response) => {
            let list: Vec<serde_json::Value> = response
                .get_jobs()
                .iter()
                .map(|job| {
                    if job.get_state() == JobState::Complete {
                        let channels =
                            helpers::channels_for_package_ident(req, &job.get_package_ident());
                        let platforms =
                            helpers::platforms_for_package_ident(req, &job.get_package_ident());
                        let mut job_json = serde_json::to_value(job).unwrap();

                        if channels.is_some() {
                            job_json["channels"] = json!(channels);
                        }

                        if platforms.is_some() {
                            job_json["platforms"] = json!(platforms);
                        }

                        job_json
                    } else {
                        serde_json::to_value(job).unwrap()
                    }
                })
                .collect();

            helpers::paginated_response(
                &list,
                response.get_count() as isize,
                response.get_start() as isize,
                response.get_stop() as isize,
            )
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn create_project_integration(req: &mut Request) -> IronResult<Response> {
    let params = match validate_params(req, &["origin", "name", "integration"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    let body = req.get::<bodyparser::Json>();
    match body {
        Ok(Some(_)) => (),
        Ok(None) => {
            warn!("create_project_integration: Empty body in request");
            return Ok(Response::with(status::BadRequest));
        }
        Err(e) => {
            warn!("create_project_integration, Error parsing body: {:?}", e);
            return Ok(Response::with(status::BadRequest));
        }
    };

    if !check_origin_access(req, &params["origin"]).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    // We know body exists and is valid, non-empty JSON, so we can unwrap safely
    let json_body = req.get::<bodyparser::Raw>().unwrap().unwrap();

    let mut opi = OriginProjectIntegration::new();
    opi.set_origin(params["origin"].clone());
    opi.set_name(params["name"].clone());
    opi.set_integration(params["integration"].clone());
    opi.set_integration_name(String::from(DEFAULT_PROJECT_INTEGRATION));
    opi.set_body(json_body);

    let mut request = OriginProjectIntegrationCreate::new();
    request.set_integration(opi);

    match route_message::<OriginProjectIntegrationCreate, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => {
            if err.get_code() == ErrCode::ENTITY_CONFLICT {
                warn!("Failed to create integration as it already exists");
                Ok(Response::with(status::Conflict))
            } else {
                error!("create_project_integration:1, err={:?}", err);
                Ok(Response::with(status::InternalServerError))
            }
        }
    }
}

// This route is only available if jobsrv_enabled is true
pub fn delete_project_integration(req: &mut Request) -> IronResult<Response> {
    let params = match validate_params(req, &["origin", "name", "integration"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    if !check_origin_access(req, &params["origin"]).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut request = OriginProjectIntegrationDelete::new();
    request.set_origin(params["origin"].clone());
    request.set_name(params["name"].clone());
    request.set_integration(params["integration"].clone());

    match route_message::<OriginProjectIntegrationDelete, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => {
            error!("delete_project_integration:1, err={:?}", err);
            Ok(Response::with(status::InternalServerError))
        }
    }
}

// This route is only available if jobsrv_enabled is true
pub fn get_project_integration(req: &mut Request) -> IronResult<Response> {
    let params = match validate_params(req, &["origin", "name", "integration"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    let mut opi = OriginProjectIntegration::new();
    opi.set_origin(params["origin"].clone());
    opi.set_name(params["name"].clone());
    opi.set_integration(params["integration"].clone());
    opi.set_integration_name(String::from(DEFAULT_PROJECT_INTEGRATION));

    let mut request = OriginProjectIntegrationGet::new();
    request.set_integration(opi);

    match route_message::<OriginProjectIntegrationGet, OriginProjectIntegration>(req, &request) {
        Ok(integration) => {
            let v: serde_json::Value = match serde_json::from_str(&integration.get_body()) {
                Ok(v) => v,
                Err(e) => {
                    debug!("Error parsing to JSON. e = {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "api:gpi:1")));
                }
            };
            Ok(render_json(status::Ok, &v))
        }
        Err(err) => match err.get_code() {
            ErrCode::ENTITY_NOT_FOUND => Ok(Response::with(status::NotFound)),
            _ => {
                error!(
                    "Unexpected error retrieving project integration, err={:?}",
                    err
                );
                Ok(Response::with((status::InternalServerError, "api:gpi:2")))
            }
        },
    }
}

// This route is only available if jobsrv_enabled is true
pub fn project_privacy_toggle(req: &mut Request) -> IronResult<Response> {
    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let name = match get_param(req, "name") {
        Some(n) => n,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let vis = match get_param(req, "visibility") {
        Some(v) => v,
        None => return Ok(Response::with(status::BadRequest)),
    };

    // users aren't allowed to set projects to hidden manually
    if vis.to_lowercase() == "hidden" {
        return Ok(Response::with(status::BadRequest));
    }

    let opv: OriginPackageVisibility = match vis.parse() {
        Ok(o) => o,
        Err(_) => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut project_get = OriginProjectGet::new();
    project_get.set_name(format!("{}/{}", origin, name));

    match route_message::<OriginProjectGet, OriginProject>(req, &project_get) {
        Ok(mut project) => {
            let real_visibility = transition_visibility(opv, project.get_visibility());
            let mut opu = OriginProjectUpdate::new();
            project.set_visibility(real_visibility);
            opu.set_project(project);

            match route_message::<OriginProjectUpdate, NetOk>(req, &opu) {
                Ok(_) => Ok(Response::with(status::NoContent)),
                Err(err) => Ok(render_net_error(&err)),
            }
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}


*/
