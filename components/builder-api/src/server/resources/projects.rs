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
use actix_web::{App, HttpRequest, HttpResponse, Json, Path, Query};
use serde_json;

use protocol::jobsrv::*;
use protocol::originsrv::*;

use bldr_core;
use hab_core::package::Plan;
use hab_net::{ErrCode, NetError, NetOk};

use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::{route_message, Authenticated};
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

#[derive(Deserialize)]
pub struct Pagination {
    range: isize,
}

const PAGINATION_RANGE_MAX: isize = 50;

pub struct Projects;

impl Projects {
    //
    // Internal - these functions should return Result<..>
    //

    fn do_create_project(
        req: HttpRequest<AppState>,
        body: Json<ProjectCreateReq>,
    ) -> Result<OriginProject> {
        let mut request = OriginProjectCreate::new();
        let mut project = OriginProject::new();
        let mut origin_get = OriginGet::new();

        helpers::check_origin_access(&req, &body.origin)?;
        let account_id = helpers::get_session_id(&req);

        debug!(
                "GITHUB-CALL builder_api::server::handlers::project_create: Getting app_installation_token; repo_id={} installation_id={}",
                body.repo_id,
                body.installation_id
            );

        let token = req
            .state()
            .github
            .app_installation_token(body.installation_id)?;

        origin_get.set_name(body.origin.clone());
        project.set_plan_path(body.plan_path.clone());
        project.set_vcs_type(String::from("git"));
        project.set_vcs_installation_id(body.installation_id);
        project.set_auto_build(body.auto_build);

        match req.state().github.repo(&token, body.repo_id) {
            Ok(Some(repo)) => project.set_vcs_data(repo.clone_url),
            Ok(None) => {
                return Err(Error::NetError(NetError::new(
                    ErrCode::ENTITY_NOT_FOUND,
                    "rg:pc:1",
                )))
            }
            Err(e) => {
                warn!("Error finding github repo. e = {:?}", e);
                return Err(Error::Github(e));
            }
        }

        let origin = route_message::<OriginGet, Origin>(&req, &origin_get)?;

        match req
            .state()
            .github
            .contents(&token, body.repo_id, &project.get_plan_path())
        {
            Ok(Some(contents)) => match contents.decode() {
                Ok(bytes) => match Plan::from_bytes(bytes.as_slice()) {
                    Ok(plan) => {
                        project.set_origin_name(String::from(origin.get_name()));
                        project.set_origin_id(origin.get_id());
                        project.set_package_name(String::from(plan.name.trim_matches('"')));
                    }
                    Err(e) => {
                        debug!("Error matching Plan. e = {:?}", e);
                        return Err(Error::NetError(NetError::new(
                            ErrCode::ENTITY_NOT_FOUND,
                            "rg:pc:3",
                        )));
                    }
                },
                Err(e) => {
                    warn!("Base64 decode failure: {:?}", e);
                    return Err(Error::NetError(NetError::new(
                        ErrCode::ENTITY_NOT_FOUND,
                        "rg:pc:4",
                    )));
                }
            },
            Ok(None) => {
                return Err(Error::NetError(NetError::new(
                    ErrCode::ENTITY_NOT_FOUND,
                    "rg:pc:5",
                )))
            }
            Err(e) => {
                warn!("Error fetching contents from GH. e = {:?}", e);
                return Err(Error::NetError(NetError::new(
                    ErrCode::ENTITY_NOT_FOUND,
                    "rg:pc:2",
                )));
            }
        }

        project.set_owner_id(account_id);
        project.set_visibility(origin.get_default_package_visibility());
        request.set_project(project);
        route_message::<OriginProjectCreate, OriginProject>(&req, &request)
    }

