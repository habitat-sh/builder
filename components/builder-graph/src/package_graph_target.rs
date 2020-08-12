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

use std::{collections::{HashMap,
                        HashSet},
          fmt,
          fs::File,
          io::prelude::*};

use petgraph::{algo::{connected_components,
                      is_cyclic_directed},
               graphmap::DiGraphMap,
               Direction};

use habitat_builder_db::models::package::PackageWithVersionArray;

use crate::hab_core::package::{ident::Identifiable,
                               PackageIdent,
                               PackageTarget};

use crate::{data_store::Unbuildable,
            graph_helpers,
            package_build_manifest_graph::PackageBuild,
            package_ident_intern::{display_ordering_cmp,
                                   PackageIdentIntern},
            package_info::PackageInfo,
            util::*};

#[derive(Debug)]
pub struct Stats {
    pub node_count:     usize,
    pub edge_count:     usize,
    pub connected_comp: usize,
    pub is_cyclic:      bool,
}

pub struct PackageGraphForTarget {
    target: PackageTarget,

    packages: HashMap<PackageIdentIntern, PackageInfo>,

    // Map from truncated ident to latest matching; it could be origin/packagename, or
    // origin/packagename/version. Logically this a map from a partially qualified ident
    // to a fully qualified one. Possibly extract this as separate entity.
    latest_map: HashMap<PackageIdentIntern, PackageIdentIntern>,

    // We build this alongside the full graph
    latest_graph: DiGraphMap<PackageIdentIntern, EdgeType>,
}

impl PackageGraphForTarget {
    pub fn new(target: PackageTarget) -> Self {
        PackageGraphForTarget { target,
                                packages: HashMap::<PackageIdentIntern, PackageInfo>::new(),
                                latest_map:
                                    HashMap::<PackageIdentIntern, PackageIdentIntern>::new(),
                                latest_graph: DiGraphMap::<PackageIdentIntern, EdgeType>::new() }
    }

    // This is currently only used for testing, but it is part of the original API and left for
    // ease of adaptation/backfit.
    pub fn build<T>(&mut self, packages: T, use_build_deps: bool) -> (usize, usize)
        where T: Iterator<Item = PackageInfo>
    {
        for p in packages {
            self.extend(&p, use_build_deps);
        }

        (self.latest_graph.node_count(), self.latest_graph.edge_count())
    }

    fn update_latest(&mut self, id: PackageIdentIntern) {
        // We should check if this is fully qualified, as we implictly assume that to be the case
        let just_package = id.short_ident();
        self.update_if_newer(just_package, id);

        let package_version = id.versioned_ident(); // technically this might fail if we already have a short ident!!!
        self.update_if_newer(package_version, id);
    }

    fn update_if_newer(&mut self, id: PackageIdentIntern, fqpi: PackageIdentIntern) {
        match self.latest_map.get(&id) {
            Some(&old_fqpi) => {
                if fqpi > old_fqpi {
                    self.latest_map.insert(id, fqpi);
                }
            }
            None => {
                self.latest_map.insert(id, fqpi);
            }
        };
    }

