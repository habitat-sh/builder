// Copyright (c) 2020 Chef Software Inc. and/or applicable contributors
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

use petgraph::{algo::tarjan_scc,
               graph::NodeIndex,
               Direction,
               Graph};

use std::collections::{HashMap,
                       HashSet,
                       VecDeque};

use std::{cmp,
          fs::File,
          io::prelude::*,
          path::Path};

use crate::hab_core::package::PackageIdent;

use crate::{ident_graph::IdentGraph,
            util::*};

type IdentIndex = usize;

pub struct PackageBuild {
    ident: PackageIdent,
    bdeps: Vec<PackageIdent>,
    rdeps: Vec<PackageIdent>,
}

impl<Value> IdentGraph<Value> where Value: Default + Copy
{
    // Compute a build ordering
    //
    // Inputs:
    //
    // * Set of base packages to build with (most likely stable channel, but as long as they're
    //   consisitent it's ok
    // * Universe of packages to build (core minus some unbuildables)
    // * Kernel of packages 'modified'
    // * Graph of package dependencies
    //
    // Process
    // 1) Take kernel of packages, and recursively expand it over reverse build/runtime deps
    //    Filter that expansion by the universe set
    //
    // 2) Compute ordering of expanded set using SCC and RT edges inside SCC
    // 3) Initialize 'latest' table using base set
    // 4) Walk ordering rebuilding packages
    //    For each package
    //    a) Resolve deps using latest,
    //    b) create new package with special name, record it in package table
    //    c) then update latest with new package
    //
    // 5) Take new latest table, walk graph to find actually used packages.

    pub fn compute_build(&self,
                         origin: &str,
                         base_set: &Vec<PackageIdent>,
                         touched: &Vec<PackageIdent>,
                         converge_count: usize)
                         -> Vec<PackageBuild> {
        let rebuild_set = self.compute_rebuild_set(touched, origin);

        let build_order = self.compute_build_order(rebuild_set);

        let mut latest = HashMap::<PackageIdent, PackageIdent>::new();
        for ident in base_set {
            latest.insert(short_ident(&ident, false), ident.clone());
        }

        let mut built = HashMap::<PackageIdent, PackageBuild>::new();

        for component in build_order {
            // TODO: if there is only one element in component, don't need to converge, can just run
            // once
            for i in 1..converge_count {
                for node in component {
                    let build = self.build_package(node, latest);
                    latest.insert(short_ident(&build.ident, false), build.ident.clone());
                    built.insert(build.ident.clone(), build);
                }
            }
        }

        let build_actual = self.prune_tsort(built, latest);

        build_actual
    }

    pub fn compute_build_order(&self, rebuild_set: Vec<PackageIdent>) -> Vec<Vec<PackageIdent>> {}

    pub fn build_package(&self,
                         node: PackageIdent,
                         latest: HashMap<PackageIdent, PackageIdent>)
                         -> PackageBuild {
    }

    pub fn prune_tsort(&self,
                       built: HashMap<PackageIdent, PackageBuild>,
                       latest: HashMap<PackageIdent, PackageIdent>)
                       -> Vec<PackageBuild> {
    }
}