    fn do_update_project(
        req: HttpRequest<AppState>,
        body: Json<ProjectUpdateReq>,
        origin: String,
        name: String,
    ) -> Result<NetOk> {
        helpers::check_origin_access(&req, &origin)?;

        let session_id = helpers::get_session_id(&req);

        let mut project_get = OriginProjectGet::new();
        project_get.set_name(format!("{}/{}", &origin, &name));
        let mut project = route_message::<OriginProjectGet, OriginProject>(&req, &project_get)?;

        let mut request = OriginProjectUpdate::new();

        debug!(
                "GITHUB-CALL builder_api::server::handlers::project_update: Getting app_installation_token; repo_id={} installation_id={}",
                body.repo_id,
                body.installation_id
            );

        let token = req
            .state()
            .github
            .app_installation_token(body.installation_id)?;

        project.set_auto_build(body.auto_build);
        project.set_plan_path(body.plan_path.clone());
        project.set_vcs_installation_id(body.installation_id);
        match req.state().github.repo(&token, body.repo_id) {
            Ok(Some(repo)) => project.set_vcs_data(repo.clone_url),
            Ok(None) => {
                return Err(Error::NetError(NetError::new(
                    ErrCode::ENTITY_NOT_FOUND,
                    "rg:pu:2",
                )))
            }
            Err(e) => {
                warn!("Error finding GH repo. e = {:?}", e);
                return Err(Error::NetError(NetError::new(
                    ErrCode::ENTITY_NOT_FOUND,
                    "rg:pu:1",
                )));
            }
        }

        match req
            .state()
            .github
            .contents(&token, body.repo_id, &project.get_plan_path())
        {
            Ok(Some(contents)) => match contents.decode() {
                Ok(bytes) => match Plan::from_bytes(bytes.as_slice()) {
                    Ok(plan) => {
                        debug!("plan = {:?}", &plan);
                        if plan.name != name {
                            return Err(Error::NetError(NetError::new(
                                ErrCode::ENTITY_NOT_FOUND,
                                "rg:pu:7",
                            )));
                        }
                        project.set_origin_name(String::from(origin));
                        project.set_package_name(String::from(name));
                    }
                    Err(e) => {
                        debug!("Error matching Plan. e = {:?}", e);
                        return Err(Error::NetError(NetError::new(
                            ErrCode::ENTITY_NOT_FOUND,
                            "rg:pu:3",
                        )));
                    }
                },
                Err(e) => {
                    debug!("Error decoding content from b64. e = {:?}", e);
                    return Err(Error::NetError(NetError::new(
                        ErrCode::ENTITY_NOT_FOUND,
                        "rg:pu:4",
                    )));
                }
            },
            Ok(None) => {
                return Err(Error::NetError(NetError::new(
                    ErrCode::ENTITY_NOT_FOUND,
                    "rg:pc:6",
                )))
            }
            Err(e) => {
                warn!("Erroring fetching contents from GH. e = {:?}", e);
                return Err(Error::NetError(NetError::new(
                    ErrCode::ENTITY_NOT_FOUND,
                    "rg:pu:5",
                )));
            }
        }

        request.set_requestor_id(session_id);
        request.set_project(project);
        route_message::<OriginProjectUpdate, NetOk>(&req, &request)
    }

    //
    // Route handlers - these functions should return HttpResponse
    //

    fn create_project(
        (req, body): (HttpRequest<AppState>, Json<ProjectCreateReq>),
    ) -> HttpResponse {
        match Self::do_create_project(req, body) {
            Ok(project) => HttpResponse::Ok().json(project),
            Err(err) => err.into(),
        }
    }

