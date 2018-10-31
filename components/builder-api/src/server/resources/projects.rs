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
use std::env;

use actix_web::http::{self, Method, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Json, Path, Query};
use serde_json;

use protocol::jobsrv::*;
use protocol::originsrv;

use bldr_core;
use hab_core::package::{PackageIdent, Plan};

use db::models::origin::*;
use db::models::package::PackageVisibility;
use db::models::package::*;
use db::models::project_integration::*;
use db::models::projects::*;

use server::authorize::authorize_session;
use server::error::Error;
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers::{self, Pagination};
use server::AppState;

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectCreateReq {
    #[serde(default)]
    pub origin: String,
    #[serde(default)]
    pub plan_path: String,
    #[serde(default)]
    pub installation_id: u32,
    #[serde(default)]
    pub repo_id: u32,
    #[serde(default)]
    pub auto_build: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectUpdateReq {
    #[serde(default)]
    pub plan_path: String,
    #[serde(default)]
    pub installation_id: u32,
    #[serde(default)]
    pub repo_id: u32,
    #[serde(default)]
    pub auto_build: bool,
}

pub struct Projects;

impl Projects {
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route("/projects", Method::POST, create_project)
            .route("/projects/{origin}", Method::GET, get_projects)
            .route("/projects/{origin}/{name}", Method::GET, get_project)
            .route("/projects/{origin}/{name}", Method::PUT, update_project)
            .route("/projects/{origin}/{name}", Method::DELETE, delete_project)
            .route("/projects/{origin}/{name}/jobs", Method::GET, get_jobs)
            .route(
                "/projects/{origin}/{name}/integrations/{integration}/default",
                Method::GET,
                get_integration,
            ).route(
                "/projects/{origin}/{name}/integrations/{integration}/default",
                Method::PUT,
                create_integration,
            ).route(
                "/projects/{origin}/{name}/integrations/{integration}/default",
                Method::DELETE,
                delete_integration,
            ).route(
                "/projects/{origin}/{name}/{visibility}",
                Method::PATCH,
                toggle_privacy,
            )
    }
}

//
// Route handlers - these functions can return any Responder trait
//

// TODO: the project creation API needs to be simplified
fn create_project((req, body): (HttpRequest<AppState>, Json<ProjectCreateReq>)) -> HttpResponse {
    if (body.origin.len() <= 0) || (body.plan_path.len() <= 0) {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let account_id = match authorize_session(&req, Some(&body.origin)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let origin = match Origin::get(&body.origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => origin,
        Err(err) => return err.into(),
    };

    // Test hook - bypass the github dance
    if env::var_os("HAB_FUNC_TEST").is_some() {
        debug!("creating test project");

        let new_project = NewProject {
            owner_id: account_id as i64,
            origin_name: &origin.name,
            package_name: "testapp",
            plan_path: &body.plan_path,
            vcs_type: "git",
            vcs_data: "https://github.com/habitat-sh/testapp.git",
            install_id: body.installation_id as i64,
            visibility: &originsrv::OriginPackageVisibility::Public.to_string(),
            auto_build: body.auto_build,
        };

        match Project::create(&new_project, &*conn).map_err(Error::DieselError) {
            Ok(project) => return HttpResponse::Created().json(project),
            Err(err) => return err.into(),
        }
    };

    debug!(
                "GITHUB-CALL builder_api::server::handlers::project_create: Getting app_installation_token; repo_id={} installation_id={}",
                body.repo_id,
                body.installation_id
            );

    let token = match req
        .state()
        .github
        .app_installation_token(body.installation_id)
    {
        Ok(token) => token,
        Err(err) => {
            warn!("Error authenticating github app installation, {}", err);
            return HttpResponse::new(StatusCode::FORBIDDEN);
        }
    };

    let vcs_data = match req.state().github.repo(&token, body.repo_id) {
        Ok(Some(repo)) => repo.clone_url,
        Ok(None) => return HttpResponse::with_body(StatusCode::NOT_FOUND, "rg:pc:2"),
        Err(e) => {
            warn!("Error finding github repo. e = {:?}", e);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "rg:pc:1");
        }
    };

    let plan = match req
        .state()
        .github
        .contents(&token, body.repo_id, &body.plan_path)
    {
        Ok(Some(contents)) => match contents.decode() {
            Ok(bytes) => match Plan::from_bytes(bytes.as_slice()) {
                Ok(plan) => plan,
                Err(e) => {
                    debug!("Error matching Plan. e = {:?}", e);
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "rg:pc:3");
                }
            },
            Err(e) => {
                warn!("Base64 decode failure: {:?}", e);
                return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "rg:pc:4");
            }
        },
        Ok(None) => return HttpResponse::with_body(StatusCode::NOT_FOUND, "rg:pc:5"),
        Err(e) => {
            warn!("Error fetching contents from GH. e = {:?}", e);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "rg:pc:6");
        }
    };

    let new_project = NewProject {
        owner_id: account_id as i64,
        origin_name: &origin.name,
        package_name: plan.name.trim_matches('"'),
        plan_path: &body.plan_path,
        vcs_type: "git",
        vcs_data: &vcs_data,
        install_id: body.installation_id as i64,
        visibility: &origin.default_package_visibility.to_string(),
        auto_build: body.auto_build,
    };

    match Project::create(&new_project, &*conn).map_err(Error::DieselError) {
        Ok(project) => HttpResponse::Created().json(project),
        Err(err) => err.into(),
    }
}

