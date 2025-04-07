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

use actix_web::{web::{self,
                      Path,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};

use crate::protocol::originsrv::OriginPackageIdent;

use crate::db::models::{origin::*,
                        package::*};

use crate::server::{authorize::authorize_session,
                    error::{Error,
                            Result},
                    helpers::{req_state,
                              Target}};

use super::reverse_dependencies::{self,
                                  ReverseDependencies};

pub struct Jobs;

impl Jobs {
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/rdeps/{origin}/{name}", web::get().to(get_rdeps));
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_rdeps(req: HttpRequest,
                   path: Path<(String, String)>,
                   qtarget: Query<Target>)
                   -> HttpResponse {
    let (origin, name) = path.into_inner();

    let target: String = qtarget.target
                                .clone()
                                .unwrap_or_else(|| "x86_64-linux".to_string());

    let connection = req_state(&req).db
                                    .get_conn()
                                    .map_err(Error::DbError)
                                    .unwrap();

    match reverse_dependencies::get_rdeps(&connection, &origin, &name, &target).await {
        Ok(reverse_dependencies) => {
            debug!("BEFORE FILTERING: reverse_dependencies: {:?}",
                   reverse_dependencies);
            let filtered = match filtered_rdeps(&req, &reverse_dependencies) {
                Ok(f) => f,
                Err(err) => return err.into(),
            };
            debug!("AFTER FILTERING: reverse_dependencies: {:?}",
                   reverse_dependencies);
            HttpResponse::Ok().json(filtered)
        }
        Err(err) => {
            debug!("{}", err);
            HttpResponse::InternalServerError().json(err.to_string())
        }
    }
}

fn filtered_rdeps(req: &HttpRequest,
                  reverse_dependencies: &ReverseDependencies)
                  -> Result<ReverseDependencies> {
    let mut origin_map = HashMap::new();
    let mut new_dependents: Vec<String> = Vec::new();
    let mut filtered_rdeps = ReverseDependencies { origin: reverse_dependencies.origin.clone(),
                                                   name:   reverse_dependencies.name.clone(),
                                                   rdeps:  Vec::new(), };

    for rdep in reverse_dependencies.rdeps.iter() {
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
            debug!("Skipping unauthorized non-public origin package: {origin_name}");
            continue; // Skip any unauthorized origin packages
        }

        new_dependents.push(rdep.clone())
    }

    filtered_rdeps.rdeps = new_dependents;
    Ok(filtered_rdeps)
}