    // Incrementally adds a node to the graph, rejecting it and doing nothing
    // if it returns a cycle.
    // Returns current node, edge count of graph.
    pub fn extend(&mut self, package_info: &PackageInfo, use_build_deps: bool) -> (usize, usize) {
        debug!("Extend: {} {} N:{} E:{} P:{}",
               package_info.ident,
               package_info.target,
               self.latest_graph.node_count(),
               self.latest_graph.edge_count(),
               self.packages.len());

        // TODO make plan for removing package_info structs from package table when they are no
        // longer part of graph Right now we keep them forever, which is unnecessary.
        let package_ident = PackageIdentIntern::from(&package_info.ident);

        let short_ident = package_ident.short_ident();

        // Next, add to latest graph. We overwrite any prior ident, so that any incoming
        // dependencies are preserved
        // Are we the newest? Ignore older versions of the package
        if !self.latest_map.contains_key(&short_ident)
           || self.latest_map[&short_ident] <= package_ident
        {
            // we will need to be checking for cycles here...
            // List current node in graph outgoing runtime (rt) deps
            // Compare to new package rt deps
            // if rt deps added, then do cycle check
            // if rt deps deleted, then smarter cycle check algos might need to do something, but
            // not us now just delete the edge.
            // if same, just no-op
            // secondary optimization; if no *incoming* rt deps we can skip cycle check as well.

            // skip fully qualified idents in the graph; they never will be rebuilt, so they only
            // add noise to the dependency graph.
            //
            // We also *could* skip partially qualified idents here. There are two cases to
            // consider: deps on a version that's not latest, which again won't be rebuilt. There's
            // a special case with some packages that bump versions in lockstep (gcc, gcc-libs) They
            // are version pinned, but always on latest. We should treat those as if they we're
            // unqualified. However at this time we don't have the proper information to know if
            // they are pointing at latest version or not.  For now we are building the graph
            // optimisically, and will need to check later if that is sane.
            let plan_deps = filter_out_fully_qualified(&package_info.plan_deps);

            let (added, deleted) = graph_helpers::changed_edges_for_type(&self.latest_graph,
                                                                         short_ident,
                                                                         &plan_deps,
                                                                         EdgeType::RuntimeDep);

            // 
            // * The graph invariant is that it is cycle free, so if we aren't adding any new edges,
            //   we can't add a cycle
            // * If this node doesn't have anyone depending on it, it can't be part of a cycle
            // * Otherwise, we have to search to see if we have created a cycle We only look at the
            //   added edges, since they are where a cycle might be introduced.
            // TODO: track some statistics on how often we advert a cycle check.
            let start = std::time::Instant::now();
            let has_cycle = graph_helpers::detect_cycles(&self.latest_graph, short_ident, &added);
            let cycle_detect_time = (start.elapsed().as_nanos() as f64) / 1_000_000_000.0;

            debug!("Detect cycle E ({}) for {} took {} s ({} edges)",
                   has_cycle,
                   short_ident,
                   cycle_detect_time,
                   plan_deps.len());

            if has_cycle {
                // Handle cycle case here
                debug!("Detect cycle for E {} found a cycle in {}s",
                       short_ident, cycle_detect_time);
                return (self.latest_graph.node_count(), self.latest_graph.edge_count());
            } else {
                // No cycle created, so
                graph_helpers::update_edges_for_type(&mut self.latest_graph,
                                                     short_ident,
                                                     &added,
                                                     &deleted,
                                                     EdgeType::RuntimeDep);
            }

            if use_build_deps {
                let plan_bdeps = filter_out_fully_qualified(&package_info.plan_bdeps);
                graph_helpers::revise_edges_for_type(&mut self.latest_graph,
                                                     short_ident,
                                                     &plan_bdeps,
                                                     EdgeType::BuildDep);

                // Long term, strong build deps should be integrated into our notion of the plan. In
                // the intermediate term this probably needs to be stored in the
                // database along with the package info However, for now, we're
                // hydrating it from a hardcoded set specific to core plans.
                let plan_sdeps = filter_out_fully_qualified(&package_info.strong_bdeps);
                graph_helpers::revise_edges_for_type(&mut self.latest_graph,
                                                     short_ident,
                                                     &plan_sdeps,
                                                     EdgeType::StrongBuildDep);
            }
        }
        self.update_latest(package_ident);
        self.packages.insert(package_ident, package_info.clone());

        debug!("Extend: {} {} N:{} E:{} P:{}",
               package_ident,
               package_info.target,
               self.latest_graph.node_count(),
               self.latest_graph.edge_count(),
               self.packages.len());
        (self.latest_graph.node_count(), self.latest_graph.edge_count())
    }

