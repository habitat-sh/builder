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

use std::{collections::HashMap,
          str::FromStr};

use itertools::Itertools;

use regex::Regex;

use habitat_builder_db::models::package::PackageWithVersionArray;

// use habitat_builder_protocol as protocol;

use crate::{hab_core::{error as herror,
                       package::{PackageIdent,
                                 PackageTarget}},
            package_ident_intern::PackageIdentIntern,
            util::*};

pub type PackageIndex = usize;

// This struct belongs elsewhere in habitat or builder, but not today, still factoring it out.
//
pub struct PackageMetadata {
    pub plan_deps:  Vec<PackageIdent>,
    pub plan_bdeps: Vec<PackageIdent>,
}

lazy_static! {
    // linux builds use backticks around dependency list, while windows doesn't,
    static ref NO_DEPS_RE: Regex =
        Regex::new(r"no (build|runtime) dependencies or undefined").unwrap();
    static ref GEN_DEP_RE: Regex =
        Regex::new(r"^\s*\* __(?P<dtype>Build)?\s*Dependencies__: `?(?P<deps>[^`]*)`?\s*$")
        .unwrap();
}
impl PackageMetadata {
    pub fn extract_from_manifest(manifest: &str) -> PackageMetadata {
        let mut found_deps = false;
        let mut found_bdeps = false;

        let mut plan_deps = Vec::new();
        let mut plan_bdeps = Vec::new();

        // investigate RegexSet usage here instead of looping over lines
        for line in manifest.lines() {
            if let Some(cap) = GEN_DEP_RE.captures(line) {
                let deplist = cap.name("deps").unwrap().as_str();
                // Maybe match against regex 'no (build|runtime) dependencies or undefined'
                let mut deps_as_ident = if !deplist.contains("dependencies or undefined") {
                    let deps_conv: herror::Result<Vec<PackageIdent>> =
                        deplist.split_whitespace()
                               .map(PackageIdent::from_str)
                               .collect();

                    deps_conv.unwrap_or_else(|_e| {
                                 // this may be worth noting as a trace event; ill formed deps
                                 Vec::new()
                             })
                } else {
                    Vec::new()
                };

                if let Some(_deptype) = cap.name("dtype") {
                    found_bdeps = true;
                    plan_bdeps.append(&mut deps_as_ident);
                } else {
                    found_deps = true;
                    plan_deps.append(&mut deps_as_ident);
                }
            }
            // early out; manifests can be large...
            if found_deps && found_bdeps {
                break;
            }
        }

        PackageMetadata { plan_bdeps,
                          plan_deps }
    }
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub ident:   PackageIdent,
    pub target:  PackageTarget,
    // We may need to create the info record before we see the package data...
    // Also, this should not be necessary for computing build order.
    // Maybe make this private, or otherwise shield this from general usage
    pub package: Option<PackageWithVersionArray>,

    pub no_deps:      bool,
    pub plan_deps:    Vec<PackageIdent>,
    pub plan_bdeps:   Vec<PackageIdent>,
    pub strong_bdeps: Vec<PackageIdentIntern>,
}

impl PackageInfo {
    pub fn extract_plan_deps(&mut self, _verbose: bool) {
        let package = self.package.as_ref().unwrap();

        let metadata = PackageMetadata::extract_from_manifest(&package.manifest);

        self.plan_deps = metadata.plan_deps.to_vec();
        self.plan_bdeps = metadata.plan_bdeps.to_vec();

        self.strong_bdeps = strong_build_deps(self);
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
            println!("Plan BDeps:\t{}", join_idents(", ", &self.strong_bdeps));
        }
    }

    // mostly used in test code to create test records
    pub fn mk(ident: &str,
              target: &str,
              deps: &[&str],
              bdeps: &[&str],
              sdeps: &[&str])
              -> PackageInfo {
        PackageInfo { ident:        PackageIdent::from_str(ident).unwrap(),
                      target:       PackageTarget::from_str(target).unwrap(),
                      package:      None,
                      no_deps:      false,
                      plan_deps:    deps.iter()
                                        .map(|x| PackageIdent::from_str(x).unwrap())
                                        .collect(),
                      plan_bdeps:   bdeps.iter()
                                         .map(|x| PackageIdent::from_str(x).unwrap())
                                         .collect(),
                      strong_bdeps: sdeps.iter()
                                         .map(|x| PackageIdentIntern::from_str(x).unwrap())
                                         .collect(), }
    }
}

impl From<PackageWithVersionArray> for PackageInfo {
    fn from(package: PackageWithVersionArray) -> Self {
        let mut package_info = PackageInfo { ident:        package.ident.0.clone(),
                                             target:       package.target.0,
                                             package:      Some(package),
                                             no_deps:      false,
                                             plan_deps:    Vec::new(),
                                             plan_bdeps:   Vec::new(),
                                             strong_bdeps: Vec::new(), };
        package_info.extract_plan_deps(false);
        package_info
    }
}

// Strong build edges represent a missing semantic in plan deps.
//
// There are cases where we have separate plans and packages for
// things that are tightly linked. Gcc and gcc-libs are an example. If
// a package uses both, it can't mix and match what it picks up; it must
// use gcc and gcc-libs of the same vintage or it won't build. A build
// time dep isn't strict enough to represent that right now, as we ignore
// them when building cycles. So we add a strong build dep as an extra
// ordering primitive when computing the build order inside a cycle.
//
// Some of this might be unnecesary if we took better account of the build edges
// in a cycle when ordering things. However, to do this we must break the cycle by
// ignoring some of the build edges, and initial attempts to do that selectively
// proved to be complicated and error prone.
//
// It might be worth revisiting this; a possible path would be to experiment on various
// heuristics around identifying the least critical build edge in the cycle and removing them
// but there still remains no clear path.
//
// Alternately we could add a concept of even more strongly coupled plans, that are built
// as a unit in some fashion, either explicitly ('a multiplan' could exist that produces multiple
// packages). This may also be related to the pattern we see where packages have deps that specify
// versions, as that seems to be an attempt to have packages coupled in lockstep.
//
// Another option would to handle this more implicitly by having strong build edges create a sub
// cluster inside the cluster that is scheduled as a unit.
//
lazy_static! {
    static ref STRONG_BUILD_DEPS: HashMap<PackageIdentIntern, Vec<PackageIdentIntern>> = {
        let mut m: HashMap<PackageIdentIntern, Vec<PackageIdentIntern>> = HashMap::new();
        m.insert(ident_intern!("core/gcc-libs"),
                 vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/happy"), vec![ident_intern!("core/ghc")]);
        m.insert(ident_intern!("core/ncurses"),
                 vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/findutils"),
                 vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/pkg-config"),
                 vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/xz"), vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/make"), vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/flex"), vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/diffutils"),
                 vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/attr"), vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/bzip2"), vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/bison"), vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/gawk"), vec![ident_intern!("core/gcc")]);
        m.insert(ident_intern!("core/gdbm"), vec![ident_intern!("core/gcc")]);
        m
    };
}

fn strong_build_deps(package: &PackageInfo) -> Vec<PackageIdentIntern> {
    STRONG_BUILD_DEPS.get(&PackageIdentIntern::from(&package.ident).short_ident())
                     .cloned()
                     .unwrap_or_default()
}
