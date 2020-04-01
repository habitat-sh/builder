// Copyright (c) 2017-2020 Chef Software Inc. and/or applicable contributors
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

use std::{cell::{Ref,
                 RefCell},
          collections::HashMap,
          rc::Rc,
          str::FromStr};

use itertools::Itertools;

use regex::Regex;

use habitat_builder_db::models::package::PackageWithVersionArray;

// use habitat_builder_protocol as protocol;

use crate::{hab_core::{error as herror,
                       package::PackageIdent},
            util::*};

pub type PackageIndex = usize;

#[derive(Debug)]
pub struct PackageInfo {
    pub ident:   PackageIdent,
    // We may need to create the info record before we see the package data...
    pub package: Option<PackageWithVersionArray>,

    pub no_deps:    bool,
    pub plan_deps:  Vec<PackageIdent>,
    pub plan_bdeps: Vec<PackageIdent>,
}

impl PackageInfo {
    pub fn extract_plan_deps(&mut self, verbose: bool) {
        // Hoist to lazy static
        lazy_static! {
            // linux builds use backticks around dependency list, while windows doesn't,
            static ref NO_DEPS_RE: Regex =
                Regex::new(r"no (build|runtime) dependencies or undefined").unwrap();
            static ref GEN_DEP_RE: Regex =
                Regex::new(r"^\s*\* __(?P<dtype>Build)?\s*Dependencies__: `?(?P<deps>[^`]*)`?\s*$")
                .unwrap();
        }

        let package = self.package.as_ref().unwrap();
        let mut found_deps = false;
        let mut found_bdeps = false;

        // investigate RegexSet usage here instead of looping over lines
        for line in package.manifest.lines() {
            if let Some(cap) = GEN_DEP_RE.captures(line) {
                if verbose {
                    println!("{} matched line {}", package.ident.0, line);
                }
                let deplist = cap.name("deps").unwrap().as_str();
                if verbose {
                    println!("{} extracted deps {}", package.ident.0, deplist);
                }
                // Maybe match against regex 'no (build|runtime) dependencies or undefined'
                let mut deps_as_ident = if !deplist.contains("dependencies or undefined") {
                    let deps_conv: herror::Result<Vec<PackageIdent>> =
                        deplist.split_whitespace()
                               .map(PackageIdent::from_str)
                               .collect();

                    deps_conv.unwrap_or_else(|e| {
                                 println!("{} ill formed deps {:?}", package.ident.0, e);
                                 Vec::new()
                             })
                } else {
                    Vec::new()
                };

                let typeflag;
                if let Some(_deptype) = cap.name("dtype") {
                    typeflag = "B";
                    found_bdeps = true;
                    if verbose {
                        println!("{} {}: {:?}",
                                 package.ident.0,
                                 typeflag,
                                 join_idents(", ", &deps_as_ident))
                    };
                    self.plan_bdeps.append(&mut deps_as_ident);
                } else {
                    typeflag = "R";
                    found_deps = true;
                    if verbose {
                        println!("{} {}: {:?}",
                                 package.ident.0,
                                 typeflag,
                                 join_idents(", ", &deps_as_ident));
                    };
                    self.plan_deps.append(&mut deps_as_ident);
                }
            }

            // early out; manifests can be large...
            if found_deps && found_bdeps {
                break;
            }
        }

        if !(found_deps && found_bdeps) {
            // Not every package has deps. There are a few classes this falls into:
            // 1) True 'no deps' packages. core/cacerts is a good example
            // 2) User packages statically built using system dependencies. While not 'pure' habitat
            // packages, this is a supported use case and common in windows.
            // 3) Bad packages... There are a lot of packages that don't list deps in their
            // manifest, but have them in their plan file. Or don't list them in their plan file,
            // but use them some how.
            println!("{}: Partial or no deps found for package B: {} R: {}",
                     package.ident.0, found_bdeps, found_deps);
            self.no_deps = true;
        } else {
            self.no_deps = false;
        }
    }

    #[allow(dead_code)]
    // It might be useful comparing this with the deps extracted from
    // the manifest and alerting when there are differences, as it is
    // either a broken package or something wrong in the code.
    pub fn infer_plan_deps(&mut self) {
        let package = self.package.as_ref().unwrap();

        // It would be more correct to parse the manifest and use that to fill plan_deps/bdeps
        // That may belong inside PackageWithVersionArray
        // For now, just approximate with truncated ident
        for dep in &package.deps {
            self.plan_deps.push(short_ident(&dep.0, false));
        }
        for dep in &package.build_deps {
            self.plan_bdeps.push(short_ident(&dep.0, false));
        }
    }

    #[allow(dead_code)]
    pub fn write(&self) {
        println!("PackageIdent: {}, no_deps: {}", self.ident, self.no_deps);
        if let Some(package_data) = &self.package {
            println!("Target:\t{}", package_data.target.0);
            println!("Deps:\t{}",
                     package_data.deps.iter().format_with(", ", |x, f| f(&x.0)));
            println!("BDeps:\t{}",
                     package_data.build_deps
                                 .iter()
                                 .format_with(", ", |x, f| f(&x.0)));

            println!("Plan Deps:\t{}", join_idents(", ", &self.plan_deps));
            println!("Plan BDeps:\t{}", join_idents(", ", &self.plan_bdeps));
        }
    }
}