    fn get_project(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, name) = Path::<(String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        let mut project_get = OriginProjectGet::new();
        project_get.set_name(format!("{}/{}", &origin, &name));

        if helpers::check_origin_access(req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        match route_message::<OriginProjectGet, OriginProject>(req, &project_get) {
            Ok(project) => HttpResponse::Ok().json(project),
            Err(err) => err.into(),
        }
    }

    fn delete_project(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, name) = Path::<(String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok
        let session_id = helpers::get_session_id(req);

        if helpers::check_origin_access(req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut project_del = OriginProjectDelete::new();
        project_del.set_name(format!("{}/{}", &origin, &name));
        project_del.set_requestor_id(session_id);

        match route_message::<OriginProjectDelete, NetOk>(req, &project_del) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    fn update_project(
        (req, body): (HttpRequest<AppState>, Json<ProjectUpdateReq>),
    ) -> HttpResponse {
        let (origin, name) = Path::<(String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        match Self::do_update_project(req, body, origin, name) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    fn get_projects(req: &HttpRequest<AppState>) -> HttpResponse {
        let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
        let mut projects_get = OriginProjectListGet::new();

        if helpers::check_origin_access(req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        projects_get.set_origin(origin);

        match route_message::<OriginProjectListGet, OriginProjectList>(req, &projects_get) {
            Ok(projects) => HttpResponse::Ok().json(projects),
            Err(err) => err.into(),
        }
    }

    fn get_jobs((pagination, req): (Query<Pagination>, HttpRequest<AppState>)) -> HttpResponse {
        let (origin, name) = Path::<(String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        let mut jobs_get = ProjectJobsGet::new();

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        jobs_get.set_start(pagination.range as u64);
        jobs_get.set_stop((pagination.range + PAGINATION_RANGE_MAX) as u64);

        match route_message::<ProjectJobsGet, ProjectJobsGetResponse>(&req, &jobs_get) {
            Ok(response) => {
                let list: Vec<serde_json::Value> = response
                    .get_jobs()
                    .iter()
                    .map(|job| {
                        if job.get_state() == JobState::Complete {
                            let channels =
                                helpers::channels_for_package_ident(&req, &job.get_package_ident());
                            let platforms = helpers::platforms_for_package_ident(
                                &req,
                                &job.get_package_ident(),
                            );
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

                let body = helpers::package_results_json(
                    &list,
                    response.get_count() as isize,
                    response.get_start() as isize,
                    response.get_stop() as isize,
                );

                let mut response =
                    if response.get_count() as isize > (response.get_stop() as isize + 1) {
                        HttpResponse::PartialContent()
                    } else {
                        HttpResponse::Ok()
                    };

                response
                    .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                    .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                    .body(body)
            }
            Err(err) => err.into(),
        }
    }

    pub fn create_integration((req, body): (HttpRequest<AppState>, String)) -> HttpResponse {
        let (origin, name, integration) = Path::<(String, String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut opi = OriginProjectIntegration::new();
        opi.set_origin(origin.clone());
        opi.set_name(name.clone());
        opi.set_integration(integration.clone());
        opi.set_integration_name(String::from(DEFAULT_PROJECT_INTEGRATION));
        opi.set_body(body.clone());

        let mut request = OriginProjectIntegrationCreate::new();
        request.set_integration(opi);

        match route_message::<OriginProjectIntegrationCreate, NetOk>(&req, &request) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    pub fn delete_integration(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, name, integration) = Path::<(String, String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut request = OriginProjectIntegrationDelete::new();
        request.set_origin(origin.clone());
        request.set_name(name.clone());
        request.set_integration(integration.clone());

        match route_message::<OriginProjectIntegrationDelete, NetOk>(req, &request) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(err) => err.into(),
        }
    }

    pub fn get_integration(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, name, integration) = Path::<(String, String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let mut opi = OriginProjectIntegration::new();
        opi.set_origin(origin.clone());
        opi.set_name(name.clone());
        opi.set_integration(integration.clone());
        opi.set_integration_name(String::from(DEFAULT_PROJECT_INTEGRATION));

        let mut request = OriginProjectIntegrationGet::new();
        request.set_integration(opi);

        match route_message::<OriginProjectIntegrationGet, OriginProjectIntegration>(req, &request)
        {
            Ok(integration) => match serde_json::from_str(&integration.get_body()) {
                Ok(v) => {
                    let json_value: serde_json::Value = v;
                    HttpResponse::Ok().json(json_value)
                }
                Err(e) => {
                    debug!("Error parsing to JSON. e = {:?}", e);
                    Error::SerdeJson(e).into()
                }
            },
            Err(err) => err.into(),
        }
    }

    pub fn toggle_privacy(req: &HttpRequest<AppState>) -> HttpResponse {
        let (origin, name, visibility) = Path::<(String, String, String)>::extract(&req)
            .unwrap()
            .into_inner(); // Unwrap Ok

        if helpers::check_origin_access(&req, &origin).is_err() {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        // users aren't allowed to set projects to hidden manually
        if visibility.to_lowercase() == "hidden" {
            return HttpResponse::new(StatusCode::UNAUTHORIZED);
        }

        let opv: OriginPackageVisibility = match visibility.parse() {
            Ok(o) => o,
            Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
        };

        let mut project_get = OriginProjectGet::new();
        project_get.set_name(format!("{}/{}", origin, name));

        match route_message::<OriginProjectGet, OriginProject>(req, &project_get) {
            Ok(mut project) => {
                let real_visibility =
                    bldr_core::helpers::transition_visibility(opv, project.get_visibility());
                let mut opu = OriginProjectUpdate::new();
                project.set_visibility(real_visibility);
                opu.set_project(project);

                match route_message::<OriginProjectUpdate, NetOk>(req, &opu) {
                    Ok(_) => HttpResponse::NoContent().finish(),
                    Err(err) => err.into(),
                }
            }
            Err(err) => err.into(),
        }
    }

    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.resource("/projects", |r| {
            r.middleware(Authenticated);
            r.method(http::Method::POST).with(Self::create_project);
        }).resource("/projects/{origin}/{name}", |r| {
                r.middleware(Authenticated);
                r.get().f(Self::get_project);
            })
            .resource("/projects/{origin}", |r| {
                r.middleware(Authenticated);
                r.get().f(Self::get_projects);
            })
            .resource("/projects/{origin}/{name}/jobs", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::GET).with(Self::get_jobs);
            })
            .resource("/projects/{origin}/{name}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::PUT).with(Self::update_project);
            })
            .resource("/projects/{origin}/{name}", |r| {
                r.middleware(Authenticated);
                r.delete().f(Self::delete_project);
            })
            .resource(
                "/projects/{origin}/{name}/integrations/{integration}/default",
                |r| {
                    r.middleware(Authenticated);
                    r.get().f(Self::get_integration);
                },
            )
            .resource(
                "/projects/{origin}/{name}/integrations/{integration}/default",
                |r| {
                    r.middleware(Authenticated);
                    r.method(http::Method::PUT).with(Self::create_integration);
                },
            )
            .resource(
                "/projects/{origin}/{name}/integrations/{integration}/default",
                |r| {
                    r.middleware(Authenticated);
                    r.delete().f(Self::delete_integration);
                },
            )
            .resource("/projects/{origin}/{name}/{visibility}", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::PATCH).f(Self::toggle_privacy);
            })
    }
}