    // This is basically extend above, but only checks, doesn't update.
    // The embarassing levels of parallel construction between the two should be cleaned up and
    // unified
    // Returns true if we can add this w/o a cycle
    pub fn check_extend(&self, package_info: &PackageInfo, _use_build_deps: bool) -> bool {
        // TODO make plan for removing package_info structs from package table when they are no
        // longer part of graph Right now we keep them forever, which is unnecessary.
        let package_ident = PackageIdentIntern::from(&package_info.ident);

        let short_ident = package_ident.short_ident();
        // Next, add to latest graph. We overwrite any prior ident, so that any incoming
        // dependencies are preserved
        // Are we the newest? Ignore older versions of the package
        if !self.latest_map.contains_key(&short_ident)
           || self.latest_map[&short_ident] <= package_ident
        {
            let plan_deps = filter_out_fully_qualified(&package_info.plan_deps);

            let (added, _deleted) = graph_helpers::changed_edges_for_type(&self.latest_graph,
                                                                          short_ident,
                                                                          &plan_deps,
                                                                          EdgeType::RuntimeDep);

            // 
            // * The graph invariant is that it is cycle free, so if we aren't adding any new edges,
            //   we can't add a cycle
            // * If this node doesn't have anyone depending on it, it can't be part of a cycle
            // * Otherwise, we have to search to see if we have created a cycle We only look at the
            //   added edges, since they are where a cycle might be introduced.
            // TODO: track some statistics on how often we advert a cycle check.
            // TODO: examine whether the reverse dep scan is faster (most nodes are leaf nodes)
            let start = std::time::Instant::now();
            let has_cycle = graph_helpers::detect_cycles(&self.latest_graph, short_ident, &added);
            let cycle_detect_time = (start.elapsed().as_nanos() as f64) / 1_000_000_000.0;

            debug!("Detect cycle CE ({}) for {} took {} s ({} edges)",
                   has_cycle,
                   short_ident,
                   cycle_detect_time,
                   plan_deps.len());

            if has_cycle {
                // Handle cycle case here
                return false;
            }
            // NOTE:
            // At some point we should be checking the global graph for build cycles across origins
        }

        true
    }

