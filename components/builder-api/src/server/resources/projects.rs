// Copyright (c) 2018-2021 Chef Software Inc. and/or applicable contributors
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

use actix_web::{http::StatusCode,
                web::{self,
                      Data,
                      Path,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};

use crate::db::models::{origin::*,
                        project_integration::*,
                        projects::*};

use crate::server::{authorize::authorize_session,
                    error::Error,
                    helpers::{self,
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
    #[serde(default)]
    pub name:            String,
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
        cfg.route("/projects/{origin}", web::get().to(get_projects))
           .route("/projects/{origin}/{name}", web::get().to(get_project))
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

#[allow(clippy::needless_pass_by_value)]
async fn get_project(req: HttpRequest,
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

    match Project::get(&project, &target, &conn).map_err(Error::DieselError) {
        Ok(project) => HttpResponse::Ok().json(project),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_projects(req: HttpRequest, path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Project::list(&origin, &conn) {
        Ok(projects) => {
            let names: Vec<String> = projects.iter().map(|p| p.package_name.clone()).collect();
            HttpResponse::Ok().json(names)
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn create_integration(req: HttpRequest,
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

    match ProjectIntegration::create(&npi, &conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn delete_integration(req: HttpRequest,
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

    match ProjectIntegration::delete(&origin, &name, &integration, &conn)
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
async fn get_integration(req: HttpRequest,
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

    match ProjectIntegration::get(&origin, &name, &integration, &conn).map_err(Error::DieselError) {
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
async fn toggle_privacy(req: HttpRequest,
                        path: Path<(String, String, String)>,
                        state: Data<AppState>)
                        -> HttpResponse {
    do_toggle_privacy(req, path, state)
}
