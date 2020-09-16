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
// limitations under the License

use petgraph::{algo::{connected_components,
                      is_cyclic_directed},
               graph::NodeIndex,
               graphmap::DiGraphMap,
               Direction,
               Graph};
use std::{cmp::Ordering,
          collections::{BinaryHeap,
                        HashMap,
                        HashSet,
                        VecDeque},
          iter::FromIterator,
          str::FromStr};

use habitat_core::package::PackageTarget;

use crate::{acyclic_rdeps::rdeps,
            data_store::Unbuildable,
            hab_core::package::PackageIdent,
            package_build_manifest_graph::{PackageBuildManifest,
                                           UnbuildableReason,
                                           UnresolvedPackageIdent},
            package_graph_trait::{PackageGraphTrait,
                                  Stats},
            package_ident_intern::PackageIdentIntern,
            protocol::originsrv,
            util::EdgeType};

fn short_name(name: &str) -> String {
    let parts: Vec<&str> = name.split('/').collect();
    assert!(parts.len() >= 2);
    format!("{}/{}", parts[0], parts[1])
}

#[derive(Default)]
pub struct AcyclicPackageGraph {
    package_max:   usize,
    package_map:   HashMap<String, (usize, NodeIndex)>,
    latest_map:    HashMap<String, PackageIdent>,
    package_names: Vec<String>,
    graph:         Graph<usize, usize>,
}

impl PackageGraphTrait for AcyclicPackageGraph {
    fn build(&mut self,
             packages: &[originsrv::OriginPackage],
             use_build_deps: bool)
             -> (usize, usize) {
        assert!(self.package_max == 0);

        for p in packages {
            self.extend(&p, use_build_deps);
        }

        (self.graph.node_count(), self.graph.edge_count())
    }

    fn check_extend(&mut self, package: &originsrv::OriginPackage, use_build_deps: bool) -> bool {
        let name = format!("{}", package.get_ident());
        let pkg_short_name = short_name(&name);

        // If package is brand new, we can't have a circular dependency
        if !self.package_map.contains_key(&pkg_short_name) {
            debug!("check_extend: no package found - OK");
            return true;
        }

        let (_, pkg_node) = self.package_map[&pkg_short_name];

        // Temporarily remove edges
        let mut saved_nodes = Vec::new();
        let neighbors: Vec<NodeIndex> = self.graph
                                            .neighbors_directed(pkg_node, Direction::Incoming)
                                            .collect();
        for n in neighbors {
            let e = self.graph.find_edge(n, pkg_node).unwrap();
            saved_nodes.push(n);
            self.graph.remove_edge(e).unwrap();
        }

        // Check to see if extension would create a circular dependency
        let mut circular_dep = false;
        let mut dep_nodes = Vec::new();
        let mut deps;
        let build_deps;

        if use_build_deps {
            deps = package.get_deps().iter().collect::<Vec<_>>();
            build_deps = package.get_build_deps();

            deps.extend(build_deps);
        } else {
            deps = package.get_deps().iter().collect::<Vec<_>>();
        }

        for dep in deps {
            let dep_name = format!("{}", dep);
            let dep_short_name = short_name(&dep_name);

            if self.package_map.contains_key(&dep_short_name) {
                let (_, dep_node) = self.package_map[&dep_short_name];
                dep_nodes.push(dep_node);

                self.graph.extend_with_edges(&[(dep_node, pkg_node)]);

                // Check for circular dependency
                if is_cyclic_directed(&self.graph) {
                    debug!("graph is cyclic after adding {} -> {} - failing check_extend",
                           dep_name, name);
                    circular_dep = true;
                    break;
                }
            }
        }

        // Undo the edge changes
        for dep_node in dep_nodes {
            let e = self.graph.find_edge(dep_node, pkg_node).unwrap();
            self.graph.remove_edge(e).unwrap();
        }

        for saved_node in saved_nodes {
            self.graph.extend_with_edges(&[(saved_node, pkg_node)]);
        }

        !circular_dep
    }

