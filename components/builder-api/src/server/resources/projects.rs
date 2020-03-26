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

use std::{env,
          str::FromStr};

use actix_web::{http::{self,
                       StatusCode},
                web::{self,
                      Data,
                      Json,
                      Path,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};
use serde_json;

use crate::protocol::jobsrv;

use crate::hab_core::package::{PackageTarget,
                               Plan};

use crate::db::models::{jobs::*,
                        origin::*,
                        project_integration::*,
                        projects::*};

use crate::server::{authorize::authorize_session,
                    error::Error,
                    framework::headers,
                    helpers::{self,
                              Pagination,
                              Target},
                    resources::settings::do_toggle_privacy,
                    AppState};

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectCreateReq {
    #[serde(default)]
    pub origin:          String,
    #[serde(default)]
    pub plan_path:       String,
    #[serde(default = "default_target")]
    pub target:          String,
    #[serde(default)]
    pub installation_id: u32,
    #[serde(default)]
    pub repo_id:         u32,
    #[serde(default)]
    pub auto_build:      bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectUpdateReq {
    #[serde(default)]
    pub plan_path:       String,
    #[serde(default = "default_target")]
    pub target:          String,
    #[serde(default)]
    pub installation_id: u32,
    #[serde(default)]
    pub repo_id:         u32,
    #[serde(default)]
    pub auto_build:      bool,
}

fn default_target() -> String { "x86_64-linux".to_string() }

pub struct Projects;

impl Projects {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/projects", web::post().to(create_project))
           .route("/projects/{origin}", web::get().to(get_projects))
           .route("/projects/{origin}/{name}", web::get().to(get_project))
           .route("/projects/{origin}/{name}", web::put().to(update_project))
           .route("/projects/{origin}/{name}",
                  web::delete().to(delete_project))
           .route("/projects/{origin}/{name}/jobs", web::get().to(get_jobs))
           .route("/projects/{origin}/{name}/integrations/{integration}/default",
                  web::get().to(get_integration))
           .route("/projects/{origin}/{name}/integrations/{integration}/default",
                  web::put().to(create_integration))
           .route("/projects/{origin}/{name}/integrations/{integration}/default",
                  web::delete().to(delete_integration))
           .route("/projects/{origin}/{name}/{visibility}",
                  web::patch().to(toggle_privacy));
    }
}

// Route handlers - these functions can return any Responder trait
//

// TODO: the project creation API needs to be simplified
#[allow(clippy::needless_pass_by_value)]
async fn create_project(req: HttpRequest,
                        body: Json<ProjectCreateReq>,
                        state: Data<AppState>)
                        -> HttpResponse {
    if body.origin.is_empty() || body.plan_path.is_empty() {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let account_id =
        match authorize_session(&req, Some(&body.origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Do we really need this? Maybe to validate the origin exists?
    let origin = match Origin::get(&body.origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => origin,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let target = match PackageTarget::from_str(&body.target) {
        Ok(t) => t,
        Err(err) => {
            debug!("Invalid target requested: err = {:?}", err);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    // Test hook - bypass the github dance
    if env::var_os("HAB_FUNC_TEST").is_some() {
        let new_project =
            NewProject { owner_id:            account_id as i64,
                         origin:              &origin.name,
                         package_name:        "testapp",
                         name:                &format!("{}/{}", &origin.name, "testapp"),
                         plan_path:           &body.plan_path,
                         target:              &target,
                         vcs_type:            "git",
                         vcs_data:            "https://github.com/habitat-sh/testapp.git",
                         vcs_installation_id: Some(i64::from(body.installation_id)),
                         auto_build:          body.auto_build, };

        match Project::create(&new_project, &*conn).map_err(Error::DieselError) {
            Ok(project) => return HttpResponse::Created().json(project),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        }
    };

    let token = match state.github
                           .app_installation_token(body.installation_id)
                           .await
    {
        Ok(token) => token,
        Err(err) => {
            warn!("Error authenticating github app installation, {}", err);
            return HttpResponse::new(StatusCode::FORBIDDEN);
        }
    };

    let vcs_data = match state.github.repo(&token, body.repo_id).await {
        Ok(Some(repo)) => repo.clone_url,
        Ok(None) => return HttpResponse::new(StatusCode::NOT_FOUND),
        Err(e) => {
            debug!("Error finding github repo. e = {:?}", e);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    let plan = match state.github
                          .contents(&token, body.repo_id, &body.plan_path)
                          .await
    {
        Ok(Some(contents)) => {
            match contents.decode() {
                Ok(bytes) => {
                    match Plan::from_bytes(bytes.as_slice()) {
                        Ok(plan) => plan,
                        Err(e) => {
                            debug!("Error matching Plan. e = {:?}", e);
                            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                        }
                    }
                }
                Err(e) => {
                    debug!("Base64 decode failure: {:?}", e);
                    return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                }
            }
        }
        Ok(None) => return HttpResponse::new(StatusCode::NOT_FOUND),
        Err(e) => {
            debug!("Error fetching contents from GH. e = {:?}", e);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    let package_name = plan.name.trim_matches('"');
    let new_project = NewProject { owner_id: account_id as i64,
                                   origin: &origin.name,
                                   package_name,
                                   name: &format!("{}/{}", &origin.name, package_name),
                                   plan_path: &body.plan_path,
                                   target: &target,
                                   vcs_type: "git",
                                   vcs_data: &vcs_data,
                                   vcs_installation_id: Some(i64::from(body.installation_id)),
                                   auto_build: body.auto_build };

    match Project::create(&new_project, &*conn).map_err(Error::DieselError) {
        Ok(project) => HttpResponse::Created().json(project),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_project(req: HttpRequest,
               path: Path<(String, String)>,
               qtarget: Query<Target>,
               state: Data<AppState>)
               -> HttpResponse {
    let (origin, name) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let target = helpers::extract_target(&qtarget);
    let project = format!("{}/{}", &origin, &name);

    match Project::get(&project, &target, &*conn).map_err(Error::DieselError) {
        Ok(project) => HttpResponse::Ok().json(project),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_project(req: HttpRequest,
                  path: Path<(String, String)>,
                  qtarget: Query<Target>,
                  state: Data<AppState>)
                  -> HttpResponse {
    let (origin, name) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let target = helpers::extract_target(&qtarget);
    let project_delete = format!("{}/{}", &origin, &name);

    match Project::delete(&project_delete, &target, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn update_project(req: HttpRequest,
                        path: Path<(String, String)>,
                        body: Json<ProjectUpdateReq>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let (origin, name) = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    if body.plan_path.is_empty() {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let target = match PackageTarget::from_str(&body.target) {
        Ok(t) => t,
        Err(err) => {
            debug!("Invalid target requested, err = {:?}", err);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    // TODO (SA): We should not need to fetch the origin project here.
    // Simplify origin project update sproc to not require project id,
    // and to not need to pass in the origin id again
    let project_get = format!("{}/{}", &origin, &name);

    let project = match Project::get(&project_get, &target, &*conn).map_err(Error::DieselError) {
        Ok(project) => project,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    // Test hook - bypass the github dance
    if env::var_os("HAB_FUNC_TEST").is_some() {
        let update_project =
            UpdateProject { id:                  project.id,
                            origin:              &project.origin,
                            owner_id:            account_id as i64,
                            package_name:        "testapp",
                            plan_path:           &body.plan_path,
                            target:              &target,
                            vcs_type:            "git",
                            vcs_data:            "https://github.com/habitat-sh/testapp.git",
                            vcs_installation_id: Some(i64::from(body.installation_id)),
                            auto_build:          body.auto_build, };

        match Project::update(&update_project, &*conn).map_err(Error::DieselError) {
            Ok(_) => return HttpResponse::NoContent().finish(),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        }
    };

    let token = match state.github
                           .app_installation_token(body.installation_id)
                           .await
    {
        Ok(token) => token,
        Err(err) => {
            debug!("Error authenticating github app installation, {}", err);
            return HttpResponse::new(StatusCode::FORBIDDEN);
        }
    };

    let vcs_data = match state.github.repo(&token, body.repo_id).await {
        Ok(Some(repo)) => repo.clone_url,
        Ok(None) => return HttpResponse::new(StatusCode::NOT_FOUND),
        Err(e) => {
            debug!("Error finding GH repo. e = {:?}", e);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    let plan = match state.github
                          .contents(&token, body.repo_id, &body.plan_path)
                          .await
    {
        Ok(Some(contents)) => {
            match contents.decode() {
                Ok(bytes) => {
                    match Plan::from_bytes(bytes.as_slice()) {
                        Ok(plan) => {
                            debug!("plan = {:?}", &plan);
                            if plan.name != name {
                                return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                            }
                            plan
                        }
                        Err(e) => {
                            debug!("Error matching Plan. e = {:?}", e);
                            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                        }
                    }
                }
                Err(e) => {
                    debug!("Error decoding content from b64. e = {:?}", e);
                    return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
                }
            }
        }
        Ok(None) => return HttpResponse::new(StatusCode::NOT_FOUND),
        Err(e) => {
            warn!("Erroring fetching contents from GH. e = {:?}", e);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    let update_project = UpdateProject { id:                  project.id,
                                         owner_id:            account_id as i64,
                                         origin:              &project.origin,
                                         package_name:        &plan.name.trim_matches('"'),
                                         plan_path:           &body.plan_path,
                                         target:              &target,
                                         vcs_type:            "git",
                                         vcs_data:            &vcs_data,
                                         vcs_installation_id: Some(i64::from(body.installation_id)),
                                         auto_build:          body.auto_build, };

    match Project::update(&update_project, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_projects(req: HttpRequest, path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Project::list(&origin, &*conn) {
        Ok(projects) => {
            let names: Vec<String> = projects.iter()
                                             .map(|ref p| p.package_name.clone())
                                             .collect();
            HttpResponse::Ok().json(names)
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_jobs(req: HttpRequest,
            path: Path<(String, String)>,
            pagination: Query<Pagination>,
            state: Data<AppState>)
            -> HttpResponse {
    let (origin, name) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (page, per_page) = helpers::extract_pagination_in_pages(&pagination);
    assert!(page >= 1);

    let lpr = ListProjectJobs { name:  format!("{}/{}", origin, name),
                                page:  page as i64,
                                limit: per_page as i64, };

    match Job::list(lpr, &*conn).map_err(Error::DieselError) {
        Ok((jobs, total_count)) => {
            let start = (page - 1) * per_page;
            let stop = match jobs.len() {
                0 => per_page - 1,
                len => (start + (len as isize) - 1),
            };

            let list: Vec<serde_json::Value> = jobs.into_iter()
                                                   .map(|job| {
                                                       let protojob: jobsrv::Job = job.into();
                                                       serde_json::to_value(protojob).unwrap()
                                                   })
                                                   .collect();

            let body = helpers::package_results_json(&list,
                                                     total_count as isize,
                                                     start as isize,
                                                     stop as isize);

            let mut response = if total_count as isize > (stop + 1) {
                HttpResponse::PartialContent()
            } else {
                HttpResponse::Ok()
            };

            response.header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                    .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                    .body(body)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_integration(req: HttpRequest,
                      path: Path<(String, String, String)>,
                      body: String,
                      state: Data<AppState>)
                      -> HttpResponse {
    let (origin, name, integration) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return err.into();
    }

    if body.is_empty() {
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    let _: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(err) => {
            debug!("Error parsing project integration body, err={:?}", err);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let npi = NewProjectIntegration { origin:      &origin,
                                      name:        &name,
                                      integration: &integration,
                                      body:        &body, };

    match ProjectIntegration::create(&npi, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_integration(req: HttpRequest,
                      path: Path<(String, String, String)>,
                      state: Data<AppState>)
                      -> HttpResponse {
    let (origin, name, integration) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match ProjectIntegration::delete(&origin, &name, &integration, &*conn)
        .map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_integration(req: HttpRequest,
                   path: Path<(String, String, String)>,
                   state: Data<AppState>)
                   -> HttpResponse {
    let (origin, name, integration) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match ProjectIntegration::get(&origin, &name, &integration, &*conn).map_err(Error::DieselError)
    {
        Ok(integration) => {
            match serde_json::from_str(&integration.body) {
                Ok(v) => {
                    let json_value: serde_json::Value = v;
                    HttpResponse::Ok().json(json_value)
                }
                Err(e) => {
                    debug!("Error parsing to JSON. e = {:?}", e);
                    Error::SerdeJson(e).into()
                }
            }
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// This function is deprecated. Ultimately it should be removed as a route.
// In the meantime we pass this off to a function in the settings module
// For real though. This behavior is available via routes in settings
// and this should just be disabled.
#[allow(clippy::needless_pass_by_value)]
fn toggle_privacy(req: HttpRequest,
                  path: Path<(String, String, String)>,
                  state: Data<AppState>)
                  -> HttpResponse {
    do_toggle_privacy(req, path, state)
}
