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

use std::io::prelude::*;

use std::{cmp::Ordering,
          collections::{BinaryHeap,
                        HashMap}};

use petgraph::{algo::{connected_components,
                      is_cyclic_directed},
               dot::{Config,
                     Dot},
               graph::{EdgeIndex,
                       NodeIndex},
               stable_graph::StableGraph};

use habitat_builder_db::models::package::PackageWithVersionArray;

// use habitat_builder_protocol as protocol;

use crate::{hab_core::{error as herror,
                       package::{ident::Identifiable,
                                 PackageIdent,
                                 PackageTarget}},
            ident_graph::*,
            package_table::{PackageIndex,
                            PackageInfo,
                            PackageTable},
            rdeps::rdeps,
            util::*};

#[derive(Debug)]
pub struct Stats {
    pub node_count:     usize,
    pub edge_count:     usize,
    pub connected_comp: usize,
    pub is_cyclic:      bool,
}

#[derive(Eq)]
struct HeapEntry {
    pkg_index:  usize,
    rdep_count: usize,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &HeapEntry) -> Ordering { self.rdep_count.cmp(&other.rdep_count) }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &HeapEntry) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &HeapEntry) -> bool { self.pkg_index == other.pkg_index }
}

#[derive(Default)]
pub struct PackageGraphForTarget {
    target: Option<PackageTarget>,

    packages: PackageTable,

    // Map from truncated ident to latest matching; it could be origin/packagename, or
    // origin/packagename/version
    latest_map: HashMap<PackageIdent, PackageIndex>,

    // Possible refactor would be to put packageinfo in graph structure; complication is in
    // multigraph situations
    full_graph:                StableGraph<PackageIndex, EdgeType>,
    full_graph_node_index_map: HashMap<PackageIndex, NodeIndex>,

    // We build this alongside the full graph
    latest_graph: IdentGraph<PackageIndex>,
}

impl PackageGraphForTarget {
    pub fn new(target: PackageTarget) -> Self {
        let mut pg = PackageGraphForTarget::default();
        pg.target = Some(target);
        pg
    }

    // This is used for testing
    pub fn build<T>(&mut self, packages: T, use_build_deps: bool) -> (usize, usize)
        where T: Iterator<Item = PackageWithVersionArray>
    {
        for p in packages {
            self.extend(&p, use_build_deps);
        }

        (self.full_graph.node_count(), self.full_graph.edge_count())
    }

    fn update_latest<'a>(&'a mut self, id: &'a PackageIdent, index: PackageIndex) {
        let just_package = short_ident(id, false);
        self.update_if_newer(just_package, index);