    #[allow(clippy::map_entry)]
    fn extend(&mut self,
              package: &originsrv::OriginPackage,
              use_build_deps: bool)
              -> (usize, usize) {
        let name = format!("{}", package.get_ident());
        let (pkg_id, pkg_node) = self.generate_id(&name);

        assert_eq!(pkg_id, pkg_node.index());

        let pkg_ident = PackageIdent::from_str(&name).unwrap();
        let short_name = short_name(&name);

        let add_deps = if self.latest_map.contains_key(&short_name) {
            let skip_update = {
                let latest = &self.latest_map[&short_name];
                pkg_ident < *latest
            };

            if skip_update {
                false
            } else {
                let neighbors: Vec<NodeIndex> =
                    self.graph
                        .neighbors_directed(pkg_node, Direction::Incoming)
                        .collect();
                for n in neighbors {
                    let e = self.graph.find_edge(n, pkg_node).unwrap();
                    self.graph.remove_edge(e).unwrap();
                }
                self.latest_map.insert(short_name, pkg_ident);
                true
            }
        } else {
            self.latest_map.insert(short_name, pkg_ident);
            true
        };

        if add_deps {
            let mut deps;
            let build_deps;

            if use_build_deps {
                deps = package.get_deps().iter().collect::<Vec<_>>();
                build_deps = package.get_build_deps();
                deps.extend(build_deps);
            } else {
                deps = package.get_deps().iter().collect::<Vec<_>>();
            }

            for dep in deps {
                let depname = format!("{}", dep);

                let (_, dep_node) = self.generate_id(&depname);
                self.graph.extend_with_edges(&[(dep_node, pkg_node)]);

                // sanity check
                if is_cyclic_directed(&self.graph) {
                    warn!("graph is cyclic after adding {} -> {} - rolling back",
                          depname, name);
                    let e = self.graph.find_edge(dep_node, pkg_node).unwrap();
                    self.graph.remove_edge(e).unwrap();
                }
            }
        }

        (self.graph.node_count(), self.graph.edge_count())
    }

    fn rdeps(&self, name: &str) -> Option<Vec<(String, String)>> {
        let mut v: Vec<(String, String)> = Vec::new();

        match self.package_map.get(name) {
            Some(&(_, pkg_node)) => {
                match rdeps(&self.graph, pkg_node) {
                    Ok(deps) => {
                        for n in deps {
                            let name = self.package_names[n].clone();
                            let ident = format!("{}", self.latest_map[&name]);
                            v.push((name, ident));
                        }
                    }
                    Err(e) => panic!("Error: {:?}", e),
                }
            }
            None => return None,
        }

        Some(v)
    }

    // Given an identifier in 'origin/name' format, returns the
    // most recent version (fully-qualified package ident string)
    fn resolve(&self, name: &str) -> Option<String> {
        match self.latest_map.get(name) {
            Some(ident) => Some(format!("{}", ident)),
            None => None,
        }
    }

    fn stats(&self) -> Stats {
        Stats { node_count:     self.graph.node_count(),
                edge_count:     self.graph.edge_count(),
                connected_comp: connected_components(&self.graph),
                is_cyclic:      is_cyclic_directed(&self.graph), }
    }

