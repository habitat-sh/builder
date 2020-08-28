// Copyright (c) 2019 Chef Software Inc. and/or applicable contributors
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

use actix_web::{body::Body,
                http::StatusCode,
                web::{self,
                      Data,
                      Json,
                      Path,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};

use builder_core::Error::PackageSettingDeleteError;

use crate::db::models::{origin::*,
                        package::*,
                        settings::*};
use diesel::PgConnection;

use crate::server::{authorize::authorize_session,
                    error::{Error,
                            Result},
                    helpers::req_state,
                    AppState};

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateOriginPackageSettingsReq {
    #[serde(default)]
    pub visibility: String,
}

pub struct Settings;

impl Settings {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/settings/{origin}/{name}",
                  web::post().to(create_origin_package_settings))
           .route("/settings/{origin}/{name}",
                  web::get().to(get_origin_package_settings))
           .route("/settings/{origin}/{name}",
                  web::put().to(update_origin_package_settings))
           .route("/settings/{origin}/{name}",
                  web::delete().to(delete_origin_package_settings));
    }
}

// get_origin_package_settings
#[allow(clippy::needless_pass_by_value)]
fn get_origin_package_settings(req: HttpRequest, path: Path<(String, String)>) -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let conn = match req_state(&req).db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let get_ops = &GetOriginPackageSettings { origin: &origin,
                                              name:   &pkg, };

    match OriginPackageSettings::get(&get_ops, &*conn).map_err(Error::DieselError) {
        Ok(ops) => HttpResponse::Ok().json(ops),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// create_origin_package_settings
#[allow(clippy::needless_pass_by_value)]
fn create_origin_package_settings(req: HttpRequest,
                                  path: Path<(String, String)>,
                                  state: Data<AppState>)
                                  -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Validate that the origin exists before attempting to create pkg settings
    let (oname, pv) = match Origin::get(&origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => (origin.name, origin.default_package_visibility),
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    match OriginPackageSettings::create(
        &NewOriginPackageSettings {
            origin: &oname,
            name: &pkg,
            visibility: &pv,
            owner_id: account_id as i64,
        },
        &*conn).map_err(Error::DieselError) {
        Ok(ops) => {
            HttpResponse::Created().json(ops)
        },
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn update_origin_package_settings(req: HttpRequest,
                                  path: Path<(String, String)>,
                                  body: Json<UpdateOriginPackageSettingsReq>)
                                  -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    if body.0.visibility.is_empty() {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let conn = match req_state(&req).db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let pv: PackageVisibility = match body.0.visibility.parse() {
        Ok(o) => o,
        Err(err) => {
            debug!("{:?}", err);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    match OriginPackageSettings::update(&UpdateOriginPackageSettings { origin:     &origin,
                                                                       name:       &pkg,
                                                                       visibility: &pv,
                                                                       owner_id:   account_id
                                                                                   as i64, },
                                        &*conn).map_err(Error::DieselError)
    {
        Ok(ups) => HttpResponse::Ok().json(ups),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_origin_package_settings(req: HttpRequest, path: Path<(String, String)>) -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let conn = match req_state(&req).db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Prior to passing the deletion request to the backend, we validate
    // that the user has already cleaned up any existing packages.
    match package_settings_delete_preflight(&origin, &pkg, &*conn) {
        Ok(_) => {
            // Delete the package setting
            match OriginPackageSettings::delete(&DeleteOriginPackageSettings { origin:   &origin,
                                                                               name:     &pkg,
                                                                               owner_id: account_id
                                                                                         as i64, },
                                                &*conn).map_err(Error::DieselError)
            {
                Ok(_) => HttpResponse::new(StatusCode::NO_CONTENT),
                Err(err) => {
                    debug!("{}", err);
                    err.into()
                }
            }
        }
        Err(err) => {
            debug!("Origin preflight determined that {} is not deletable, err = {}!",
                   origin, err);
            // Here we want to enrich the http response with a sanitized error
            // by returning a 409 with a helpful message in the body.
            HttpResponse::with_body(StatusCode::CONFLICT, Body::from_message(format!("{}", err)))
        }
    }
}

fn package_settings_delete_preflight(origin: &str, pkg: &str, conn: &PgConnection) -> Result<()> {
    match OriginPackageSettings::count_packages_for_origin_package(&origin, &pkg, &*conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} packages remaining for setting {}/{}. Must be zero.",
                              count, origin, pkg);
            return Err(Error::BuilderCore(PackageSettingDeleteError(err)));
        }
        Err(e) => return Err(Error::DieselError(e)),
    };
    Ok(())
}

// This function is deprecated.
#[allow(clippy::needless_pass_by_value)]
pub fn do_toggle_privacy(req: HttpRequest,
                         path: Path<(String, String, String)>,
                         state: Data<AppState>)
                         -> HttpResponse {
    let (origin, name, visibility) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return err.into();
    }

    // users aren't allowed to set projects to hidden manually
    if visibility.to_lowercase() == "hidden" {
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    let pv: PackageVisibility = match visibility.parse() {
        Ok(o) => o,
        Err(err) => {
            debug!("{:?}", err);
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let ops = match OriginPackageSettings::get(&GetOriginPackageSettings { origin: &origin,
                                                                           name:   &name, },
                                               &*conn).map_err(Error::DieselError)
    {
        Ok(pkg_settings) => pkg_settings,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let update_project = UpdateOriginPackageSettings { origin:     &ops.origin,
                                                       name:       &ops.name,
                                                       visibility: &pv,
                                                       owner_id:   ops.owner_id, };

    if let Err(err) =
        OriginPackageSettings::update(&update_project, &*conn).map_err(Error::DieselError)
    {
        debug!("{}", err);
        return err.into();
    }

    HttpResponse::NoContent().finish()
}
