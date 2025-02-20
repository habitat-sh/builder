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

use std::{collections::HashMap,
          str::FromStr};

use protobuf::RepeatedField;

use actix_web::{web::{self,
                      Path,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};

use crate::protocol::{jobsrv,
                      originsrv::OriginPackageIdent};

use crate::hab_core::package::PackageTarget;

use crate::db::models::{origin::*,
                        package::*};

use crate::server::{authorize::authorize_session,
                    error::{Error,
                            Result},
                    framework::middleware::route_message,
                    helpers::{self,
                              req_state,
                              Target}};

pub struct Jobs;

impl Jobs {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/rdeps/{origin}/{name}", web::get().to(get_rdeps))
           .route("/rdeps/{origin}/{name}/group",
                  web::get().to(get_rdeps_group));
    }
}
// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
async fn get_rdeps(req: HttpRequest,
                   path: Path<(String, String)>,
                   qtarget: Query<Target>)
                   -> HttpResponse {
    let (origin, name) = path.into_inner();

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => return Error::HabitatCore(err).into(),
            }
        }
        None => helpers::target_from_headers(&req),
    };

    let mut rdeps_get = jobsrv::JobGraphPackageReverseDependenciesGet::new();
    rdeps_get.set_origin(origin);
    rdeps_get.set_name(name);
    rdeps_get.set_target(target.to_string());

    match route_message::<jobsrv::JobGraphPackageReverseDependenciesGet,
                        jobsrv::JobGraphPackageReverseDependencies>(&req, &rdeps_get).await
    {
        Ok(rdeps) => {
            let filtered = match filtered_rdeps(&req, &rdeps) {
                Ok(f) => f,
                Err(err) => return err.into(),
            };
            HttpResponse::Ok().json(filtered)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

fn filtered_rdeps(req: &HttpRequest,
                  rdeps: &jobsrv::JobGraphPackageReverseDependencies)
                  -> Result<jobsrv::JobGraphPackageReverseDependencies> {
    let mut new_rdeps = jobsrv::JobGraphPackageReverseDependencies::new();
    new_rdeps.set_origin(rdeps.get_origin().to_string());
    new_rdeps.set_name(rdeps.get_name().to_string());

    let mut origin_map = HashMap::new();
    let mut short_deps = RepeatedField::new();

    for rdep in rdeps.get_rdeps() {
        let ident = OriginPackageIdent::from_str(rdep)?;
        let origin_name = ident.get_origin();
        let pv = if !origin_map.contains_key(origin_name) {
            let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;
            let origin = Origin::get(origin_name, &conn)?;
            origin_map.insert(origin_name.to_owned(),
                              origin.default_package_visibility.clone());
            origin.default_package_visibility
        } else {
            origin_map[origin_name].clone()
        };
        if pv != PackageVisibility::Public
           && authorize_session(req, Some(origin_name), Some(OriginMemberRole::Member)).is_err()
        {
            debug!("Skipping unauthorized non-public origin package: {}", rdep);
            continue; // Skip any unauthorized origin packages
        }

        short_deps.push(rdep.to_string())
    }

    new_rdeps.set_rdeps(short_deps);
    Ok(new_rdeps)
}

#[allow(clippy::needless_pass_by_value)]
async fn get_rdeps_group(req: HttpRequest,
                         path: Path<(String, String)>,
                         qtarget: Query<Target>)
                         -> HttpResponse {
    let (origin, name) = path.into_inner();

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => return Error::HabitatCore(err).into(),
            }
        }
        None => helpers::target_from_headers(&req),
    };

    let mut rdeps_get = jobsrv::JobGraphPackageReverseDependenciesGroupedGet::new();
    rdeps_get.set_origin(origin);
    rdeps_get.set_name(name);
    rdeps_get.set_target(target.to_string());

    match route_message::<jobsrv::JobGraphPackageReverseDependenciesGroupedGet,
                        jobsrv::JobGraphPackageReverseDependenciesGrouped>(&req, &rdeps_get).await
    {
        Ok(rdeps) => {
            let filtered = match filtered_group_rdeps(&req, &rdeps) {
                Ok(f) => f,
                Err(err) => return err.into(),
            };
            HttpResponse::Ok().json(filtered)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

fn filtered_group_rdeps(req: &HttpRequest,
                        rdeps: &jobsrv::JobGraphPackageReverseDependenciesGrouped)
                        -> Result<jobsrv::JobGraphPackageReverseDependenciesGrouped> {
    let mut new_rdeps = jobsrv::JobGraphPackageReverseDependenciesGrouped::new();
    new_rdeps.set_origin(rdeps.get_origin().to_string());
    new_rdeps.set_name(rdeps.get_name().to_string());

    let mut origin_map = HashMap::new();
    let mut new_groups = RepeatedField::new();

    for group in rdeps.get_rdeps() {
        let mut ident_list = Vec::new();
        for ident_str in group.get_idents() {
            let ident = OriginPackageIdent::from_str(ident_str)?;
            let origin_name = ident.get_origin();
            let pv = if !origin_map.contains_key(origin_name) {
                let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;
                let origin = Origin::get(origin_name, &conn)?;
                origin_map.insert(origin_name.to_owned(),
                                  origin.default_package_visibility.clone());
                origin.default_package_visibility
            } else {
                origin_map[origin_name].clone()
            };
            if pv != PackageVisibility::Public
               && authorize_session(req, Some(origin_name), None).is_err()
            {
                debug!("Skipping unauthorized non-public origin package: {}",
                       ident_str);
                continue; // Skip any unauthorized origin packages
            }
            ident_list.push(ident_str.to_owned())
        }

        let mut new_group = jobsrv::JobGraphPackageReverseDependencyGroup::new();
        new_group.set_group_id(group.get_group_id());
        new_group.set_idents(RepeatedField::from_vec(ident_list));
        new_groups.push(new_group)
    }

    new_rdeps.set_rdeps(new_groups);
    Ok(new_rdeps)
}