    // compute_build for AcyclicGraph
    // Process
    // 1) Take kernel of packages, and recursively expand it over reverse build/runtime deps
    //    creating a new sub-graph.
    // 2) Filter unbuildable nodes in subgraph, flag them as directly
    //    (no plan connection, autobuilds off) unbuildable
    // 3) Use the unbuildable packages as kernel, expand it over the subgraph to
    //    create a list of all unbuildable packages.
    // 4) Iterate over all unbuildable packages.
    //    Flag new entries as indirectly unbuildable.
    //    Remove that package from the sub-graph
    // 5) Walk the remaining sub-graph, collecting dependencies that are external to
    //    the sub-graph.
    // 6) Create a PackageBuildManifestGraph using the information generated in 1-5.
    fn compute_build(&self,
                     touched: &[PackageIdentIntern],
                     unbuildable: &dyn Unbuildable,
                     target: PackageTarget /* WRONG WRONG WRONG WRONG, WRONG WRONG WRONG
                                            * WRONG */)
                     -> PackageBuildManifest {
        let mut rebuild_graph: DiGraphMap<PackageIdentIntern, EdgeType> = DiGraphMap::new();
        let mut unbuildable_map: HashMap<PackageIdentIntern, UnbuildableReason> = HashMap::new();
        let mut all_external_dependencies: HashSet<PackageIdentIntern> = HashSet::new();

        let mut worklist: VecDeque<PackageIdentIntern> =
            VecDeque::from_iter(touched.iter().copied());
        let mut seen: HashSet<PackageIdentIntern> = HashSet::from_iter(touched.iter().copied());

        // Starting with our 'touched' set, walk the acyclic graph adding all
        // reverse dependencies to a new sub-graph.
        //
        // Error case: A package is missing from the graph
        //   * This package is flagged as missing in the unbuildables list.
        while !worklist.is_empty() {
            // Assumption: package is a short ident
            let package: PackageIdentIntern = worklist.pop_back().unwrap();

            if let Some((_, node_index)) = self.package_map.get(&package.to_string()) {
                // We reverse the sense of dependency edges when building the graph, so 'outgoing'
                // edges are really pointing to packages that depend on us
                for neighbor in self.graph
                                    .neighbors_directed(*node_index, Direction::Outgoing)
                {
                    let short_ident =
                        PackageIdentIntern::from_str(&self.package_names[neighbor.index()])
                            .expect("Unable to generate PackageIdentIntern");

                    if seen.insert(short_ident) {
                        worklist.push_back(short_ident);
                    }

                    rebuild_graph.add_edge(short_ident, package, EdgeType::RuntimeDep);
                }
            } else {
                // It's possible we've never seen this package because it's the first time it was
                // built, and so never uploaded. That should only happen when we're explicitly
                // rebuilding the package, e.g. it's in the touched set.
                if touched.contains(&package) {
                    rebuild_graph.add_node(package);
                } else {
                    unbuildable_map.insert(package, UnbuildableReason::Missing);
                }
            }
        }

        // Query the oracle for unbuildable packages. Use the returned set as the
        // kernel to flood the sub-graph to create a smaller subset that will be
        // pruned in a future step.
        // Flag any packages returned from the oracle as directly unbuildable
        let rebuild_idents: Vec<PackageIdentIntern> = rebuild_graph.nodes().collect();
        let flooded =
            if let Ok(unbuildables) = unbuildable.filter_unbuildable(&rebuild_idents, target) {
                for package in &unbuildables {
                    unbuildable_map.entry(*package)
                                   .or_insert(UnbuildableReason::Direct);
                }
                crate::graph_helpers::flood_deps_in_origin(&rebuild_graph, &unbuildables, None)
            } else {
                warn!("Filter unbuildable returned an error?");
                Vec::new()
            };

        // Use the flooded subset to prune the sub-graph of unbuildable packages.
        // Add any packages not already in the unbuildable map as indirectly unbuildable
        for package in flooded {
            unbuildable_map.entry(package)
                           .or_insert(UnbuildableReason::Indirect);

            rebuild_graph.remove_node(package);
        }

        let mut unresolved_rebuild_graph: DiGraphMap<UnresolvedPackageIdent, EdgeType> =
            DiGraphMap::new();
        // Compute required external deps
        for package in rebuild_graph.nodes() {
            // for my deps if dep is in graph, skip, otherwise add to external_deps
            if let Some((_, node_index)) = self.package_map.get(&package.to_string()) {
                for neighbor in self.graph
                                    .neighbors_directed(*node_index, Direction::Incoming)
                {
                    let neighbor_short_ident =
                        PackageIdentIntern::from_str(&self.package_names[neighbor.index()])
                            .unwrap_or_else(|_| {
                                // For this to happen, the graph has to have a node that was
                                // never entered or somehow removed from package_names. This
                                // seems only possible with a deeply broken graph, and so our
                                // best course is to panic and restart jobsrv
                                panic!(
                                    "Unable to generate PackageIdentIntern for dependency of {}",
                                    package
                                )
                            });
                    if rebuild_graph.contains_node(neighbor_short_ident) {
                        unresolved_rebuild_graph.add_edge(UnresolvedPackageIdent::InternalNode(package, 1),
                                                          UnresolvedPackageIdent::InternalNode(neighbor_short_ident, 1),
                                                          EdgeType::RuntimeDep);
                    } else {
                        unresolved_rebuild_graph.add_edge(UnresolvedPackageIdent::InternalNode(package, 1),
                                                          UnresolvedPackageIdent::ExternalLatestVersion(neighbor_short_ident),
                                                          EdgeType::RuntimeDep);
                        all_external_dependencies.insert(neighbor_short_ident);
                    }
                }
            } else {
                // Because of how we process things in the worklist algorithm above, we think this
                // only can happen if the graph changed under us. That should never
                // happen (we are taking a lock on the graph in the calling code)
                // Alternatively we could return a result and cancel the job in the calling code.
                panic!("Could not find package {} when computing build manifest",
                       package)
            }
        }

        PackageBuildManifest { graph:                 unresolved_rebuild_graph,
                               input_set:             HashSet::from_iter(touched.iter().copied()),
                               external_dependencies: all_external_dependencies,
                               unbuildable_reasons:   unbuildable_map, }
    }
}

