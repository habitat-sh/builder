// Copyright (c) 2016-2020 Chef Software Inc. and/or applicable contributors
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

use std::{collections::HashSet,
          fs::File,
          io::{BufReader,
               Write},
          iter::FromIterator,
          path::Path,
          str::FromStr,
          sync::Arc};

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
                          package::origin_packages_with_version_array,
                          project::origin_projects},
                 DbPool},
            diesel::{dsl::sql,
                     ExpressionMethods,
                     QueryDsl,
                     RunQueryDsl},
            error::{Error,
                    Result},
            package_info::PackageInfo};

use crate::package_ident_intern::PackageIdentIntern;

// Take a list of packages and return the set that is unbuildable
// Separate from the below DataStoreTrait to minimize leakage into the solver
pub trait Unbuildable: AsUnbuildable {
    fn filter_unbuildable(&self,
                          packages: &[PackageIdentIntern],
                          _target: PackageTarget)
                          -> Result<Vec<PackageIdentIntern>> {
        Ok(filter_unbuildable_static(&packages))
    }
}

// These two exist because trait upcasting isn't a thing in rust, so we have to do some magic
// instead. Thank you stack overflow https://stackoverflow.com/questions/28632968/why-doesnt-rust-support-trait-object-upcasting
pub trait AsUnbuildable {
    fn as_unbuildable(&self) -> &dyn Unbuildable;
}

impl<T: Unbuildable> AsUnbuildable for T {
    fn as_unbuildable(&self) -> &dyn Unbuildable { self }
}

// Capture the key database APIs and separate them from implementation to make testing with static
// data easier.
pub trait DataStoreTrait: Unbuildable {
    fn get_job_graph_packages(&self) -> Result<Vec<PackageInfo>>;

    fn get_job_graph_package(&self,
                             ident: &PackageIdent,
                             target: PackageTarget)
                             -> Result<PackageWithVersionArray>;

    fn get_origin_channel_latest(&self,
                                 origin: &str,
                                 channel: &str,
                                 target: PackageTarget)
                                 -> Result<Vec<PackageIdent>>;

    fn serialize(&self,
                 filename: &str,
                 origin: &str,
                 channel: &str,
                 target: PackageTarget)
                 -> Result<()>;
}

/// Simple serial
#[derive(Debug, Serialize, Deserialize)]
pub struct SerializedDatabase {
    origin:        String,
    channel:       String,
    target:        PackageTarget,
    packages:      Vec<PackageWithVersionArray>,
    base_packages: Vec<PackageIdent>,
}

impl SerializedDatabase {
    pub fn write_to_file(&self, filename: &str) -> Result<()> {
        let path = Path::new(filename);
        let mut file = File::create(&path).unwrap();

        let serialized = serde_json::to_string(&self).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        Ok(())
    }

    pub fn read_from_file(filename: &str) -> Result<SerializedDatabase> {
        let path = Path::new(filename);
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let u: SerializedDatabase = serde_json::from_reader(reader)?;
        Ok(u)
    }

    pub fn package_count(&self) -> usize { self.packages.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // cargo test simple_round_trip
    fn simple_round_trip() {
        let target = PackageTarget::from_str("x86_64-linux").unwrap();

        let sd = SerializedDatabase { origin: "foo".to_string(),
                                      channel: "bar".to_string(),
                                      target,
                                      packages: Vec::new(),
                                      base_packages: Vec::new() };
        // todo figure out tmpfile
        let filename = "testfile.json";
        sd.write_to_file(filename).unwrap();

        let new_sd = SerializedDatabase::read_from_file(filename);
        assert!(new_sd.is_ok());
        let new_sd = new_sd.unwrap();
        assert_eq!(new_sd.origin, "foo");
        assert_eq!(new_sd.channel, "bar");
        assert_eq!(new_sd.target, target);
    }
}

impl DataStoreTrait for SerializedDatabase {
    fn get_job_graph_packages(&self) -> Result<Vec<PackageInfo>> {
        let packages = self.packages
                           .iter()
                           .cloned()
                           .map(PackageInfo::from)
                           .collect();
        Ok(packages)
    }

    fn get_job_graph_package(&self,
                             ident: &PackageIdent,
                             target: PackageTarget)
                             -> Result<PackageWithVersionArray> {
        self.packages
            .iter()
            .find(|&x| &x.ident.0 == ident && x.target.0 == target)
            .ok_or(Error::DieselError(diesel::result::Error::NotFound))
            .map(|x| x.clone())
    }