#[derive(Default)]
pub struct PackageTable {
    // This is the master data store; all packages live here.
    packages:    Vec<Rc<RefCell<PackageInfo>>>,
    // Maps package ident to position in packages vector above.
    package_map: HashMap<PackageIdent, PackageIndex>,
}

// impl<'a> IntoIter for PackageTable<'a>{
//     type Item = Ref<PackageInfo>;
//     type IntoIter = IntoIter<Ref<PackageInfo>>;

//     fn into_iter(mut self) -> IntoIter<'a Ref<PackageInfo>> {
//         self.packages.into_iter()
//     }
// }

impl PackageTable {
    pub fn new() -> Self { PackageTable::default() }

    pub fn count(&self) -> usize { self.packages.len() }

    pub fn insert_package(&mut self, package: &PackageWithVersionArray) -> PackageIndex {
        let pi = self.generate_id(&package.ident.0);
        let mut package_info = self.packages[pi].borrow_mut();
        package_info.package = Some(package.clone());
        package_info.extract_plan_deps(false);
        pi
    }

    pub fn generate_id<'a>(&'a mut self, ident: &PackageIdent) -> PackageIndex {
        if self.package_map.contains_key(&ident) {
            self.package_map[&ident]
        } else {
            let package_index = self.packages.len();
            let package_info = PackageInfo { ident:      ident.clone(),
                                             package:    None,
                                             no_deps:    false,
                                             plan_deps:  Vec::new(),
                                             plan_bdeps: Vec::new(), };

            self.packages.push(Rc::new(RefCell::new(package_info)));
            self.package_map.insert(ident.clone(), package_index);
            package_index
        }
    }

    pub fn build<T>(&mut self, packages: T) -> usize
        where T: Iterator<Item = PackageWithVersionArray>
    {
        for p in packages {
            self.insert_package(&p);
        }
        self.packages.len()
    }

    // Consider implementing Index as sugar here
    pub fn find(&self, ident: &PackageIdent) -> Option<Rc<RefCell<PackageInfo>>> {
        self.package_map
            .get(ident)
            .map(|index| self.packages[*index].clone())
    }

    pub fn find_index(&self, ident: &PackageIdent) -> Option<PackageIndex> {
        self.package_map.get(ident).map(|x| *x)
    }

    pub fn get(&self, index: PackageIndex) -> Option<Rc<RefCell<PackageInfo>>> {
        if index < self.packages.len() {
            Some(self.packages[index].clone())
        } else {
            None
        }
    }

    pub fn get_ident<'a>(&'a self, index: PackageIndex) -> Option<PackageIdent> {
        if index < self.packages.len() {
            Some(self.packages[index].borrow().ident.clone())
        } else {
            None
        }
    }

    pub fn values(&self) -> impl Iterator<Item = Rc<RefCell<PackageInfo>>> {
        self.packages.into_iter()
    }

    // Note we only write the inner PackageWithVersionArray info, and
    // plan on regenerating the rest
    pub fn write_json(&self, filename: &str, filter: Option<&str>) {
        let mut output: Vec<PackageWithVersionArray> = Vec::new();
        let mut keep = 0;
        let mut m = 0;
        for package_ref in &self.packages {
            if filter_match(&package_ref.borrow().ident, filter) {
                m += 1;
                if let Some(_p) = &package_ref.borrow().package {
                    keep += 1;
                    output.push(package_ref.borrow().package.as_ref().unwrap().clone())
                }
            }
        }
        debug!("Wrote {}/{}/{} K/M/T packages with filter {:?}",
               keep,
               m,
               self.packages.len(),
               filter);
        write_packages_json(output.into_iter(), filename)
    }

    pub fn read_json(&mut self, filename: &str) {
        let u = read_packages_json(filename);
        self.build(u.into_iter());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util;

    use tempfile::NamedTempFile;

    #[test]
    fn write_restore_packages() {
        let empty: [&str; 0] = [];
        let mut vec = Vec::new();

        let package1 = util::mk_package_with_versionarray("foo/bar/1/2",
                                                          "x86_64-linux",
                                                          &["foo/baz/1/2"],
                                                          &empty);
        vec.push(package1);

        let package2 = util::mk_package_with_versionarray("foo/baz/1/2",
                                                          "x86_64-linux",
                                                          &["foo/bar/1/2"],
                                                          &empty);
        vec.push(package2);

        let tmpfile = "/tmp/package_table_test.out";
        let mut table = PackageTable::new();
        table.build(vec.into_iter());

        table.write_packages_json(tmpfile, None);

        let mut table2 = PackageTable::new();
        table2.read_packages_json(tmpfile);
        assert_eq!(table2.count(), 2);
    }
}