    pub fn write_packages_json(&self, filename: &str, filter: Option<&str>) {
        let mut output: Vec<PackageWithVersionArray> = Vec::new();
        let mut keep = 0;
        let mut m = 0;
        for package_ref in self.packages.values() {
            if filter_match(&package_ref.ident, filter) {
                m += 1;
                if let Some(p) = &package_ref.package {
                    keep += 1;
                    output.push(p.clone())
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

    pub fn read_packages_json(&mut self, filename: &str, use_build_edges: bool) {
        let packages = read_packages_json(filename);
        for package in packages {
            let package_info = PackageInfo::from(package.clone());
            self.extend(&package_info, use_build_edges);
        }
    }

    pub fn rdeps(&self, name: PackageIdentIntern, origin: Option<&str>) -> Vec<PackageIdentIntern> {
        let seed = vec![name];
        graph_helpers::flood_deps_in_origin(&self.latest_graph, &seed, origin)
    }

    // Mostly for debugging
    pub fn rdeps_dump(&self) {
        debug!("Reverse dependencies:");
        unimplemented!("Rdeps aren't a thing right now, come back later");
    }

    pub fn search(&self, _phrase: &str) -> Vec<String> {
        unimplemented!("Search isn't a thing right now, come back later");
        // TODO: Rework this for new PackageTable construct
        //      let v: Vec<String> = self
        //     .packages
        //     .values()
        //     .map(|package| format!("{}", package.borrow().ident))
        //     .filter(|s| s.contains(phrase))
        //     .collect();
        // v
    }

    pub fn latest(&self) -> Vec<String> {
        self.latest_map.values().map(|x| format!("{}", x)).collect()
    }

    // Given an identifier in 'origin/name' format, returns the
    // most recent version (fully-qualified package ident string)
    pub fn resolve(&self, ident: &PackageIdent) -> Option<PackageIdent> {
        let ident = PackageIdentIntern::from(ident);
        self.latest_map.get(&ident).map(|x| (*x).into())
    }

    pub fn stats(&self) -> Stats {
        Stats { node_count:     self.latest_graph.node_count(),
                edge_count:     self.latest_graph.edge_count(),
                connected_comp: connected_components(&self.latest_graph),
                is_cyclic:      is_cyclic_directed(&self.latest_graph), }
    }

    // Who has the most things depending on them?
    pub fn top(&self, _max: usize) -> Vec<(String, usize)> {
        unimplemented!("Top isn't a thing right now, come back later");
        // TODO REIMPLEMENT IN NEW WORLD;
    }

    // Takes a initial list of package idents and expands their deps; then permutes this
    // to produce a map of dep with the list of each item in the initial set that required it.
    // Optionally follows build time dep edges as well.
    //
    pub fn compute_attributed_deps(&self,
                                   idents: &[PackageIdentIntern],
                                   include_build_deps: bool)
                                   -> HashMap<PackageIdentIntern, Vec<PackageIdentIntern>> {
        let mut acc: HashMap<PackageIdentIntern, HashSet<PackageIdentIntern>> = HashMap::new();
        for ident in idents {
            let deps = graph_helpers::transitive_deps(&self.latest_graph,
                                                      &[*ident],
                                                      None,
                                                      include_build_deps);
            for dep in deps {
                acc.entry(dep)
                   .and_modify(|e| {
                       (*e).insert(*ident);
                   })
                   .or_insert_with(|| {
                       let mut s = HashSet::new();
                       s.insert(*ident);
                       s
                   });
            }
        }
        let mut results: HashMap<PackageIdentIntern, Vec<PackageIdentIntern>> = HashMap::new();

        for dep in acc.keys() {
            let mut r: Vec<PackageIdentIntern> = acc[dep].iter()
                                                         .cloned()
                                                         .collect::<Vec<PackageIdentIntern>>();
            r.sort_by(display_ordering_cmp);
            results.insert(*dep, r);
        }
        results
    }

    pub fn write_deps(&self, ident: &PackageIdent) {
        let ident = PackageIdentIntern::from(ident);
        let full_ident = if ident.fully_qualified() {
            Some(ident)
        } else {
            self.latest_map.get(&ident).cloned()
        };
        let maybe_package = full_ident.and_then(|pi| self.packages.get(&pi));

        match maybe_package {
            Some(pkg) => pkg.write(),
            None => println!("Couldn't find match for {}", ident),
        }
    }

    pub fn dump_graph(&self, _file: &str) {
        unimplemented!("dump_graph unimplemented");
    }

    pub fn dump_latest_graph_raw_h<T>(&self, file: &str, p: &T)
        where T: fmt::Display
    {
        let filename = format!("{}_{}", file, p).replace("/", "_");
        self.dump_latest_graph_raw(filename.as_str(), None)
    }

    // Output a human readable, machine parsable form of the graph; useful for debugging
    pub fn dump_latest_graph_raw(&self, file: &str, origin: Option<&str>) {
        graph_helpers::dump_graph_raw(&self.latest_graph, file, origin)
    }

    // The built in Dot utility wasn't flexible for what I wanted, so implemented our own.
    pub fn dump_latest_graph_as_dot(&self, file: &str, origin: Option<&str>) {
        graph_helpers::emit_graph_as_dot(&self.latest_graph, file, origin)
    }

    pub fn dump_build_levels(&self, _file: &str, _origin: Option<&str>) {
        unimplemented!("Isn't a thing right now, come back later");
        // self.latest_graph.dump_build_levels(file, origin)
    }

    pub fn dump_scc(&self, file: &str, origin: Option<&str>) {
        graph_helpers::dump_scc(&self.latest_graph, file, origin)
    }

    pub fn dump_diagnostics(&self, file: &str, _origin: Option<&str>) {
        let mut _file = std::fs::File::create(file).unwrap();
        unimplemented!("Isn't a thing right now, come back later");
    }

    pub fn dump_build_ordering(&mut self,
                               unbuildable: &dyn Unbuildable,
                               _filename: &str,
                               origin: &str,
                               base_set: &[PackageIdent],
                               touched: &[PackageIdent])
                               -> Vec<PackageBuild> {
        self.compute_build(unbuildable, origin, base_set, touched, 3)
    }

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
                         unbuildable: &dyn Unbuildable,
                         origin: &str,
                         base_set: &[PackageIdent],
                         touched: &[PackageIdent],
                         converge_count: usize)
                         -> Vec<PackageBuild> {
        // debug!("Using base: {} {}\n",
        // base_set.len(),
        // join_idents(", ", &base_set));

        debug!("Using touched: {} {}\n",
               touched.len(),
               join_idents(", ", &touched));

        let preconditioned_graph = self.precondition_graph(origin);

        let touched: Vec<PackageIdentIntern> = touched.iter().map(|x| x.into()).collect();
        let rebuild_set = graph_helpers::compute_rebuild_set(&preconditioned_graph,
                                                             unbuildable,
                                                             &touched,
                                                             origin,
                                                             self.target);

        // TODO DO check of rebuild set to make sure that it includes the pinned versions that had
        // edges added in the precondition_graph phase above.

        debug!("Rebuild: {} {}\n",
               rebuild_set.len(),
               join_idents(", ", &rebuild_set));

        let build_order = graph_helpers::compute_build_order(&preconditioned_graph, &rebuild_set);
        // Rework this later
        debug!("CB: {} components", build_order.len());
        for component in &build_order {
            debug!("CB: #{} {}", component.len(), join_idents(", ", component));
        }

        let mut latest = HashMap::<PackageIdent, PackageIdent>::new();
        for ident in base_set {
            latest.insert(short_ident(&ident, false), ident.clone());
        }

        let mut file = File::create("latest_from_base.txt").expect("Failed to initialize file");
        for (k, v) in &latest {
            file.write_all(format!("{}: {}\n", &k, &v).as_bytes())
                .unwrap();
        }

        let mut built: Vec<PackageBuild> = Vec::new();
        for component in build_order.iter() {
            // If there is only one element in component, don't need to converge, can just run
            // once
            let component_converge_count = if component.len() > 1 {
                converge_count
            } else {
                1
            };

            for _i in 1..=component_converge_count {
                for &ident in component {
                    let ident: PackageIdentIntern = ident;
                    let ident_latest = self.latest_map[&ident];
                    let package =
                        self.packages.get(&ident_latest).unwrap_or_else(|| {
                                                            panic!("Expected to find package for \
                                                                    {} {} iter {}",
                                                                   ident_latest, ident, _i)
                                                        });
                    let build = build_package(package, &mut latest);
                    latest.insert(short_ident(&build.ident, false), build.ident.clone());
                    built.push(build);
                }
            }
        }

        built
    }

    // Precondition Graph
    //
    // The graph is built incrementally, and we may not be able to fixup things because we
    // lack full knowledge at the point of insertion.
    // In particular, we have to treat edges with version information specially. If we depend
    // on a particular version of a package, and the version is the latest, then we will rebuild it
    // if the package rebuilds. If we depend on an older version, we will not rebuild it unless
    // a new release of that version is uploaded (or if we modify builder to build old versions)
    // So we fixup the graph here to represent that
    pub fn precondition_graph(&self, origin: &str) -> DiGraphMap<PackageIdentIntern, EdgeType> {
        let mut graph: DiGraphMap<PackageIdentIntern, EdgeType> = DiGraphMap::new();
        for node in self.latest_graph.nodes() {
            if self.node_filter_helper(Some(origin), node) {
                graph.add_node(node);
            }
        }
        for (src, dst, edge) in self.latest_graph.all_edges() {
            if graph.contains_node(src) && graph.contains_node(dst) {
                if dst.version().is_some() {
                    let short_dst = dst.short_ident();
                    if let Some(latest) = self.latest_map.get(&short_dst) {
                        if latest.version() <= dst.version() {
                            // If we are pointing to the latest version, it's treated just as if
                            // we were using an short ident. Otherwise we ignore this as it will
                            // never trigger a rebuild.
                            // We choose <= just in case we have a race condition w
                            graph.add_edge(src, short_dst, *edge);
                        }
                    } else {
                        // This is an interesting subcase.
                        // Here we have a dependency on a package *we've never seen* (because
                        // it's not in latest_map)
                        // That's serious breakage, as we can't build anything that depends on
                        // it.assert_eq! we're going to put it in the graph
                        // to make it visible but not fail, because it might
                        // be worth doing a partial build. A open question is how to best
                        // communicate to users that a package is
                        // unbuildable because of missing dependencies.
                        graph.add_edge(src, short_dst, *edge);
                    }
                } else {
                    graph.add_edge(src, dst, *edge);
                }
            }
        }

        graph
    }

    // We keep the node if it either is in the origin, or if it is directly depended on by a node in
    // the origin
    //
    pub fn node_filter_helper(&self, origin: Option<&str>, node: PackageIdentIntern) -> bool {
        if filter_match(&node, origin) {
            true
        } else {
            for pred in self.latest_graph
                            .neighbors_directed(node, Direction::Incoming)
            {
                if filter_match(&pred, origin) {
                    return true;
                }
            }
            false
        }
    }
}

// While parameterizing over a hasher is a nice thing for a library, this code is very specialized,
// and we won't be using anything beyond the standard PackageIdent hasher here.
// https://rust-lang.github.io/rust-clippy/master/index.html#implicit_hasher
#[allow(clippy::implicit_hasher)]
pub fn build_package(package: &PackageInfo,
                     latest: &mut HashMap<PackageIdent, PackageIdent>)
                     -> PackageBuild {
    // Create our package name
    let ident = make_temp_ident(&package.ident);

    // resolve our runtime and build deps
    let mut bt_deps = Vec::new();
    let mut rt_deps = Vec::new();

    for dep in &package.plan_bdeps {
        // Horrible hack to get around our own pinning
        let sdep = short_ident(dep, false);
        bt_deps.push(latest.get(&sdep)
                           .unwrap_or_else(|| {
                               panic!("{} Unable to find bt dep {} ({})", &ident, &dep, &sdep)
                           })
                           .clone())
    }
    for dep in &package.plan_deps {
        // Horrible hack to get around our own pinning
        let sdep = short_ident(dep, false);
        rt_deps.push(latest.get(&sdep)
                           .unwrap_or_else(|| {
                               panic!("{} Unable to find rt dep {} ({})", &ident, &dep, &sdep)
                           })
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

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    const TGT: &str = "x86_64-linux";
    const EMPTY: [&str; 0] = [];
    #[test]
    #[ignore] //  This is probably broken by the changes to serialization
    fn write_restore_packages() {
        let mut packages = Vec::new();

        let package1 = PackageInfo::mk("foo/bar/1/2", TGT, &["foo/baz/1/2"], &EMPTY, &EMPTY);
        let package2 = PackageInfo::mk("foo/baz/1/2", TGT, &["foo/bat/1/2"], &EMPTY, &EMPTY);

        packages.push(package1);
        packages.push(package2);

        let tmpfile = "/tmp/junk"; // Do this smarter
        let mut graph = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());
        graph.build(packages.into_iter(), true);
        let stats = graph.stats();
        assert_eq!(stats.node_count, 2);

        graph.write_packages_json(tmpfile, None);

        let mut graph2 = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());
        graph2.read_packages_json(tmpfile, true);
        let stats = graph2.stats();
        assert_eq!(stats.node_count, 2);
    }

    // we can create a simple graph
    #[test]
    fn pre_check_with_dep_not_present() {
        let mut graph =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        let empty: [&str; 0] = [];

        let package1 = PackageInfo::mk("foo/bar/1/2", "x86_64-linux", &["foo/baz"], &empty, &empty);

        let package2 = PackageInfo::mk("foo/baz/1/2", "x86_64-linux", &["foo/xyz"], &empty, &empty);

        let pre_check1 = graph.check_extend(&package1, true);
        assert_eq!(pre_check1, true);

        let (ncount, ecount) = graph.extend(&package1, true);
        assert_eq!(ncount, 2);
        assert_eq!(ecount, 1);

        let pre_check2 = graph.check_extend(&package2, true);
        assert_eq!(pre_check2, true);

        let (ncount, ecount) = graph.extend(&package2, true);
        assert_eq!(ncount, 3);
        assert_eq!(ecount, 2);
    }

    // A run time circular dependency is forbidden, and should not change the graph if attempted.
    #[test]
    fn disallow_circular_dependency() {
        let mut graph = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());
        let mut packages = Vec::new();

        let package1 = PackageInfo::mk("foo/bar/1/2", TGT, &["foo/baz"], &EMPTY, &EMPTY);
        let package2 = PackageInfo::mk("foo/baz/1/2", TGT, &["foo/bar"], &EMPTY, &EMPTY);
        packages.push(package1);

        let (ncount, ecount) = graph.build(packages.into_iter(), true);

        // Both nodes will be present in the graph, but only the first edge should exist.
        // The first package created will create nodes for all of its declared dependencies.
        // The second node added will already exist, but the edge back to the first node
        // should not be created as this will cause a cycle.
        assert_eq!(ncount, 2);
        assert_eq!(ecount, 1);

        let stats = graph.stats();
        assert_eq!(stats.is_cyclic, false);

        // check extend should reject a cycle
        let pre_check = graph.check_extend(&package2, true);
        assert_eq!(pre_check, false);

        let (ncount, ecount) = graph.extend(&package2, true);
        // We shouldn't add any edges for a circular dependency
        assert_eq!(ncount, 2);
        assert_eq!(ecount, 1);
    }

    // A build time circular dependency is ok
    #[test]
    fn allow_circular_build_dependency() {
        let mut graph = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());
        let mut packages = Vec::new();

        let package1 = PackageInfo::mk("foo/bar/1/2", TGT, &["foo/baz"], &EMPTY, &EMPTY);
        let package2 = PackageInfo::mk("foo/baz/1/2", TGT, &EMPTY, &["foo/bar"], &EMPTY);
        packages.push(package1);

        let (ncount, ecount) = graph.build(packages.into_iter(), true);

        assert_eq!(ncount, 2);
        assert_eq!(ecount, 1);

        let stats = graph.stats();
        assert_eq!(stats.is_cyclic, false);

        // check extend should allow a cycle for a build dep
        let pre_check = graph.check_extend(&package2, true);
        assert_eq!(pre_check, true);

        let (ncount, ecount) = graph.extend(&package2, true);
        // We should see the edges including the circular dependency
        assert_eq!(ncount, 2);
        assert_eq!(ecount, 2);
    }

    // Test that updated nodes with removed edges do the right thing
    #[test]
    fn updates_remove_edges_correctly() {
        let mut graph = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());

        let package1v1 = PackageInfo::mk("foo/bar/1/2", TGT, &["foo/baz"], &EMPTY, &EMPTY);
        let package1v2 = PackageInfo::mk("foo/bar/2/2", TGT, &EMPTY, &EMPTY, &EMPTY);

        let package2 = PackageInfo::mk("foo/baz/1/2", TGT, &["foo/bar"], &EMPTY, &EMPTY);

        let (ncount, ecount) = graph.extend(&package1v1, true);
        assert_eq!(ncount, 2);
        assert_eq!(ecount, 1);

        // We reject adding a runtime dep
        assert_eq!(false, graph.check_extend(&package2, true));

        // update the package
        let (ncount, ecount) = graph.extend(&package1v2, true);
        assert_eq!(ncount, 2);
        assert_eq!(ecount, 0);

        // We allow adding a runtime dep, once the cycle is removed
        assert_eq!(true, graph.check_extend(&package2, true));

        let (ncount, ecount) = graph.extend(&package2, true);
        assert_eq!(ncount, 2);
        assert_eq!(ecount, 1);
    }

