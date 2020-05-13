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
                          package::origin_packages_with_version_array},
                 DbPool},
            diesel::{dsl::sql,
                     ExpressionMethods,
                     QueryDsl,
                     RunQueryDsl},
            error::{Error,
                    Result},
            util};

pub trait DataStoreTrait {
    fn get_job_graph_packages(&self) -> Result<Vec<PackageWithVersionArray>>;

    fn get_job_graph_package(&self,
                             ident: &PackageIdent,
                             target: PackageTarget)
                             -> Result<PackageWithVersionArray>;

    fn get_origin_channel_latest(&self,
                                 origin: &str,
                                 channel: &str,
                                 target: PackageTarget)
                                 -> Result<Vec<PackageIdent>>;
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

impl DataStoreTrait for DataStore {
    fn get_job_graph_packages(&self) -> Result<Vec<PackageWithVersionArray>> {
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

    fn get_job_graph_package(&self,
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
        // let query =
        // origin_packages_with_version_array::table.select(origin_packages_with_version_array::
        // ident) .filter(origin_packages_with_version_array::target.eq(target.
        // to_string()));
        //
        let result: Vec<BuilderPackageIdent> = query.get_results(&conn).unwrap();
        let idents: Vec<PackageIdent> = result.into_iter().map(|r| r.0).collect();

        Ok(idents)
    }
}

pub struct DummyDataStore {
    channel_latest: Vec<PackageIdent>,
    packages:       Vec<PackageWithVersionArray>,
}

impl DummyDataStore {
    pub fn new(filename: &str) -> Self {
        let packages: Vec<PackageWithVersionArray> = util::read_packages_json(filename);

        DummyDataStore { channel_latest: Vec::new(),
                         packages }
    }
}

impl DataStoreTrait for DummyDataStore {
    fn get_job_graph_packages(&self) -> Result<Vec<PackageWithVersionArray>> {
        let packages = self.packages.iter().cloned().collect();
        Ok(packages)
    }

    fn get_job_graph_package(&self,
                             ident: &PackageIdent,
                             target: PackageTarget)
                             -> Result<PackageWithVersionArray> {
        let package = self.packages[0].clone(); // not actually what we want
        Ok(package)
    }

    fn get_origin_channel_latest(&self,
                                 _origin: &str,
                                 _channel: &str,
                                 _target: PackageTarget)
                                 -> Result<Vec<PackageIdent>> {
        let gang_of_42 = "core/acl/2.2.53/20190115012136
        core/attr/2.4.48/20190115012129
        core/bash/4.4.19/20190115012619
        core/bash-completion/2.8/20190131162722
        core/bash-static/4.4.19/20190402151504
        core/binutils/2.31.1/20190115003743
        core/bison/3.0.5/20190115012441
        core/bison2/2.7.1/20190115161755
        core/bzip2/1.0.6/20190725144604
        core/bzip2-musl/1.0.6/20190115014608
        core/coreutils/8.30/20190115012313
        core/coreutils-static/8.30/20190115183917
        core/db/5.3.28/20190115012845
        core/dbus/1.13.8/20190201184338
        core/dejagnu/1.6.1/20190115014148
        core/diffutils/3.6/20190115013221
        core/expect/5.45.4/20190115014137
        core/file/5.34/20190115003731
        core/filebeat/7.2.0/20190626161648
        core/findutils/4.6.0/20190115013303
        core/gawk/4.2.1/20190115012752
        core/gcc/8.2.0/20190115004042
        core/gcc-libs/8.2.0/20190115011926
        core/gdbm/1.17/20190115012826
        core/gettext/0.19.8/20190115013412
        core/glibc/2.27/20190115002733
        core/gmp/6.1.2/20190115003943
        core/grep/3.1/20190115012541
        core/gzip/1.9/20190115013612
        core/iana-etc/2.30/20190115013006
        core/inetutils/1.9.4/20190115012922
        core/less/530/20190115013008
        core/lessmsi/1.6.1/20190131231730
        core/libcap/2.25/20190115012150
        core/libcap-ng/0.7.8/20190117153131
        core/libmpc/1.1.0/20190115004027
        core/linux-headers/4.3/20170513200956
        core/linux-headers-musl/3.12.6-6/20190115014537
        core/m4/1.4.18/20190115003920
        core/make/4.2.1/20190115013626
        core/mpfr/4.0.1/20190115004008
        core/ncurses/6.1/20190115012027
        core/ncurses5/6.1/20190116151445
        core/patch/2.7.6/20190115013636
        core/patchelf/0.9/20190115011946
        core/pcre/8.42/20190115012526
        core/perl/5.28.0/20190115013014
        core/procps-ng/3.3.15/20190115012258
        core/readline/7.0.3/20190115012607
        core/readline6/6.3.8/20190117175148
        core/sed/4.5/20190115012152
        core/tcl/8.6.8/20190115013933
        core/texinfo/6.5/20190115013702
        core/xz/5.2.4/20190115013348
        core/xz-musl/5.2.4/20190115014612
        core/zlib/1.2.8/20170513201911
        core/zlib-musl/1.2.8/20180310002650
        ";
        let idents: Vec<PackageIdent> =
            gang_of_42.lines()
                      .map(|x| PackageIdent::from_str(x.trim()).unwrap())
                      .collect();
        Ok(idents)
    }
}