    fn get_origin_channel_latest(&self,
                                 _origin: &str,
                                 _channel: &str,
                                 _target: PackageTarget)
                                 -> Result<Vec<PackageIdent>> {
        // Maybe check if we're giving the expected origin, channel and target...
        let base_packages = self.base_packages.to_vec();
        Ok(base_packages)
    }

    /// Serialized output
    fn serialize(&self,
                 filename: &str,
                 _origin: &str,
                 _channel: &str,
                 _target: PackageTarget)
                 -> Result<()> {
        // TODO check that origin, channel, target match and fail
        self.write_to_file(filename).unwrap();
        Ok(())
    }
}

impl Unbuildable for SerializedDatabase {
    fn filter_unbuildable(&self,
                          packages: &[PackageIdentIntern],
                          _target: PackageTarget)
                          -> Result<Vec<PackageIdentIntern>> {
        Ok(filter_unbuildable_static(&packages))
    }
}

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
}

impl Unbuildable for DataStore {
    fn filter_unbuildable(&self,
                          packages: &[PackageIdentIntern],
                          target: PackageTarget)
                          -> Result<Vec<PackageIdentIntern>> {
        let package_idents: Vec<BuilderPackageIdent> =
            packages.iter()
                    .map(|x| BuilderPackageIdent((*x).into()))
                    .collect();

        let conn = self.pool.get_conn()?;

        let query = origin_projects::table.select(origin_projects::name)
                                          .filter(origin_projects::auto_build.eq(true))
                                          .filter(origin_projects::target.eq(target.to_string()))
                                          .filter(origin_projects::name.eq_any(package_idents));

        // let debug = diesel::query_builder::debug_query::<diesel::pg::Pg, _>(&query);

        let result: Vec<BuilderPackageIdent> = query.get_results(&conn).unwrap();

        let input_packages = HashSet::<PackageIdentIntern>::from_iter(packages.iter().cloned());
        let buildable_idents =
            HashSet::<PackageIdentIntern>::from_iter(result.into_iter().map(|r| r.0.into()));

        let unbuildable = input_packages.difference(&buildable_idents);

        Ok(unbuildable.cloned().collect())
    }
}

impl DataStoreTrait for DataStore {
    fn get_job_graph_packages(&self) -> Result<Vec<PackageInfo>> {
        let mut packages = Vec::new();

        let conn = self.pool.get_conn()?;

        let rows = Package::get_all_latest(&conn).map_err(Error::DieselError)?;

        if rows.is_empty() {
            warn!("No packages found");
            return Ok(packages);
        }

        for package in rows {
            packages.push(PackageInfo::from(package));
        }

        Ok(packages)
    }

    fn get_job_graph_package(&self,
                             ident: &PackageIdent,
                             target: PackageTarget)
                             -> Result<PackageWithVersionArray> {
        let conn = self.pool.get_conn()?;

        let package = GetLatestPackage { ident:      BuilderPackageIdent(ident.clone()),
                                         target:     BuilderPackageTarget(target),
                                         visibility: PackageVisibility::all(), };

        debug!("get_job_graph_package fetching: {} {}", ident, target);

        let package = Package::get_latest(package, &conn).map_err(Error::DieselError)?;
        Ok(package)
    }

    fn get_origin_channel_latest(&self,
                                 origin: &str,
                                 channel: &str,
                                 target: PackageTarget)
                                 -> Result<Vec<PackageIdent>> {
        let conn = self.pool.get_conn()?;

        let query = origin_packages_with_version_array::table
            .inner_join(origin_channel_packages::table.inner_join(origin_channels::table))
            .select(origin_packages_with_version_array::ident)
            .distinct_on((
                origin_packages_with_version_array::name,
                origin_packages_with_version_array::target,
            ))
            .order(sql::<PackageWithVersionArray>(
                "origin_packages_with_version_array.name, \
        origin_packages_with_version_array.target, \
        string_to_array(origin_packages_with_version_array.version_array[1],'.')::\
        numeric[] desc, origin_packages_with_version_array.ident_array[4] desc",
            ))
            .filter(origin_channels::name.eq(&channel))
            .filter(origin_packages_with_version_array::origin.eq(&origin))
            .filter(origin_packages_with_version_array::target.eq(target.to_string()));

        let result: Vec<BuilderPackageIdent> = query.get_results(&conn).unwrap();
        let idents: Vec<PackageIdent> = result.into_iter().map(|r| r.0).collect();

        Ok(idents)
    }

