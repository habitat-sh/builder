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

use actix_web::{body::BoxBody,
                http::StatusCode,
                web::{self,
                      Data,
                      Json,
                      Path,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};

use builder_core::Error::PackageSettingDeleteError;

use crate::{db::models::{origin::*,
                         package::*,
                         settings::*},
            server::{authorize::authorize_session,
                     error::{Error,
                             Result},
                     helpers::req_state,
                     AppState}};

use bytes::Bytes;
use diesel::PgConnection;

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
async fn get_origin_package_settings(req: HttpRequest,
                                     path: Path<(String, String)>)
                                     -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let mut conn = match req_state(&req).db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let get_ops = &GetOriginPackageSettings { origin: &origin,
                                              name:   &pkg, };

    match OriginPackageSettings::get(get_ops, &mut *conn).map_err(Error::DieselError) {
        Ok(ops) => HttpResponse::Ok().json(ops),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// create_origin_package_settings
#[allow(clippy::needless_pass_by_value)]
async fn create_origin_package_settings(req: HttpRequest,
                                        path: Path<(String, String)>,
                                        state: Data<AppState>)
                                        -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Validate that the origin exists before attempting to create pkg settings
    let (oname, pv) = match Origin::get(&origin, &mut *conn).map_err(Error::DieselError) {
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
        &mut *conn,
    )
    .map_err(Error::DieselError)
    {
        Ok(ops) => HttpResponse::Created().json(ops),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn update_origin_package_settings(req: HttpRequest,
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
        let body = Bytes::from_static(b"Missing required package visibility");
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
    }

    let mut conn = match req_state(&req).db.get_conn().map_err(Error::DbError) {
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
                                        &mut *conn).map_err(Error::DieselError)
    {
        Ok(ups) => HttpResponse::Ok().json(ups),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn delete_origin_package_settings(req: HttpRequest,
                                        path: Path<(String, String)>)
                                        -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let mut conn = match req_state(&req).db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Prior to passing the deletion request to the backend, we validate
    // that the user has already cleaned up any existing packages.
    match package_settings_delete_preflight(&origin, &pkg, &mut *conn) {
        Ok(_) => {
            // Delete the package setting
            match OriginPackageSettings::delete(&DeleteOriginPackageSettings { origin:   &origin,
                                                                               name:     &pkg,
                                                                               owner_id: account_id
                                                                                         as i64, },
                                                &mut *conn).map_err(Error::DieselError)
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
            let body = Bytes::from(format!("{}", err).into_bytes());
            let body = BoxBody::new(body);
            HttpResponse::with_body(StatusCode::CONFLICT, body)
        }
    }
}

fn package_settings_delete_preflight(origin: &str,
                                     pkg: &str,
                                     conn: &mut PgConnection)
                                     -> Result<()> {
    match OriginPackageSettings::count_packages_for_origin_package(origin, pkg, &mut *conn) {
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
