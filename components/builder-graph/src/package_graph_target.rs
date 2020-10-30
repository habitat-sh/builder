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
          iter::FromIterator};

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
            package_build_manifest_graph::{PackageBuildManifest,
                                           UnresolvedPackageIdent},
            package_graph_trait::Stats,
            package_ident_intern::{display_ordering_cmp,
                                   PackageIdentIntern},
            package_info::PackageInfo,
            util::*};

// How many times we cycle around the loop in a cycle before declaring it converged
// This is based on cultural lore from compiler bootstrapping, where generally
// you rebuild a compiler three times on itself to wring out bugs. You need more than once, so that
// your new build tools are used to build what you ship. So it should be at least two. The third
// round is to catch subtle bugs that only manifest when you you build with yourself. Very
// occasionally bugs of this sort do manifest. More is probably pointless, as a bug subtle enough to
// only manifest past the third iteration is *extremely* low probability.
const CYCLIC_BUILD_CONVERGE_COUNT: usize = 3;

#[derive(Debug)]
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
    #[tracing::instrument(skip(self))]
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
            tracing::event!(tracing::Level::DEBUG,
                            "Updating plan ident {} with {}",
                            short_ident,
                            package_ident);
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
            // are version pinned, but always on latest. We should treat those as if they're
            // unqualified. However at this time we don't have the proper information to know if
            // they are pointing at latest version or not.  For now we are building the graph
            // optimistically, and will need to check later if that is sane.
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
    #[tracing::instrument(skip(self))]
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

    #[tracing::instrument(skip(self))]
    pub fn as_json(&self) -> String {
        let graph = graph_helpers::dump_graph_structured(&self.latest_graph, None, false);
        debug!("Dump Graph Structured for (N:{} E:{} {}) {} elements",
               self.latest_graph.node_count(),
               self.latest_graph.edge_count(),
               self.target,
               graph.len());
        serde_json::to_string(&graph).unwrap()
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

    pub fn rdeps(&self,
                 name: PackageIdentIntern,
                 origin: Option<&str>)
                 -> Vec<(PackageIdentIntern, PackageIdentIntern)> {
        let seed = vec![name];
        let deps = graph_helpers::flood_deps_in_origin(&self.latest_graph, &seed, origin);
        deps.iter()
            .map(|&dep| {
                let fq_dep: PackageIdentIntern = *(self.latest_map.get(&dep).unwrap_or(&dep));
                (dep, fq_dep)
            })
            .collect()
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
            None => warn!("Couldn't find match for {}", ident),
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
                               touched: &[PackageIdentIntern])
                               -> PackageBuildManifest {
        self.compute_build(touched, unbuildable)
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

    // Thoughts on refining this for future work
    // Maybe we return a smarter structure (PackageBuildManifest) than Vec<PackageBuild>
    // It may be worth making it a graph internally.
    // Then unbuildable could be factor out of the function and make something that gets applied to
    // the PackageBuildManifest Base set could be computed on the fly and returned in that
    // structure We may not want to filter by origin yet.
    // Remaining signature would be (touched, unbuildable) -> PackageBuildManifest
    //
    pub fn compute_build(&self,
                         touched: &[PackageIdentIntern],
                         unbuildable: &dyn Unbuildable)
                         -> PackageBuildManifest {
        // In the future we will filter the graph to a rebuild a single origin, but that's a
        // potentially breaking change, so we're not filtering by origin today.
        let origin = None;

        info!("Compute Build, using touched: {} {}\n",
              touched.len(),
              join_idents(", ", &touched));

        // When we start restricting the builds to a single origin, we may need to rethink how we
        // compute/filter the graph. For example if we use touched to indicate things in core that
        // have been promoted, we want to propagate the updates fully before filtering the
        // graph to a single origin
        let mut preconditioned_graph = self.precondition_graph();

        // Some things in the touched set might be missing from the graph, but still buildable
        // (e.g first time build of a new plan) So we insert them here. Unbuildable will help us
        // determine which things lack a plan linkage and should be dropped.
        for &package in touched.iter() {
            preconditioned_graph.add_node(package);
        }

        let (rebuild_set, unbuildable_reasons) =
            graph_helpers::compute_rebuild_set(&preconditioned_graph,
                                               unbuildable,
                                               &touched,
                                               origin,
                                               self.target);

        // TODO DO check of rebuild set to make sure that it includes the pinned versions that had
        // edges added in the precondition_graph phase above.
        info!("Rebuild: {} {}\n",
              rebuild_set.len(),
              join_idents(", ", &rebuild_set));
        debug!("PRECOND GRAPH:\n{:#?}", preconditioned_graph);

        let build_order = graph_helpers::compute_build_order(&preconditioned_graph, &rebuild_set);
        // Rework this later
        debug!("CB: {} components", build_order.len());
        for component in &build_order {
            debug!("CB: #{} {}", component.len(), join_idents(", ", component));
        }
        debug!("BUILD ORDER:\n{:#?}", build_order);

        let mut latest = HashMap::<PackageIdentIntern, UnresolvedPackageIdent>::new();

        let mut build_graph: DiGraphMap<UnresolvedPackageIdent, EdgeType> = DiGraphMap::new();

        for component in build_order.iter() {
            // If there is only one element in component, don't need to converge, can just run
            // once
            let component_converge_count = if component.len() > 1 {
                CYCLIC_BUILD_CONVERGE_COUNT
            } else {
                1
            };

            for i in 1..=component_converge_count {
                for &ident in component {
                    let ident: PackageIdentIntern = ident;

                    let package_name = UnresolvedPackageIdent::InternalNode(ident, i as _);

                    let empty_package = &PackageInfo { ident:   ident.into(),
                                                       target:  self.target,
                                                       package: None,

                                                       no_deps:      false,
                                                       plan_deps:    Vec::new(),
                                                       plan_bdeps:   Vec::new(),
                                                       strong_bdeps: Vec::new(), };

                    let package = if self.latest_map.contains_key(&ident) {
                        let ident_latest = self.latest_map[&ident];
                        self.packages.get(&ident_latest).unwrap_or_else(|| {
                                                            panic!("Expected to find package for \
                                                                    {} {} iter {}",
                                                                   ident_latest, ident, i)
                                                        })
                    } else {
                        // We may have a package unseen previously, so construct a dummy PackageInfo
                        &empty_package
                    };

                    build_package(&mut build_graph, package, package_name, &mut latest);
                }
            }
        }

        let mut external_dependencies: HashSet<PackageIdentIntern> = HashSet::new();
        for package in build_graph.nodes() {
            match package {
                UnresolvedPackageIdent::ExternalLatestVersion(ident) => {
                    external_dependencies.insert(ident);
                }
                // pinned_verson/latest_release (cyclic graph might know enough to resolve)
                UnresolvedPackageIdent::ExternalPinnedVersion(ident) => {
                    external_dependencies.insert(ident);
                }
                //  pinned_version/pinned_release (cyclic graph might know enough to resolve)
                UnresolvedPackageIdent::ExternalFullyQualified(ident) => {
                    external_dependencies.insert(ident);
                }
                _ => (),
            }
        }

        // Forensics
        PackageBuildManifest { graph: build_graph,
                               external_dependencies,

                               input_set: HashSet::from_iter(touched.iter().copied()),
                               unbuildable_reasons }
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
    // This was originally written to filter the nodes to a single origin. But when we moved
    // resolution of external (not rebuilt) packages later, we needed to retain knowledge about
    // the full dependencies of a package, even if those dependencies weren't being rebuilt or
    // in the same origin.
    pub fn precondition_graph(&self) -> DiGraphMap<PackageIdentIntern, EdgeType> {
        let mut graph: DiGraphMap<PackageIdentIntern, EdgeType> = DiGraphMap::new();
        for node in self.latest_graph.nodes() {
            graph.add_node(node);
        }
        for (src, dst, edge) in self.latest_graph.all_edges() {
            // Both nodes have to be in the filtered graph to be relevant
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
pub fn build_package(graph: &mut DiGraphMap<UnresolvedPackageIdent, EdgeType>,
                     package: &PackageInfo,
                     package_name: UnresolvedPackageIdent,
                     latest: &mut HashMap<PackageIdentIntern, UnresolvedPackageIdent>) {
    graph.add_node(package_name);
    for dep in &package.plan_bdeps {
        let sdep_resolved = resolve_package(latest, dep.into());
        graph.add_edge(package_name, sdep_resolved, EdgeType::BuildDep);
    }
    for dep in &package.plan_deps {
        let sdep_resolved = resolve_package(latest, dep.into());
        graph.add_edge(package_name, sdep_resolved, EdgeType::RuntimeDep);
    }

    // update latest
    let short_name: PackageIdentIntern = (&package.ident).into();
    latest.insert(short_name.short_ident(), package_name);
}

pub fn resolve_package(latest: &mut HashMap<PackageIdentIntern, UnresolvedPackageIdent>,
                       dep: PackageIdentIntern)
                       -> UnresolvedPackageIdent {
    let sdep = dep.short_ident();

    let resolved_sdep = latest.entry(sdep).or_insert_with(|| {
                                              // TODO Does not handle pins yet
                                              UnresolvedPackageIdent::ExternalLatestVersion(sdep)
                                          });
    *resolved_sdep
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    use crate::package_build_manifest_graph::UnbuildableReason;

    const TGT: &str = "x86_64-linux";
    const EMPTY: [&str; 0] = [];

    fn mk_pkg(ident: &str, deps: &[&str], bdeps: &[&str], sdeps: &[&str]) -> PackageInfo {
        PackageInfo::mk(ident, TGT, deps, bdeps, sdeps)
    }

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
    // ghost nodes? (nodes that we've not seen package/plan info for) (plan connection, but no pkg
    // upload)

    ///////////////////////////////////////////////////////////////////////////
    // Test build graph
    //
    // TODO: Missing coverage around connectivity of graph; we are verifing presence/absence of
    // nodes but not order.
    //
    fn make_diamond_graph() -> PackageGraphForTarget {
        let packages = vec![mk_pkg("a/top/c/d", &[], &[], &[]),
                            mk_pkg("a/left/c/d", &["a/top"], &[], &[]),
                            mk_pkg("a/right/c/d", &["a/top"], &[], &[]),
                            mk_pkg("a/bottom/c/d", &["a/left", "a/right"], &[], &[])];
        let mut graph = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());
        graph.build(packages.into_iter(), true);
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

        let stats = graph.stats();
        assert_eq!(stats.node_count, 4);
        assert_eq!(stats.edge_count, 4);

        let touched: Vec<PackageIdentIntern> = vec![ident_intern!("a/top")];
        let unbuildable = UnbuildableMock { unbuildable_packages: Vec::new(), };

        let manifest = graph.compute_build(&touched, &unbuildable);

        println!("Manifest\n{:?}\n", manifest);

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

        let manifest = graph.compute_build(&touched, &unbuildable);
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

        let manifest = graph.compute_build(&touched, &unbuildable);
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
        let packages = vec![mk_pkg("a/top/c/d", &["core/apple"], &[], &[]),
                            mk_pkg("a/left/c/d", &["a/top", "core/frob"], &[], &[]),
                            mk_pkg("a/right/c/d", &["a/top"], &[], &[]),
                            mk_pkg("a/bottom/c/d", &["a/left", "a/right"], &[], &[]),];
        let mut graph =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        graph.build(packages.into_iter(), true);

        let touched: Vec<PackageIdentIntern> = ident_intern_vec!("a/left");
        let unbuildable = UnbuildableMock { unbuildable_packages: Vec::new(), };

        let manifest = graph.compute_build(&touched, &unbuildable);
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

        let manifest = graph.compute_build(&touched, &unbuildable);

        assert_eq!(manifest.input_set.len(), 2);

        assert_eq!(manifest.unbuildable_reasons.len(), 0);

        assert_eq!(manifest.graph.node_count(), 5);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/right", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/bottom", 1)), true);
        // we should check the dependencies of ZZ top; it should stand alone
        assert_eq!(manifest.graph.contains_node(mk_IN("zz/top", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/top")), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/left")), true);
    }

    fn make_circular_graph() -> PackageGraphForTarget {
        let packages = vec![mk_pkg("a/gcc/1/d", &["a/libgcc/1", "a/glibc"], &["a/make"], &[]),
                            mk_pkg("a/libgcc/1/d", &[], &["a/gcc/1", "a/make"], &[]),
                            mk_pkg("a/glibc/c/d", &[], &["a/gcc", "a/make"], &[]),
                            mk_pkg("a/make/c/d", &["a/glibc"], &[], &[]),
                            mk_pkg("a/out/c/d", &["a/glibc"], &["a/make", "a/gcc"], &[])];
        let mut graph = PackageGraphForTarget::new(PackageTarget::from_str(TGT).unwrap());
        graph.build(packages.into_iter(), true);
        graph
    }

    // Starting with a circular graph,
    //
    #[test]
    fn simple_circular() {
        let graph = make_circular_graph();

        let touched: Vec<PackageIdentIntern> = ident_intern_vec!("a/gcc");
        let unbuildable = UnbuildableMock { unbuildable_packages: Vec::new(), };

        let manifest = graph.compute_build(&touched, &unbuildable);
        println!("Manifest\n{:?}\n", manifest);

        assert_eq!(manifest.input_set.len(), 1);

        assert_eq!(manifest.unbuildable_reasons.len(), 0);

        assert_eq!(manifest.graph.node_count(), 15); // 2 external, 4*3 (cycle) + 1 (non-cycle)
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/gcc")), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/make")), true);

        assert_eq!(manifest.graph.contains_node(mk_IN("a/gcc", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/glibc", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/make", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/libgcc", 1)), true);

        assert_eq!(manifest.graph.contains_node(mk_IN("a/gcc", 2)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/glibc", 2)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/make", 2)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/libgcc", 2)), true);

        assert_eq!(manifest.graph.contains_node(mk_IN("a/gcc", 3)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/glibc", 3)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/make", 3)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/libgcc", 3)), true);

        assert_eq!(manifest.graph.contains_node(mk_IN("a/out", 1)), true);
    }

    // ////////////////////////////////////////////////////////////////////////
    //
    // Skipping this test for now, as it represents a case we don't correctly
    // handle, but is one which shouldn't exist given the current state of
    // StrongBuildDeps. When we look to expose that concept as a first class
    // part of our build language, this case will become important to handle
    // correctly.
    //
    // The issue with this test is that we declare a/make has a runtime
    // dependency on a/strong, and a/strong has a Strong build dependency
    // on a/make. This ends up looking like a runtime cycle to our build
    // ordering algorithms causing the tsort_subgraph function to fail on the
    // assert, as a/strong and a/make are never able to build.
    //
    // To the user, a failure here would be confusing as Strong build edges
    // were intended to be a mechanism to force some ordering, but because we
    // treat them as Run edges in the graph. We don't actually expose this as a
    // concept yet, and its use is limited to a well known subset of core
    // packages, so for now we'll leave this comment and test in place for when
    // we loop back around to this topic.
    //
    // ////////////////////////////////////////////////////////////////////////
    // Starting with a circular graph, extend with some complex build edges
    //
    #[test]
    #[ignore]
    fn simple_circular_with_strong_build_edges() {
        let mut graph = make_circular_graph();

        let extended = vec![mk_pkg("a/strong/c/d",
                                   &["a/libgcc", "a/glibc"],
                                   &["a/make"],
                                   &["a/make"]),
                            mk_pkg("a/make/z/d", &["a/strong", "a/glibc"], &[], &[])];

        graph.build(extended.into_iter(), true);

        let touched: Vec<PackageIdentIntern> = ident_intern_vec!("a/gcc");
        let unbuildable = UnbuildableMock { unbuildable_packages: Vec::new(), };

        let manifest = graph.compute_build(&touched, &unbuildable);
        println!("Manifest\n{:?}\n", manifest);

        assert_eq!(manifest.input_set.len(), 1);

        assert_eq!(manifest.unbuildable_reasons.len(), 0);

        assert_eq!(manifest.graph.node_count(), 15); // 2 external, 4*3 (cycle) + 1 (non-cycle)
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/gcc")), true);
        assert_eq!(manifest.graph.contains_node(mk_ELV("a/make")), true);

        assert_eq!(manifest.graph.contains_node(mk_IN("a/gcc", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/glibc", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/make", 1)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/libgcc", 1)), true);

        assert_eq!(manifest.graph.contains_node(mk_IN("a/gcc", 2)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/glibc", 2)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/make", 2)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/libgcc", 2)), true);

        assert_eq!(manifest.graph.contains_node(mk_IN("a/gcc", 3)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/glibc", 3)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/make", 3)), true);
        assert_eq!(manifest.graph.contains_node(mk_IN("a/libgcc", 3)), true);

        assert_eq!(manifest.graph.contains_node(mk_IN("a/out", 1)), true);
    }
}
