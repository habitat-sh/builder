// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
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
          collections::HashMap};

use habitat_builder_db::models::package::PackageWithVersionArray;

// use habitat_builder_protocol as protocol;

use crate::{build_ordering::PackageBuild,
            hab_core::package::{PackageIdent,
                                PackageTarget},
            package_graph_target::{PackageGraphForTarget,
                                   Stats},
            util::*};

// Multitarget support
//
pub struct PackageGraph {
    current_target: PackageTarget,
    graphs:         HashMap<PackageTarget, RefCell<PackageGraphForTarget>>,
}

impl Default for PackageGraph {
    fn default() -> Self { Self::new() }
}

impl PackageGraph {
    pub fn new() -> Self {
        PackageGraph { current_target: PackageTarget::active_target(),
                       graphs:         HashMap::new(), }
    }

    pub fn build<T>(&mut self, packages: T, use_build_deps: bool) -> (usize, usize)
        where T: Iterator<Item = PackageWithVersionArray>
    {
        for p in packages {
            let target = p.target.0;
            self.graphs
                .entry(target)
                .or_insert_with(|| RefCell::new(PackageGraphForTarget::new(target)))
                .borrow_mut()
                .extend(&p, use_build_deps);
        }

        // TODO Extract this info better
        (0, 0)
    }

    pub fn clear(&mut self) { self.graphs.clear() }

    pub fn targets(&self) -> Vec<PackageTarget> { self.graphs.keys().copied().collect() }

    pub fn current_target(&self) -> PackageTarget { self.current_target }

    pub fn set_target(&mut self, target: PackageTarget) {
        self.current_target = target;
        if !self.graphs.contains_key(&target) {
            println!("No data for target {}", target);
            println!("Data for target can be loaded with 'load_file' or 'load_db'");
        }
    }

    // Delegate to subgraphs
    pub fn rdeps(&self, name: &PackageIdent) -> Option<Vec<(String, String)>> {
        if let Some(target_graph) = self.graphs.get(&self.current_target) {
            target_graph.borrow().rdeps(name)
        } else {
            println!("No graph loaded for target: {}", &self.current_target);
            None
        }
    }

    pub fn search(&self, phrase: &str) -> Vec<String> {
        if let Some(target_graph) = self.graphs.get(&self.current_target) {
            target_graph.borrow().search(phrase)
        } else {
            println!("No graph loaded for target: {}", &self.current_target);
            Vec::new()
        }
    }

    pub fn latest(&self) -> Vec<String> {
        if let Some(target_graph) = self.graphs.get(&self.current_target) {
            target_graph.borrow().latest()
        } else {
            println!("No graph loaded for target: {}", &self.current_target);
            Vec::new()
        }
    }

    pub fn resolve(&self, ident: &PackageIdent) -> Option<PackageIdent> {
        if let Some(target_graph) = self.graphs.get(&self.current_target) {
            target_graph.borrow().resolve(ident)
        } else {
            println!("No graph loaded for target: {}", &self.current_target);
            None
        }
    }

    pub fn stats(&self) -> Option<Stats> {
        if let Some(target_graph) = self.graphs.get(&self.current_target) {
            Some(target_graph.borrow().stats())
        } else {
            println!("No graph loaded for target: {}", &self.current_target);
            None
        }
    }

    // TODO SORT THESE
    pub fn all_stats(&self) -> Vec<(PackageTarget, Stats)> {
        self.graphs
            .keys()
            .map(|key| (*key, self.graphs[&key].borrow().stats()))
            .collect()
    }

    pub fn top(&self, max: usize) -> Vec<(String, usize)> {
        self.graphs[&self.current_target].borrow().top(max)
    }

    pub fn emit_graph(&self,
                      file: &str,
                      origin_filter: Option<&str>,
                      latest: bool,
                      edge_type: Option<EdgeType>) {
        self.graphs[&self.current_target].borrow()
                                         .emit_graph(file, origin_filter, latest, edge_type)
    }

    pub fn write_deps(&self, ident: &PackageIdent) {
        if let Some(target_graph) = self.graphs.get(&self.current_target) {
            target_graph.borrow().write_deps(ident);
        } else {
            println!("No graph loaded for target: {}", &self.current_target);
        }
    }

    pub fn dump_graph(&self, file: &str) {
        self.graphs[&self.current_target].borrow().dump_graph(file)
    }

    pub fn dump_latest_graph_raw(&self, file: &str, origin: Option<&str>) {
        self.graphs[&self.current_target].borrow()
                                         .dump_latest_graph_raw(file, origin)
    }

    pub fn dump_latest_graph_as_dot(&self, file: &str, origin: Option<&str>) {
        self.graphs[&self.current_target].borrow()
                                         .dump_latest_graph_as_dot(file, origin)
    }

    pub fn dump_scc(&self, file: &str, origin: Option<&str>) {
        self.graphs[&self.current_target].borrow()
                                         .dump_scc(file, origin)
    }

    pub fn dump_build_levels(&self, file: &str, origin: Option<&str>) {
        self.graphs[&self.current_target].borrow()
                                         .dump_build_levels(file, origin)
    }

    pub fn write_packages_json(&self, filename: &str, filter: Option<&str>) {
        self.graphs[&self.current_target].borrow()
                                         .write_packages_json(filename, filter)
    }

    pub fn dump_diagnostics(&self, filename: &str, filter: Option<&str>) {
        self.graphs[&self.current_target].borrow()
                                         .dump_diagnostics(filename, filter)
    }

    pub fn dump_build_ordering(&self,
                               filename: &str,
                               filter: &str,
                               base_set: &Vec<PackageIdent>,
                               touched: &Vec<PackageIdent>)
                               -> Vec<PackageBuild> {
        self.graphs[&self.current_target].borrow()
                                         .dump_build_ordering(filename, filter, base_set, touched)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util;
    #[test]
    fn empty_graph() {
        let mut graph = PackageGraph::new();
        let packages = Vec::new();
        let (ncount, ecount) = graph.build(packages.into_iter(), true);
        assert_eq!(ncount, 0);
        assert_eq!(ecount, 0);
    }
}