impl AcyclicPackageGraph {
    pub fn new() -> Self { AcyclicPackageGraph::default() }

    #[allow(clippy::map_entry)]
    fn generate_id(&mut self, name: &str) -> (usize, NodeIndex) {
        let short_name = short_name(name);

        if self.package_map.contains_key(&short_name) {
            self.package_map[&short_name]
        } else {
            self.package_names.push(short_name.clone());
            assert_eq!(self.package_names[self.package_max], short_name);

            let node_index = self.graph.add_node(self.package_max);
            self.package_map
                .insert(short_name, (self.package_max, node_index));
            self.package_max += 1;

            (self.package_max - 1, node_index)
        }
    }

    // Mostly for debugging
    #[allow(dead_code)]
    fn rdeps_dump(&self) {
        debug!("Reverse dependencies:");

        for (pkg_name, pkg_id) in &self.package_map {
            let (_, node) = *pkg_id;
            debug!("{}", pkg_name);

            match rdeps(&self.graph, node) {
                Ok(v) => {
                    for n in v {
                        debug!("|_ {}", self.package_names[n]);
                    }
                }
                Err(e) => panic!("Error: {:?}", e),
            }
        }
    }

    #[allow(dead_code)]
    fn search(&self, phrase: &str) -> Vec<String> {
        let v: Vec<String> = self.package_names
                                 .iter()
                                 .cloned()
                                 .filter(|s| s.contains(phrase))
                                 .collect();

        v
    }

    #[allow(dead_code)]
    fn latest(&self) -> Vec<String> { self.latest_map.values().map(|x| format!("{}", x)).collect() }

    #[allow(dead_code)]
    fn top(&self, max: usize) -> Vec<(String, usize)> {
        let mut v = Vec::new();
        let mut heap = BinaryHeap::new();

        for pkg_id in self.package_map.values() {
            let (index, node) = *pkg_id;

            match rdeps(&self.graph, node) {
                Ok(v) => {
                    let he = HeapEntry { pkg_index:  index,
                                         rdep_count: v.len(), };
                    heap.push(he);
                }
                Err(e) => panic!("Error: {:?}", e),
            }
        }

        let mut i = 0;
        while (i < max) && !heap.is_empty() {
            let he = heap.pop().unwrap();
            v.push((self.package_names[he.pkg_index].clone(), he.rdep_count));
            i += 1;
        }

        v
    }
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

#[cfg(test)]
mod test {
    use super::*;
    use protobuf::RepeatedField;

    #[test]
    fn empty_graph() {
        let mut graph = AcyclicPackageGraph::new();
        let packages = Vec::new();
        let (ncount, ecount) = graph.build(&packages, true);
        assert_eq!(ncount, 0);
        assert_eq!(ecount, 0);
    }

    #[test]
    fn disallow_circular_dependency() {
        let mut graph = AcyclicPackageGraph::new();
        let mut packages = Vec::new();

        let mut package1 = originsrv::OriginPackage::new();
        package1.set_ident(originsrv::OriginPackageIdent::from_str("foo/bar/1/2").unwrap());
        let mut package1_deps = RepeatedField::new();
        package1_deps.push(originsrv::OriginPackageIdent::from_str("foo/baz/1/2").unwrap());
        package1.set_deps(package1_deps);
        packages.push(package1);

        let mut package2 = originsrv::OriginPackage::new();
        package2.set_ident(originsrv::OriginPackageIdent::from_str("foo/baz/1/2").unwrap());
        let mut package2_deps = RepeatedField::new();
        package2_deps.push(originsrv::OriginPackageIdent::from_str("foo/bar/1/2").unwrap());
        package2.set_deps(package2_deps);
        packages.push(package2.clone());

        let (ncount, ecount) = graph.build(&packages, true);

        assert_eq!(ncount, 2);
        assert_eq!(ecount, 1); // only the first edge added

        let stats = graph.stats();
        assert_eq!(stats.is_cyclic, false);

        let pre_check = graph.check_extend(&package2, true);
        assert_eq!(pre_check, false);
    }

