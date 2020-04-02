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

use std::collections::{HashMap,
                       HashSet};

use petgraph::{algo::tarjan_scc,
               graph::NodeIndex};

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

        println!("Rebuild: {} {}",
                 rebuild_set.len(),
                 join_idents(", ", &rebuild_set));

        let build_order = self.compute_build_order(&rebuild_set);

        // Rework this later
        println!("CB: {} components", build_order.len());
        for component in &build_order {
            println!("CB: C:{} {}", component.len(), join_idents(", ", component));
        }

        let mut latest = HashMap::<PackageIdent, PackageIdent>::new();
        for ident in base_set {
            latest.insert(short_ident(&ident, false), ident.clone());
        }

        let mut built = HashMap::<PackageIdent, PackageBuild>::new();

        for component in &build_order {
            // TODO: if there is only one element in component, don't need to converge, can just run
            // once
            for _i in 1..converge_count {
                for node in component {
                    let build = self.build_package(&node, &latest);
                    latest.insert(short_ident(&build.ident, false), build.ident.clone());
                    built.insert(build.ident.clone(), build);
                }
            }
        }

        let build_actual = self.prune_tsort(&built, &latest);

        build_actual
    }

    // This could be implmented by creating a subgraph in PetGraph, but my initial experiments had
    // issues with NodeIndex changing in the new graph, which scrambled our system for tracking
    // things via NodeIndex. It might be worth converting to GraphMap, which would remove the
    // need to track, and thus enable the use of subgraphs.
    pub fn compute_build_order(&self, rebuild_set: &Vec<PackageIdent>) -> Vec<Vec<PackageIdent>> {
        // compute SCC
        //

        let scc = self.filtered_scc(rebuild_set);

        let mut node_order: Vec<Vec<NodeIndex>> = Vec::new();
        for component in scc {
            node_order.push(self.tsort_subgraph(&component))
        }

        let ident_result =
            node_order.iter()
                      .map(|c| c.iter().map(|n| self.ident_for_node(*n).clone()).collect())
                      .collect();

        ident_result
    }

    pub fn filtered_scc(&self, rebuild_set: &Vec<PackageIdent>) -> Vec<Vec<NodeIndex>> {
        // This a returns a vector of components, each of which
        // contains a vector of nodes in the component. A component
        // may only contain a single node, when that node has no back
        // edges/ cyclic dependencies. These nodes are returned in
        // topological sort order. All we need to do to compute a
        // valid build ordering is to take the components and sort
        // them in runtime edge topological order
        let scc: Vec<Vec<NodeIndex>> = tarjan_scc(&self.graph);

        let mut rebuild_nodeindex = HashSet::new();
        for ident in rebuild_set {
            let (node_index, _) = self.get_node_if_exists(&ident);
            rebuild_nodeindex.insert(node_index);
        }

        // Most common case is core, which is a substantial fraction of the total packages we would
        // automatically rebuild, so we choose a size on the larger end to avoid
        // reallocation.
        let mut filtered_set = Vec::with_capacity(scc.len());
        for component in scc {
            // Maybe there's a more idomatic way of writing the filter body?
            let result = component.iter().fold(0, |count, node_index| {
                                             if rebuild_nodeindex.contains(node_index) {
                                                 count + 1
                                             } else {
                                                 count
                                             }
                                         });

            match result {
                0 => (),
                len if len == component.len() => filtered_set.push(component.clone()),
                _ => {
                    panic!("Unexpected filter result {}, expected 0 or {}",
                           result,
                           component.len())
                }
            }
        }
        filtered_set
    }

    pub fn build_package(&self,
                         node: &PackageIdent,
                         _latest: &HashMap<PackageIdent, PackageIdent>)
                         -> PackageBuild {
        PackageBuild { ident: node.clone(),
                       bdeps: Vec::new(),
                       rdeps: Vec::new(), }
    }

    pub fn prune_tsort(&self,
                       _built: &HashMap<PackageIdent, PackageBuild>,
                       _latest: &HashMap<PackageIdent, PackageIdent>)
                       -> Vec<PackageBuild> {
        Vec::new()
    }
}
