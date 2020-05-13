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

use std::{cell::RefCell,
          collections::{HashMap,
                        HashSet},
          fs::File,
          io::Write,
          rc::Rc};

use petgraph::{algo::tarjan_scc,
               graph::NodeIndex};

use crate::hab_core::package::PackageIdent;

use crate::{build_z3::emit_z3,
            ident_graph::IdentGraph,
            package_table::{PackageIndex,
                            PackageInfo,
                            PackageTable},
            util::*};

type IdentIndex = usize;

pub struct PackageBuild {
    pub ident:   PackageIdent,
    pub bt_deps: Vec<PackageIdent>,
    pub rt_deps: Vec<PackageIdent>,
}

impl PackageBuild {
    pub fn format_for_shell(&self) -> String {
        let short_ident = short_ident(&self.ident, false).to_string();
        let deps: Vec<PackageIdent> = self.bt_deps
                                          .iter()
                                          .chain(self.rt_deps.iter())
                                          .map(|x| x.clone())
                                          .collect();
        format!("{}\t{}\t{}\n",
                short_ident,
                self.ident,
                join_idents(",", &deps))
    }
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

    pub fn compute_build(&mut self,
                         origin: &str,
                         package_table: &PackageTable,
                         latest_map: &HashMap<PackageIdent, PackageIndex>,
                         base_set: &Vec<PackageIdent>,
                         touched: &Vec<PackageIdent>,
                         converge_count: usize)
                         -> Vec<PackageBuild> {
        // debug!("Using base: {} {}\n",
        // base_set.len(),
        // join_idents(", ", &base_set));

        println!("Using touched: {} {}\n",
                 touched.len(),
                 join_idents(", ", &touched));
        self.dump_graph_raw("raw-pre-graph.txt", Some("core"));
        // TODO We should check

        let rebuild_set = self.compute_rebuild_set(touched, origin);

        // TODO DO check of rebuild set to make sure that it includes the pinned versions that had
        // edges added in the precondition_graph phase above.

        println!("Rebuild: {} {}\n",
                 rebuild_set.len(),
                 join_idents(", ", &rebuild_set));

        let build_order = self.compute_build_order(&rebuild_set);
        // Rework this later
        debug!("CB: {} components", build_order.len());
        for component in &build_order {
            debug!("CB: #{} {}", component.len(), join_idents(", ", component));
        }

        let packages_in_build_order: Vec<Vec<Rc<RefCell<PackageInfo>>>> =
            build_order.iter()
                       .map(|package_set| {
                           package_set.iter()
                                      .map(|package_ident| {
                                          let index =
                                              latest_map.get(&package_ident)
                                                        .expect(format!("Couldn't find {} in \
                                                                         latest_map",
                                                                        package_ident).as_str());
                                          package_table.get(*index)
                                                       .expect(format!("Couldn't find {}",
                                                                       package_ident).as_str())
                                      })
                                      .collect()
                       })
                       .collect();

        let mut latest = HashMap::<PackageIdent, PackageIdent>::new();
        for ident in base_set {
            latest.insert(short_ident(&ident, false), ident.clone());
        }

        let mut file = File::create("latest_from_base.txt").expect("Failed to initialize file");
        for (k, v) in &latest {
            file.write(format!("{}: {}\n", &k, &v).as_bytes()).unwrap();
        }

        // let mut built = HashMap::<PackageIdent, PackageBuild>::new();
        let mut built: Vec<PackageBuild> = Vec::new();
        for component in &packages_in_build_order {
            // If there is only one element in component, don't need to converge, can just run
            // once
            let component_converge_count = if component.len() > 1 {
                converge_count
            } else {
                1
            };

            for _i in 1..=component_converge_count {
                for package_ref in component {
                    let package: &PackageInfo = &package_ref.borrow();
                    let build = self.build_package(package, &mut latest);
                    latest.insert(short_ident(&build.ident, false), build.ident.clone());
                    // built.insert(build.ident.clone(), build);
                    built.push(build);
                }
            }
        }

        self.dump_graph_raw("raw-graph.txt", Some(origin));

        // Z3 playground
        // let z3_file = format!("{}_build.smt2", origin);
        // let workers = 4;
        // println!("Emitting z3 constraint file {}", z3_file);
        // emit_z3(workers, &built, &HashMap::new(), &z3_file);

        // let build_actual = self.prune_tsort(&built, &latest);
        // build_actual
        built
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
            let ordered_component = self.tsort_subgraph(&component);
            node_order.push(ordered_component)
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
                         package: &PackageInfo,
                         latest: &mut HashMap<PackageIdent, PackageIdent>)
                         -> PackageBuild {
        // Create our package name
        let ident = make_temp_ident(&package.ident);

        // resolve our runtime and build deps
        let mut bt_deps = Vec::new();
        let mut rt_deps = Vec::new();

        // println!("Building package {} with BDEP {} RDEP {}",
        // ident,
        // join_idents(", ", &package.plan_bdeps),
        // join_idents(", ", &package.plan_deps));

        for dep in &package.plan_bdeps {
            // Horrible hack to get around our own pinning
            let sdep = short_ident(dep, false);
            bt_deps.push(latest.get(&sdep)
                               .expect(format!("{} Unable to find bt dep {} ({})",
                                               &ident, &dep, &sdep).as_str())
                               .clone())
        }
        for dep in &package.plan_deps {
            // Horrible hack to get around our own pinning
            let sdep = short_ident(dep, false);
            rt_deps.push(latest.get(&sdep)
                               .expect(format!("{} Unable to find rt dep {} ({})",
                                               &ident, &dep, &sdep).as_str())
                               .clone())
        }

        // update latest
        latest.insert(short_ident(&ident, false), ident.clone());
        latest.insert(short_ident(&ident, true), ident.clone());

        // Make the package
        PackageBuild { ident,
                       bt_deps,
                       rt_deps }
    }

    pub fn prune_tsort(&self,
                       _built: &HashMap<PackageIdent, PackageBuild>,
                       _latest: &HashMap<PackageIdent, PackageIdent>)
                       -> Vec<PackageBuild> {
        Vec::new()
    }
}