    #[test]
    fn pre_check_with_dep_not_present() {
        let mut graph = AcyclicPackageGraph::new();

        let mut package1 = originsrv::OriginPackage::new();
        package1.set_ident(originsrv::OriginPackageIdent::from_str("foo/bar/1/2").unwrap());
        let mut package1_deps = RepeatedField::new();
        package1_deps.push(originsrv::OriginPackageIdent::from_str("foo/baz/1/2").unwrap());
        package1.set_deps(package1_deps);

        let mut package2 = originsrv::OriginPackage::new();
        package2.set_ident(originsrv::OriginPackageIdent::from_str("foo/baz/1/2").unwrap());
        let mut package2_deps = RepeatedField::new();
        package2_deps.push(originsrv::OriginPackageIdent::from_str("foo/xyz/1/2").unwrap());
        package2.set_deps(package2_deps);

        let pre_check1 = graph.check_extend(&package1, true);
        assert_eq!(pre_check1, true);

        let (..) = graph.extend(&package1, true);

        let pre_check2 = graph.check_extend(&package2, true);
        assert_eq!(pre_check2, true);

        let (..) = graph.extend(&package2, true);
    }

    fn make_package(ident: &str, deps: &[&str]) -> originsrv::OriginPackage {
        let mut package = originsrv::OriginPackage::new();
        package.set_ident(originsrv::OriginPackageIdent::from_str(ident).unwrap());

        let mut package_deps = RepeatedField::new();
        for dep in deps {
            package_deps.push(originsrv::OriginPackageIdent::from_str(dep).unwrap());
        }

        package.set_deps(package_deps);

        package
    }

    // This is very similar to the testing in CyclicPackageGraph. Signatures are slightly
    // different, and it wasn't worth the pain to be clever for a replication that
    // will disappear along with ACyclicPackageGraph
    fn make_diamond_graph() -> AcyclicPackageGraph {
        let packages = vec![make_package("a/top/c/d", &[]),
                            make_package("a/left/c/d", &["a/top/c/d"]),
                            make_package("a/right/c/d", &["a/top/c/d"]),
                            make_package("a/bottom/c/d", &["a/left/c/d", "a/right/c/d"]),];
        let mut graph = AcyclicPackageGraph::new();
        graph.build(&packages, true);
        graph
    }

    // maybe move to data_store.rs
    struct UnbuildableMock {
        pub unbuildable_packages: Vec<PackageIdentIntern>,
    }
    use crate::error::Result;
    impl Unbuildable for UnbuildableMock {
        fn filter_unbuildable(&self,
                              _: &[PackageIdentIntern],
                              _: PackageTarget)
                              -> Result<Vec<PackageIdentIntern>> {
            Ok(self.unbuildable_packages.clone())
        }
    }

    #[allow(non_snake_case)]
    fn mk_IN(ident: &str, rev: u8) -> UnresolvedPackageIdent {
        UnresolvedPackageIdent::InternalNode(ident_intern!(ident), rev)
    }

    #[allow(non_snake_case)]
    fn mk_ELV(ident: &str) -> UnresolvedPackageIdent {
        UnresolvedPackageIdent::ExternalLatestVersion(ident_intern!(ident))
    }