    /// Serialized output
    fn serialize(&self,
                 filename: &str,
                 origin: &str,
                 channel: &str,
                 target: PackageTarget)
                 -> Result<()> {
        // When we start testing with more than one origin/target we will want filter these by
        // origin and target for sanity
        let package_infos = self.get_job_graph_packages()?;
        let packages = package_infos.iter()
                                    .map(|x| x.package.as_ref().unwrap().clone())
                                    .collect();
        let base_packages = self.get_origin_channel_latest(origin, channel, target)?;

        let sd = SerializedDatabase { origin: origin.to_string(),
                                      channel: channel.to_string(),
                                      target,
                                      packages,
                                      base_packages };

        sd.write_to_file(filename).unwrap();
        Ok(())
    }
}

// When we are working with the serialized dataset (mostly for tests)
// this is a list of things that are known unbuildable. This could
// be stored in the serialized data, but for now we're defining it statically
// here.
lazy_static! {
    static ref UNBUILDABLES: HashSet<PackageIdentIntern> = {
        let mut m = HashSet::new();
        let idents = vec![ident_intern!("core/hab"),
                          ident_intern!("core/hab-butterfly"),
                          ident_intern!("core/hab-sup"),
                          ident_intern!("core/hab-builder-admin"),
                          ident_intern!("core/hab-builder-api"),
                          ident_intern!("core/hab-builder-jobsrv"),
                          ident_intern!("core/hab-builder-router"),
                          ident_intern!("core/hab-builder-sessionsrv"),
                          ident_intern!("core/hab-builder-vault"),
                          ident_intern!("core/hab-depot"),
                          ident_intern!("core/hab-director"),
                          ident_intern!("core/hab-dynamic"),
                          ident_intern!("core/hab-eventsrv"),
                          ident_intern!("core/habitat-builder-web"),
                          ident_intern!("core/hab-launcher"),
                          ident_intern!("core/hab-pkg-export-helm"),
                          ident_intern!("core/hab-pkg-export-kubernetes"),
                          ident_intern!("core/hab-pkg-export-tar"),
                          ident_intern!("core/hab-pkg-export-docker"),
                          ident_intern!("core/hab-spider"),
                          ident_intern!("core/hab-sup-static"),
                          ident_intern!("core/builder-admin"),
                          ident_intern!("core/builder-admin-proxy"),
                          ident_intern!("core/builder-api"),
                          ident_intern!("core/builder-api-proxy"),
                          ident_intern!("core/builder-datastore"),
                          ident_intern!("core/builder-graph"),
                          ident_intern!("core/builder-jobsrv"),
                          ident_intern!("core/builder-originsrv"),
                          ident_intern!("core/builder-router"),
                          ident_intern!("core/builder-scheduler"),
                          ident_intern!("core/builder-sessionsrv"),
                          ident_intern!("core/builder-web"),
                          ident_intern!("core/nginx-builder-api"),
                          ident_intern!("core/bazel"),
                          ident_intern!("core/bison2"),
                          ident_intern!("core/clang5"),
                          ident_intern!("core/corretto"),
                          ident_intern!("core/corretto8"),
                          ident_intern!("core/corretto11"),
                          ident_intern!("core/geoip"),
                          ident_intern!("core/jre7"),
                          ident_intern!("core/jre8"),
                          ident_intern!("core/jre9"),
                          ident_intern!("core/jdk7"),
                          ident_intern!("core/jdk8"),
                          ident_intern!("core/jdk9"),
                          ident_intern!("core/llvm5"),
                          ident_intern!("core/mention-bot"),
                          ident_intern!("core/mono4"),
                          ident_intern!("core/php5"),
                          ident_intern!("core/rethinkdb"),
                          ident_intern!("core/ruby22"),
                          ident_intern!("core/ruby23"),
                          ident_intern!("core/scaffolding-chef"),
                          ident_intern!("core/server-jre"),
                          ident_intern!("core/stringencoders"),];
        m.extend(idents.iter());
        m
    };
}

fn filter_unbuildable_static(idents: &[PackageIdentIntern]) -> Vec<PackageIdentIntern> {
    idents.iter()
          .filter(|x| UNBUILDABLES.contains(x))
          .copied()
          .collect()
}