    fn extend_variant_helper(graph: &mut PackageGraphForTarget,
                             package: &PackageInfo,
                             success_expected: bool,
                             node_delta: i64,
                             edge_delta: i64) {
        let Stats { node_count: ncount,
                    edge_count: ecount,
                    .. } = graph.stats();
        assert_eq!(success_expected, graph.check_extend(&package, true));

        // assert_graph_extend!(graph, pkg_info, expected_status, expected_node_count,
        // expected_edge_count);

        let (new_ncount, new_ecount) = graph.extend(&package, true);
        if !success_expected {
            assert_eq!(ncount, new_ncount);
            assert_eq!(ecount, new_ecount);
        }

        assert_eq!(node_delta,
                   (new_ncount as i64) - (ncount as i64),
                   "Node expected delta not equal to actual");
        assert_eq!(edge_delta,
                   (new_ecount as i64) - (ecount as i64),
                   "Edge expected delta not equal to actual");
    }

    // test for long cycles
    #[test]
    fn longer_cycles_are_spotted() {
        let mut graph = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());

        let packages = vec![PackageInfo::mk("foo/c1/1/2", TGT, &["foo/c2"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c2/1/2", TGT, &["foo/c3"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c3/2/2", TGT, &["foo/c4"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c4/2/2", TGT, &["foo/c5"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c5/2/2", TGT, &["foo/c6"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c6/2/2", TGT, &["foo/c7"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c7/2/2", TGT, &["foo/c1"], &EMPTY, &EMPTY),];

        extend_variant_helper(&mut graph, &packages[0], true, 2, 1);
        extend_variant_helper(&mut graph, &packages[1], true, 1, 1);
        extend_variant_helper(&mut graph, &packages[2], true, 1, 1);
        extend_variant_helper(&mut graph, &packages[3], true, 1, 1);
        extend_variant_helper(&mut graph, &packages[4], true, 1, 1);
        extend_variant_helper(&mut graph, &packages[5], true, 1, 1);
        extend_variant_helper(&mut graph, &packages[6], false, 0, 0);
    }

    // test for pinned deps
    #[test]
    fn pinned_deps_are_ignored() {
        let mut graph = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());

        let packages = vec![PackageInfo::mk("foo/c1/1/2", TGT, &["foo/c2"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c2/1/2", TGT, &["foo/c1/1/2"], &EMPTY, &EMPTY),];

        extend_variant_helper(&mut graph, &packages[0], true, 2, 1);
        extend_variant_helper(&mut graph, &packages[1], true, 0, 0);
    }

    // This test the currently implmented behaviour, but this might need to change.
    // While we ignore fully qualified deps, we have to track (for now) partially qualified deps.
    // A common idiom is to have a pair of packages (say A & B) with a dependency from A to B where
    // B must have a particular version. Most commonly, this happens when two packages bump
    // version in lockstep and must have the same version number.
    // If A points to the latest version of B, we treat this as an edge to B for build purposes, and
    // otherwise ignore it, as currently we don't rebuild older versions.
    // It might be nice to track this in the graph, but we might not have full visibility into
    // whether it is the latest during an incremental build process. So we track it for now, and
    // fix up in the build phase. Note the build phase should explicitly test this!
    #[test]
    fn version_pinned_deps_are_ignored() {
        let mut graph = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());

        let packages = vec![PackageInfo::mk("foo/c1/0/2", TGT, &["foo/c2"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c1/1/2", TGT, &["foo/c2"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c2/1/2", TGT, &["foo/c1/0"], &EMPTY, &EMPTY),
                            PackageInfo::mk("foo/c2/2/2", TGT, &["foo/c1/1"], &EMPTY, &EMPTY),];

        extend_variant_helper(&mut graph, &packages[0], true, 2, 1);
        extend_variant_helper(&mut graph, &packages[1], true, 0, 0);

        // older version pin not rejected, but changes nothing
        extend_variant_helper(&mut graph, &packages[2], true, 1, 1);
        // newer version pin allowed as latest.
        extend_variant_helper(&mut graph, &packages[3], true, 1, 0);
    }
    // TODO:

    // ghost nodes? (nodes that we've not seen package/plan info for)
}
