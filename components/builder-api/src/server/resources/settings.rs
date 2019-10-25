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

use actix_web::{http::StatusCode,
                web::{self,
                      Json,
                      Path,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};

use crate::db::models::{origin::*,
                        package::*,
                        settings::*};

use crate::server::{authorize::authorize_session,
                    error::Error,
                    helpers::req_state};

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
                  web::put().to(update_origin_package_settings));
    }
}

// get_origin_package_settings
fn get_origin_package_settings(req: HttpRequest, path: Path<(String, String)>) -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let _account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match req_state(&req).db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let get_ops = &GetOriginPackageSettings { origin, name: pkg };

    match OriginPackageSettings::get(&get_ops, &*conn).map_err(Error::DieselError) {
        Ok(ops) => HttpResponse::Ok().json(ops),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// create_origin_package_settings
fn create_origin_package_settings(req: HttpRequest, path: Path<(String, String)>) -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match req_state(&req).db.get_conn().map_err(Error::DbError) {
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
            origin: oname,
            name: pkg,
            visibility: pv,
            owner_id: account_id as i64,
        },
        &*conn).map_err(Error::DieselError) {
        Ok(ops) => HttpResponse::Created().json(ops),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

fn update_origin_package_settings(req: HttpRequest,
                                  path: Path<(String, String)>,
                                  body: Json<UpdateOriginPackageSettingsReq>)
                                  -> HttpResponse {
    let (origin, pkg) = path.into_inner();

    let account_id = match authorize_session(&req, Some(&origin)) {
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

    match OriginPackageSettings::update(&UpdateOriginPackageSettings { origin,
                                                                       name: pkg,
                                                                       visibility: pv,
                                                                       owner_id: account_id
                                                                                 as i64 },
                                        &*conn).map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::NoContent().into(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}
