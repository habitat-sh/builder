// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

use std::sync::Arc;

use crate::hab_core::package::{PackageIdent,
                               PackageTarget};

use crate::db::models::package::{BuilderPackageIdent,
                                 BuilderPackageTarget,
                                 PackageWithVersionArray};

use crate::{config::Config,
            db::{models::package::{GetLatestPackage,
                                   Package,
                                   PackageVisibility},
                 schema::{channel::{origin_channel_packages,
                                    origin_channels},
                          package::origin_packages},
                 DbPool},
            diesel::{ExpressionMethods,
                     QueryDsl,
                     RunQueryDsl},
            error::{Error,
                    Result}};
// DataStore inherits Send + Sync by virtue of having only one member, the pool itself.
#[derive(Clone)]
pub struct DataStore {
    pool: DbPool,
}

// Sample connection_url: "postgresql://hab@127.0.0.1/builder"

impl DataStore {
    /// Create a new DataStore.
    ///
    /// * Can fail if the pool cannot be created
    /// * Blocks creation of the datastore on the existince of the pool; might wait indefinetly.
    pub fn new(config: &Config) -> Self {
        let pool = DbPool::new(&config.datastore);
        DataStore { pool }
    }

    /// Create a new DataStore from a pre-existing pool; useful for testing the database.
    pub fn from_pool(pool: DbPool, _: Arc<String>) -> Result<DataStore> { Ok(DataStore { pool }) }

    /// Setup the datastore.
    ///
    /// This includes all the schema and data migrations, along with stored procedures for data
    /// access.
    pub fn setup(&self) -> Result<()> { Ok(()) }

    pub fn get_job_graph_packages(&self) -> Result<Vec<PackageWithVersionArray>> {
        let mut packages = Vec::new();

        let conn = self.pool.get_conn()?;

        let rows = Package::get_all_latest(&conn).map_err(Error::DieselError)?;

        if rows.is_empty() {
            warn!("No packages found");
            return Ok(packages);
        }

        for package in rows {
            packages.push(package);
        }

        Ok(packages)
    }

    pub fn get_job_graph_package(&self,
                                 ident: &PackageIdent,
                                 target: PackageTarget)
                                 -> Result<PackageWithVersionArray> {
        let conn = self.pool.get_conn()?;

        let package = GetLatestPackage { ident:      BuilderPackageIdent(ident.clone()),
                                         target:     BuilderPackageTarget(target),
                                         visibility: PackageVisibility::all(), };

        println!("Package fetching: {:?}", package);

        let package = Package::get_latest(package, &conn).map_err(Error::DieselError)?;
        Ok(package)
    }

    pub fn get_origin_channel_latest(&self,
                                     origin: &str,
                                     channel: &str,
                                     target: PackageTarget)
                                     -> Result<Vec<PackageIdent>> {
        let conn = self.pool.get_conn()?;

        let query =
            origin_packages::table.inner_join(origin_channel_packages::table.inner_join(origin_channels::table))
                                  .distinct_on((origin_packages::name, origin_packages::target))
                                  .select(origin_packages::ident)
                                  .filter(origin_channels::name.eq(&channel))
                                  .filter(origin_packages::origin.eq(&origin))
                                  .filter(origin_packages::target.eq(target.to_string()));

        let result: Vec<BuilderPackageIdent> = query.get_results(&conn).unwrap();
        let idents = result.into_iter().map(|r| r.0).collect();
        Ok(idents)
    }
}
