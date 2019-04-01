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

use std::{str::FromStr,
          sync::Arc};

use postgres;
use protobuf::{self,
               RepeatedField};

use crate::{config::Config,
            db::pool::Pool,
            error::{Error,
                    Result},
            protocol::originsrv};

// DataStore inherits Send + Sync by virtue of having only one member, the pool itself.
#[derive(Debug, Clone)]
pub struct DataStore {
    pool: Pool,
}

// Sample connection_url: "postgresql://hab@127.0.0.1/builder"

impl DataStore {
    /// Create a new DataStore.
    ///
    /// * Can fail if the pool cannot be created
    /// * Blocks creation of the datastore on the existince of the pool; might wait indefinetly.
    pub fn new(config: &Config) -> Self {
        let pool = Pool::new(&config.datastore);
        DataStore { pool }
    }

    /// Create a new DataStore from a pre-existing pool; useful for testing the database.
    pub fn from_pool(pool: Pool, _: Arc<String>) -> Result<DataStore> { Ok(DataStore { pool }) }

    /// Setup the datastore.
    ///
    /// This includes all the schema and data migrations, along with stored procedures for data
    /// access.
    pub fn setup(&self) -> Result<()> { Ok(()) }

    pub fn get_job_graph_packages(&self) -> Result<RepeatedField<originsrv::OriginPackage>> {
        let mut packages = RepeatedField::new();

        let conn = self.pool.get()?;

        let rows = &conn.query("SELECT * FROM get_graph_packages_v1()", &[])
                        .map_err(Error::JobGraphPackagesGet)?;

        if rows.is_empty() {
            warn!("No packages found");
            return Ok(packages);
        }

        for row in rows {
            let package = self.row_to_origin_package(&row)?;
            packages.push(package);
        }

        Ok(packages)
    }

    pub fn get_job_graph_package(&self, ident: &str) -> Result<originsrv::OriginPackage> {
        let conn = self.pool.get()?;

        let rows = &conn.query("SELECT * FROM get_graph_package_v1($1)", &[&ident])
                        .map_err(Error::JobGraphPackagesGet)?;

        if rows.is_empty() {
            error!("No package found");
            return Err(Error::UnknownJobGraphPackage);
        }

        assert!(rows.len() == 1);
        let package = self.row_to_origin_package(&rows.get(0))?;
        Ok(package)
    }

    fn row_to_origin_package(&self, row: &postgres::rows::Row) -> Result<originsrv::OriginPackage> {
        let mut package = originsrv::OriginPackage::new();
        let id: i64 = row.get("id");
        package.set_id(id as u64);
        package.set_origin(row.get("origin"));
        let owner_id: i64 = row.get("owner_id");
        package.set_owner_id(owner_id as u64);
        let ident: String = row.get("ident");
        package.set_ident(originsrv::OriginPackageIdent::from_str(ident.as_str()).unwrap());
        package.set_checksum(row.get("checksum"));
        package.set_manifest(row.get("manifest"));
        package.set_config(row.get("config"));
        package.set_target(row.get("target"));
        let expose: String = row.get("exposes");
        let mut exposes: Vec<u32> = Vec::new();
        for ex in expose.split(':') {
            if let Ok(e) = ex.parse::<u32>() {
                exposes.push(e)
            }
        }

        package.set_exposes(exposes);
        package.set_deps(Self::dep_to_idents(row.get("deps")));
        package.set_tdeps(Self::dep_to_idents(row.get("tdeps")));
        package.set_build_deps(Self::dep_to_idents(row.get("build_deps")));
        package.set_build_tdeps(Self::dep_to_idents(row.get("build_tdeps")));

        // let pv: String = row.get("visibility");
        // TED removing for now to kill the FromString in originsrv
        // This should get converted to PackageVisibility in the future
        // let pv2: originsrv::OriginPackageVisibility = pv.parse().unwrap();
        // package.set_visibility(pv2);

        Ok(package)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn dep_to_idents(column: String) -> protobuf::RepeatedField<originsrv::OriginPackageIdent> {
        let mut idents = protobuf::RepeatedField::new();
        for ident in column.split(':') {
            if !ident.is_empty() {
                idents.push(originsrv::OriginPackageIdent::from_str(ident).unwrap());
            }
        }
        idents
    }
}