fn get_project(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, name) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let project_get = format!("{}/{}", &origin, &name);

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Project::get(&project_get, &*conn).map_err(Error::DieselError) {
        Ok(project) => HttpResponse::Ok().json(project),
        Err(err) => err.into(),
    }
}

fn delete_project(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, name) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let project_delete = format!("{}/{}", &origin, &name);

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Project::delete(&project_delete, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn update_project((req, body): (HttpRequest<AppState>, Json<ProjectUpdateReq>)) -> HttpResponse {
    let (origin, name) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(id) => id,
        Err(err) => return err.into(),
    };

    if body.plan_path.len() <= 0 {
        return HttpResponse::with_body(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Missing value for field: `plan_path`",
        );
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // TODO (SA): We should not need to fetch the origin project here.
    // Simplify origin project update sproc to not require project id,
    // and to not need to pass in the origin id again
    let project_get = format!("{}/{}", &origin, &name);

    let project = match Project::get(&project_get, &*conn).map_err(Error::DieselError) {
        Ok(project) => project,
        Err(err) => return err.into(),
    };

    // Test hook - bypass the github dance
    if env::var_os("HAB_FUNC_TEST").is_some() {
        debug!("updating test project");

        let update_project = UpdateProject {
            id: project.id,
            origin_id: project.origin_id,
            owner_id: account_id as i64,
            package_name: "testapp",
            plan_path: &body.plan_path,
            vcs_type: "git",
            vcs_data: "https://github.com/habitat-sh/testapp.git",
            install_id: body.installation_id as i64,
            visibility: &originsrv::OriginPackageVisibility::Public.to_string(),
            auto_build: body.auto_build,
        };

        match Project::update(&update_project, &*conn).map_err(Error::DieselError) {
            Ok(_) => return HttpResponse::NoContent().finish(),
            Err(err) => return err.into(),
        }
    };

    debug!(
                "GITHUB-CALL builder_api::server::handlers::project_update: Getting app_installation_token; repo_id={} installation_id={}",
                body.repo_id,
                body.installation_id
            );

    let token = match req
        .state()
        .github
        .app_installation_token(body.installation_id)
    {
        Ok(token) => token,
        Err(err) => {
            debug!("Error authenticating github app installation, {}", err);
            return HttpResponse::new(StatusCode::FORBIDDEN);
        }
    };

    let vcs_data = match req.state().github.repo(&token, body.repo_id) {
        Ok(Some(repo)) => repo.clone_url,
        Ok(None) => return HttpResponse::with_body(StatusCode::NOT_FOUND, "rg:pu:2"),
        Err(e) => {
            warn!("Error finding GH repo. e = {:?}", e);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "rg:pu:1");
        }
    };

    let plan = match req
        .state()
        .github
        .contents(&token, body.repo_id, &body.plan_path)
    {
        Ok(Some(contents)) => match contents.decode() {
            Ok(bytes) => match Plan::from_bytes(bytes.as_slice()) {
                Ok(plan) => {
                    debug!("plan = {:?}", &plan);
                    if plan.name != name {
                        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "rg:pu:7");
                    }
                    plan
                }
                Err(e) => {
                    debug!("Error matching Plan. e = {:?}", e);
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "rg:pu:3");
                }
            },
            Err(e) => {
                debug!("Error decoding content from b64. e = {:?}", e);
                return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "rg:pu:4");
            }
        },
        Ok(None) => return HttpResponse::with_body(StatusCode::NOT_FOUND, "rg:pc:6"),
        Err(e) => {
            warn!("Erroring fetching contents from GH. e = {:?}", e);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, "rg:pu:5");
        }
    };

    let update_project = UpdateProject {
        id: project.id,
        owner_id: account_id as i64,
        origin_id: project.origin_id,
        package_name: &plan.name.trim_matches('"'),
        plan_path: &body.plan_path,
        vcs_type: "git",
        vcs_data: &vcs_data,
        install_id: body.installation_id as i64,
        visibility: &project.visibility,
        auto_build: body.auto_build,
    };

    match Project::update(&update_project, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn get_projects(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Project::list(&origin, &*conn) {
        Ok(projects) => {
            let names: Vec<String> = projects
                .iter()
                .map(|ref p| p.package_name.clone())
                .collect();
            HttpResponse::Ok().json(names)
        }
        Err(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn get_jobs((pagination, req): (Query<Pagination>, HttpRequest<AppState>)) -> HttpResponse {
    let (origin, name) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let mut jobs_get = ProjectJobsGet::new();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let (start, stop) = helpers::extract_pagination(&pagination);

    jobs_get.set_name(format!("{}/{}", origin, name));
    jobs_get.set_start(start as u64);
    jobs_get.set_stop(stop as u64);

    match route_message::<ProjectJobsGet, ProjectJobsGetResponse>(&req, &jobs_get) {
        Ok(response) => {
            let list: Vec<serde_json::Value> = response
                .get_jobs()
                .iter()
                .map(|job| {
                    if job.get_state() == JobState::Complete {
                        let channels =
                            helpers::channels_for_package_ident(&req, &job.get_package_ident());
                        let platforms =
                            helpers::platforms_for_package_ident(&req, &job.get_package_ident());
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
                }).collect();

            let body = helpers::package_results_json(
                &list,
                response.get_count() as isize,
                response.get_start() as isize,
                response.get_stop() as isize,
            );

            let mut response = if response.get_count() as isize > (response.get_stop() as isize + 1)
            {
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

fn create_integration((req, body): (HttpRequest<AppState>, String)) -> HttpResponse {
    let (origin, name, integration) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    if body.len() <= 0 {
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    let _: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(err) => {
            debug!("Error parsing project integration body, err={:?}", err);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let npi = NewProjectIntegration {
        origin: &origin,
        name: &name,
        integration: &integration,
        body: &body,
    };

    match ProjectIntegration::create(npi, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn delete_integration(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, name, integration) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match ProjectIntegration::delete(&origin, &name, &integration, &*conn)
        .map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => err.into(),
    }
}

fn get_integration(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, name, integration) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match ProjectIntegration::get(&origin, &name, &integration, &*conn).map_err(Error::DieselError)
    {
        Ok(integration) => match serde_json::from_str(&integration.body) {
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

fn toggle_privacy(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, name, visibility) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    // users aren't allowed to set projects to hidden manually
    if visibility.to_lowercase() == "hidden" {
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    let opv: originsrv::OriginPackageVisibility = match visibility.parse() {
        Ok(o) => o,
        Err(_) => return HttpResponse::new(StatusCode::BAD_REQUEST),
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let project_get = format!("{}/{}", &origin, &name);
    let project = match Project::get(&project_get, &*conn).map_err(Error::DieselError) {
        Ok(project) => project,
        Err(err) => return err.into(),
    };

    let project_visibility = project.visibility.parse().unwrap();

    let real_visibility = bldr_core::helpers::transition_visibility(&opv, &project_visibility);

    let package_name = project.package_name.clone();

    let update_project = UpdateProject {
        id: project.id,
        owner_id: project.owner_id,
        origin_id: project.origin_id,
        package_name: &package_name,
        plan_path: &project.plan_path,
        vcs_type: &project.vcs_type,
        vcs_data: &project.vcs_data,
        install_id: project.vcs_installation_id,
        visibility: &real_visibility.to_string(),
        auto_build: project.auto_build,
    };

    if let Err(err) = Project::update(&update_project, &*conn).map_err(Error::DieselError) {
        return err.into();
    }

    let ident = PackageIdent::new(project.origin_name, project.package_name, None, None);
    let pkgs =
        match Package::get_all(BuilderPackageIdent(ident), &*conn).map_err(Error::DieselError) {
            Ok(pkgs) => pkgs,
            Err(err) => return err.into(),
        };

    let mut map = HashMap::new();

    // TODO (SA): This needs to be refactored to all get done in a single transaction

    // For each row, store its id in our map, keyed on visibility
    for pkg in pkgs {
        let id = pkg.id;
        let pv: PackageVisibility = pkg.visibility;
        let vis: originsrv::OriginPackageVisibility = pv.into();
        let new_vis = bldr_core::helpers::transition_visibility(&project_visibility, &vis);
        map.entry(new_vis).or_insert(Vec::new()).push(id);
    }

    // Now do a bulk update for each different visibility
    for (vis, id_vector) in map.iter() {
        let pv = PackageVisibility::from(*vis);
        let upv = UpdatePackageVisibility {
            visibility: pv,
            ids: id_vector.clone(),
        };

        if let Err(err) = Package::update_visibility(upv, &*conn).map_err(Error::DieselError) {
            return err.into();
        };
    }

    HttpResponse::NoContent().finish()
}