        let package_version = short_ident(id, true);
        self.update_if_newer(package_version, index);
    }

    fn update_if_newer(&mut self, id: PackageIdent, index: PackageIndex) {
        match self.latest_map.get(&id) {
            Some(&old_index) => {
                if self.packages.get(index).unwrap().borrow().ident
                   > self.packages.get(old_index).unwrap().borrow().ident
                {
                    self.latest_map.insert(id, index);
                }
            }
            None => {
                self.latest_map.insert(id, index);
            }
        };
    }

    fn generate_id_for_package(&mut self,
                               package: &PackageWithVersionArray)
                               -> (PackageIndex, NodeIndex) {
        let (pi, ni) = self.generate_id(&package.ident.0);
        self.packages.insert_package(package);
        (pi, ni)
    }

    fn generate_id(&mut self, ident: &PackageIdent) -> (PackageIndex, NodeIndex) {
        let maybe_package_info = self.packages.find(ident);
        match maybe_package_info {
            Some(_package_info) => {
                let package_index = self.packages.generate_id(ident);
                let node_index = self.full_graph_node_index_map[&package_index];
                (package_index, node_index)
            }
            None => {
                let package_index = self.packages.generate_id(ident);
                let node_index = self.full_graph.add_node(package_index);
                self.full_graph_node_index_map
                    .insert(package_index, node_index);
                (package_index, node_index)
            }
        }
    }

    pub fn emit_graph(&self,
                      file: &str,
                      origin_filter: Option<&str>,
                      latest: bool,
                      edge_type: Option<EdgeType>) {
        let mut file = std::fs::File::create(file).unwrap();
        let filtered_graph: StableGraph<usize, EdgeType> =
            self.full_graph.filter_map(|node_index, node_data| {
                                           self.emit_node_filter(node_index,
                                                                 *node_data,
                                                                 origin_filter,
                                                                 latest)
                                       },
                                       |edge_index, edge_data| {
                                           self.emit_edge_filter(edge_index, *edge_data, edge_type)
                                       });

        file.write_all(
            format!(
                "{:?}",
                Dot::with_config(&filtered_graph, &[Config::EdgeNoLabel])
            )
            .as_bytes(),
        )
        .unwrap();
    }

    pub fn emit_node_filter(&self,
                            _node_index: NodeIndex,
                            node_data: usize,
                            _origin_filter: Option<&str>,
                            _latest_only: bool)
                            -> Option<usize> {
        // TODO something smarter here.
        Some(node_data)
    }

    pub fn emit_edge_filter(&self,
                            _edge_index: EdgeIndex,
                            edge_data: EdgeType,
                            _wanted_edge: Option<EdgeType>)
                            -> Option<EdgeType> {
        Some(edge_data)
    }

    // TODO: Need to implement a non-polynomial time check-extend method
    #[allow(clippy::map_entry)]
    pub fn extend(&mut self,
                  package: &PackageWithVersionArray,
                  use_build_deps: bool)
                  -> ((usize, usize), (usize, usize)) {
        // debug only
        // {
        //     if package.ident.0.name == "gcc" {
        //         println!("E: {}", package.ident.0);
        //     }
        // }
        let (pkg_id, pkg_node) = self.generate_id_for_package(package);

        assert_eq!(self.target.unwrap(), package.target.0);
        assert_eq!(pkg_id, pkg_node.index());

        // First, add to full graph
        // The full graph should never have
        // cycles, because we forbid them being added abuild things
        // after cutting cycles, but a user might get 'clever' so we
        // still need to check.
        {
            let ident = &package.ident.0;
            assert_eq!(self.packages.get(pkg_id).unwrap().borrow().ident, *ident);
            self.update_latest(ident, pkg_id);
        }

        let short_name = short_ident(&self.packages.get(pkg_id).unwrap().borrow().ident, false);

        for dep in &package.deps {
            let (_, dep_node) = self.generate_id(&dep.0);
            self.add_edge(pkg_node, dep_node, EdgeType::RuntimeDep);
        }

        if use_build_deps {
            for dep in &package.build_deps {
                let (_, dep_node) = self.generate_id(&dep.0);
                self.add_edge(pkg_node, dep_node, EdgeType::BuildDep);
            }
        }

        // Next, add to latest graph. We overwrite any prior ident, so that any incoming
        // dependencies are preserved.
        //

        // Are we the newest? Ignore older versions of the package
        if self.latest_map[&short_name] == pkg_id {
            // We start by adding a new node with dependencies pointing to the latest of each ident
            let (_, src_node_index) = self.latest_graph.upsert_node(&short_name, pkg_id);

            // Get rid of old edges; the new package version may changed its dependencies.

            // we will need to be checking for cycles here...
            // This drop/re-add process is likely too naive for a future on-line fast cycle
            // detection algorithm; it will introduce too much churn. We'll probably
            // have to move to a difference based system.
            self.latest_graph.drop_outgoing(src_node_index);

            let package_info = self.packages.get(pkg_id).unwrap();
            let package_info = package_info.borrow();

            // I'd like to write this as below, but borrow in closure is problematic.
            // package_info
            //     .plan_deps
            //     .iter()
            //     .for_each(|dep| self.add_edge_to_latest(src_node_index, dep,
            // EdgeType::RuntimeDep));
            for dep in &package_info.plan_deps {
                // skip fully qualified idents in the graph; they never will be t, so they only add
                // noise to the dependency graph
                if !dep.fully_qualified() {
                    self.latest_graph
                        .add_edge(src_node_index, dep, EdgeType::RuntimeDep);
                }
            }

            if use_build_deps {
                for dep in &package_info.plan_bdeps {
                    // skip fully qualified idents in the graph; they never will be t, so they only
                    // add noise to the dependency graph
                    if !dep.fully_qualified() {
                        self.latest_graph
                            .add_edge(src_node_index, dep, EdgeType::BuildDep);
                    }
                }
            }
        }
        ((self.full_graph.node_count(), self.full_graph.edge_count()), self.latest_graph.counts())
    }

    pub fn add_edge(&mut self, nfrom: NodeIndex, nto: NodeIndex, etype: EdgeType) {
        self.full_graph.add_edge(nfrom, nto, etype);
        // At some point we'll want to check for cycles, but not for now.
    }

    pub fn write_packages_json(&self, filename: &str, filter: Option<&str>) {
        self.packages.write_json(filename, filter)
    }

    pub fn read_packages_json(&mut self, filename: &str, use_build_edges: bool) {
        let packages = read_packages_json(filename);
        for package in packages {
            self.extend(&package, use_build_edges);
        }
    }

    pub fn rdeps(&self, name: &PackageIdent) -> Option<Vec<(String, String)>> {
        let mut v: Vec<(String, String)> = Vec::new();

        match self.packages.find_index(name) {
            Some(pkg_index) => {
                let pkg_node = self.full_graph_node_index_map[&pkg_index];
                match rdeps(&self.full_graph, pkg_node) {
                    Ok(deps) => {
                        for n in deps {
                            let package = self.packages.get(n).unwrap();
                            let package = package.borrow();
                            let name = &package.ident;
                            let ident = format!("{}", self.latest_map[&name]);
                            let namestr = format!("{}", name);
                            v.push((namestr, ident));
                        }
                    }
                    Err(e) => panic!("Error: {:?}", e),
                }
            }
            None => return None,
        }

        Some(v)
    }

    // Mostly for debugging
    pub fn rdeps_dump(&self) {
        debug!("Reverse dependencies:");

        for pkg_index in 0..self.packages.count() {
            let package_info = self.packages.get(pkg_index).unwrap();
            let pkg_ident = &package_info.borrow().ident;
            let pkg_node = self.full_graph_node_index_map[&pkg_index];
            debug!("{}", pkg_ident);

            match rdeps(&self.full_graph, pkg_node) {
                Ok(v) => {
                    for n in v {
                        debug!("|_ {}", self.packages.get(n).unwrap().borrow().ident);
                    }
                }
                Err(e) => panic!("Error: {:?}", e),
            }
        }
    }

    pub fn search(&self, phrase: &str) -> Vec<String> {
        // TODO: Rework this for new PackageTable construct
        //      let v: Vec<String> = self
        //     .packages
        //     .values()
        //     .map(|package| format!("{}", package.borrow().ident))
        //     .filter(|s| s.contains(phrase))
        //     .collect();
        // v
        vec![]
    }

    pub fn latest(&self) -> Vec<String> {
        self.latest_map.values().map(|x| format!("{}", x)).collect()
    }

    // Given an identifier in 'origin/name' format, returns the
    // most recent version (fully-qualified package ident string)
    pub fn resolve(&self, ident: &PackageIdent) -> Option<PackageIdent> {
        let index = self.latest_map.get(ident);
        index.map(|x| self.packages.get(*x).unwrap().borrow().ident.clone())
    }

    pub fn stats(&self) -> Stats {
        Stats { node_count:     self.full_graph.node_count(),
                edge_count:     self.full_graph.edge_count(),
                connected_comp: 0, // connected_components(&self.full_graph),
                is_cyclic:      is_cyclic_directed(&self.full_graph), }
    }

    // Who has the most things depending on them?
    pub fn top(&self, max: usize) -> Vec<(String, usize)> {
        let mut v = Vec::new();
        let mut heap = BinaryHeap::new();

        for &package_index in self.latest_map.values() {
            let node_id = self.full_graph_node_index_map[&package_index];

            match rdeps(&self.full_graph, node_id) {
                Ok(v) => {
                    let he = HeapEntry { pkg_index:  package_index,
                                         rdep_count: v.len(), };
                    heap.push(he);
                }
                Err(e) => panic!("Error: {:?}", e),
            }
        }

        let mut i = 0;
        while (i < max) && !heap.is_empty() {
            let he = heap.pop().unwrap();
            v.push((self.packages.get_ident(he.pkg_index).unwrap().to_string(), he.rdep_count));
            i += 1;
        }

        v
    }

    pub fn write_deps(&self, ident: &PackageIdent) {
        let maybe_package_index: Option<PackageIndex> = if ident.fully_qualified() {
            self.packages.find_index(&ident)
        } else {
            self.latest_map.get(&ident).map(|x| *x)
        };
        match maybe_package_index {
            Some(pi) => self.packages.get(pi).unwrap().borrow().write(),
            None => println!("Couldn't find match for {}", ident),
        }
    }

    pub fn dump_graph(&self, _file: &str) {
        println!("dump_graph unimplemented");
    }

    // Output a human readable, machine parsable form of the graph; useful for debugging
    pub fn dump_latest_graph_raw(&self, file: &str, origin: Option<&str>) {
        self.latest_graph.dump_graph_raw(file, origin)
    }

    // The built in Dot utility wasn't flexible for what I wanted, so implemented our own.
    pub fn dump_latest_graph_as_dot(&self, file: &str, origin: Option<&str>) {
        self.latest_graph.emit_graph_as_dot(file, origin)
    }

    pub fn dump_build_levels(&self, file: &str, origin: Option<&str>) {
        self.latest_graph.dump_build_levels(file, origin)
    }

    pub fn dump_scc(&self, file: &str, origin: Option<&str>) {
        self.latest_graph.dump_scc(file, origin)
    }

    // COALFACE
    pub fn dump_diagnostics(&self, file: &str, _origin: Option<&str>) {
        let mut file = std::fs::File::create(file).unwrap();

        let mut package_count = HashMap::<PackageIdent, usize>::new();
        file.write("============package list ======n".as_bytes())
            .unwrap();
        for package in self.packages.values_ref() {
            file.write(format!("{}\n", package.borrow().ident).as_bytes())
                .unwrap();
            let count = package_count.entry(short_ident(&package.borrow().ident, false))
                                     .or_insert(0);
            *count += 1;
        }
    }

    pub fn dump_build_ordering(&self,
                               _filename: &str,
                               origin: &str,
                               base_set: &Vec<PackageIdent>,
                               touched: &Vec<PackageIdent>) {
        let build = self.latest_graph
                        .compute_build(origin, base_set, touched, 3);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util;
    use std::str::FromStr;
    #[test]
    fn write_restore_packages() {
        let empty: [&str; 0] = [];

        let package1 = util::mk_package_with_versionarray("foo/bar/1/2",
                                                          "x86_64-linux",
                                                          &["foo/baz/1/2"],
                                                          &empty);

        let mut vec = Vec::new();
        vec.push(package1);

        let tmpfile = "/tmp/junk"; // Do this smarter
        let mut graph =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        graph.build(vec.into_iter(), true);

        graph.write_packages_json(tmpfile, None);

        let mut graph2 =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        graph2.read_packages_json(tmpfile, true);
        let stats = graph2.stats();
        assert_eq!(stats.node_count, 2);
    }

    // TODO RE-ENABLE once circular deps detection is fixed.
    // However that's going to require some serious rework of the code.
    // #[test]
    #[allow(dead_code)]
    fn disallow_circular_dependency() {
        let mut graph =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        let mut packages = Vec::new();
        let empty: [&str; 0] = [];

        let package1 = util::mk_package_with_versionarray("foo/bar/1/2",
                                                          "x86_64-linux",
                                                          &["foo/baz/1/2"],
                                                          &empty);
        packages.push(package1);
        let package2 = util::mk_package_with_versionarray("foo/baz/1/2",
                                                          "x86_64-linux",
                                                          &["foo/bar/1/2"],
                                                          &empty);
        packages.push(package2);

        let (ncount, ecount) = graph.build(packages.into_iter(), true);

        assert_eq!(ncount, 2);
        assert_eq!(ecount, 1); // only the first edge added

        let stats = graph.stats();
        assert_eq!(stats.is_cyclic, false);

        // let pre_check = graph.check_extend(&package2, true);
        // assert_eq!(pre_check, false);
    }

    #[test]
    fn pre_check_with_dep_not_present() {
        let mut graph =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        let empty: [&str; 0] = [];

        let package1 = util::mk_package_with_versionarray("foo/bar/1/2",
                                                          "x86_64-linux",
                                                          &["foo/baz/1/2"],
                                                          &empty);

        let package2 = util::mk_package_with_versionarray("foo/baz/1/2",
                                                          "x86_64-linux",
                                                          &["foo/xyz/1/2"],
                                                          &empty);

        // let pre_check1 = graph.check_extend(&package1, true);
        // assert_eq!(pre_check1, true);

        let (..) = graph.extend(&package1, true);

        // let pre_check2 = graph.check_extend(&package2, true);
        // assert_eq!(pre_check2, true);

        let (..) = graph.extend(&package2, true);
    }
}