    #[test]
    // Starting with a diamond graph, if we touch the root, all things are rebuilt
    fn all_packages_are_rebuilt() {
        let graph = make_diamond_graph();

        let touched: Vec<PackageIdentIntern> = vec![ident_intern!("a/top")];
        let unbuildable = UnbuildableMock { unbuildable_packages: Vec::new(), };

        let manifest = graph.compute_build(&touched,
                                           &unbuildable,
                                           PackageTarget::from_str("x86_64-linux").unwrap());
        assert_eq!(manifest.input_set.len(), 1);
        assert_eq!(manifest.unbuildable_reasons.len(), 0);
        assert_eq!(manifest.graph.node_count(), 4);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/top", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/left", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/right", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/bottom", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("zz/top", 1)), false);
    }
    #[test]
    // Starting with a diamond graph, if we touch the root and one corner is not buildable,
    // the corner and bottom are not rebuilt and are correctly listed as unbuildable
    fn most_packages_are_rebuilt() {
        let graph = make_diamond_graph();

        let touched: Vec<PackageIdentIntern> = ident_intern_vec!("a/top");
        let unbuildable = UnbuildableMock { unbuildable_packages: ident_intern_vec!("a/left"), };

        let manifest = graph.compute_build(&touched,
                                           &unbuildable,
                                           PackageTarget::from_str("x86_64-linux").unwrap());
        assert_eq!(manifest.input_set.len(), 1);
        assert_eq!(manifest.unbuildable_reasons.len(), 2);
        assert_eq!(manifest.unbuildable_reasons[&ident_intern!("a/left")],
                   UnbuildableReason::Direct);
        assert_eq!(manifest.unbuildable_reasons[&ident_intern!("a/bottom")],
                   UnbuildableReason::Indirect);

        assert_eq!(manifest.graph.node_count(), 2);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/right", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/top", 1)), true);
    }
    #[test]
    // Starting with a diamond graph, if we touch one corner, the corner and bottom are rebuilt.
    fn some_packages_are_rebuilt() {
        let graph = make_diamond_graph();

        let touched: Vec<PackageIdentIntern> = ident_intern_vec!("a/right");
        let unbuildable = UnbuildableMock { unbuildable_packages: Vec::new(), };

        let manifest = graph.compute_build(&touched,
                                           &unbuildable,
                                           PackageTarget::from_str("x86_64-linux").unwrap());
        assert_eq!(manifest.input_set.len(), 1);
        assert_eq!(manifest.unbuildable_reasons.len(), 0);

        assert_eq!(manifest.graph.node_count(), 4);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/right", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/bottom", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/top")), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/left")), true);
    }

    // Starting with a diamond graph that has dependencies, if we touch the root, all dependencies
    // are listed.
    #[test]
    fn dependencies_are_represented() {
        let packages = vec![make_package("a/top/c/d", &["core/apple/c/d"]),
                            make_package("a/left/c/d", &["a/top/c/d", "core/frob/c/d"]),
                            make_package("a/right/c/d", &["a/top/c/d"]),
                            make_package("a/bottom/c/d", &["a/left/c/d", "a/right/c/d"]),];
        let mut graph = AcyclicPackageGraph::new();
        graph.build(&packages, true);

        let touched: Vec<PackageIdentIntern> = ident_intern_vec!("a/left");
        let unbuildable = UnbuildableMock { unbuildable_packages: Vec::new(), };

        let manifest = graph.compute_build(&touched,
                                           &unbuildable,
                                           PackageTarget::from_str("x86_64-linux").unwrap());
        assert_eq!(manifest.input_set.len(), 1);
        assert_eq!(manifest.unbuildable_reasons.len(), 0);

        assert_eq!(manifest.graph.node_count(), 5);

        assert_eq!(manifest.graph.contains_node(mk_ELV("a/top")), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("core/frob")), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("core/apple")), false);

        assert_eq!(manifest.external_dependencies.len(), 3);
        assert_eq!(manifest.external_dependencies
                           .contains(&ident_intern!("a/top")),
                   true);
        assert_eq!(manifest.external_dependencies
                           .contains(&ident_intern!("a/right")),
                   true);
        assert_eq!(manifest.external_dependencies
                           .contains(&ident_intern!("core/frob")),
                   true);
    }

    // Starting with a diamond graph, if our touched set includes things not in the graph,
    // they are correctly listed as missing.
    #[test]
    fn missing_packages() {
        let graph = make_diamond_graph();

        let touched: Vec<PackageIdentIntern> = ident_intern_vec!("a/right", "zz/top");
        let unbuildable = UnbuildableMock { unbuildable_packages: Vec::new(), };

        let manifest = graph.compute_build(&touched,
                                           &unbuildable,
                                           PackageTarget::from_str("x86_64-linux").unwrap());
        assert_eq!(manifest.input_set.len(), 2);

        assert_eq!(manifest.unbuildable_reasons.len(), 0);

        assert_eq!(manifest.graph.node_count(), 5);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/right", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/bottom", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/top")), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/left")), true);

        // TBD: does this belong here? Also should check the dependencies of ZZ top; it should stand
        // alone
        println!("MANIFEST:\n{:?}\n", manifest);
        assert_eq!(manifest.graph.contains_node(mk_IN("zz/top", 1)), true);
    }
}
