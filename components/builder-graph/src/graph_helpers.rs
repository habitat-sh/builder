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
               graphmap::DiGraphMap,
               Direction};

use itertools::Itertools; // 0.8.0

use std::{collections::{HashMap,
                        HashSet,
                        VecDeque},
          fs::File,
          io::prelude::*,
          iter::FromIterator,
          path::Path};

use crate::hab_core::package::PackageTarget;

use crate::{data_store::Unbuildable,
            package_build_manifest_graph::UnbuildableReason,
            package_ident_intern::PackageIdentIntern,
            util::*};

pub fn find_roots(graph: &DiGraphMap<PackageIdentIntern, EdgeType>) -> Vec<PackageIdentIntern> {
    let mut r = Vec::new();
    for node in graph.nodes() {
        let (in_count, _out_count) = count_edges(&graph, node);
        if in_count == 0 {
            r.push(node)
        }
    }
    r
}

pub fn scc_map(graph: &DiGraphMap<PackageIdentIntern, EdgeType>)
               -> HashMap<PackageIdentIntern, u32> {
    let mut scc_index: HashMap<PackageIdentIntern, u32> = HashMap::new();
    let scc = tarjan_scc(graph);

    for (cluster_number, cluster) in scc.into_iter().enumerate() {
        for node in cluster {
            scc_index.insert(node, cluster_number as u32);
        }
    }
    scc_index
}

// Produce strongly coupled cluster list.
pub fn dump_scc(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                filename: &str,
                _origin_filter: Option<&str>) {
    let path = Path::new(filename);
    let mut file = File::create(&path).unwrap();

    let scc = tarjan_scc(graph);

    for (cluster_number, cluster) in scc.into_iter().enumerate() {
        for node in cluster {
            writeln!(&mut file, "{}\t{}", cluster_number, node).unwrap();
        }
    }
}

pub fn count_edges(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                   node_index: PackageIdentIntern)
                   -> (u32, u32) {
    let mut in_count = 0;
    let mut out_count = 0;
    for _pred_index in graph.neighbors_directed(node_index, Direction::Incoming) {
        in_count += 1;
    }
    for _succ_index in graph.neighbors_directed(node_index, Direction::Outgoing) {
        out_count += 1;
    }
    (in_count, out_count)
}

pub fn count_edges_filtered(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                            node: PackageIdentIntern,
                            node_filter: Option<&str>,
                            edge_filter: Option<EdgeType>)
                            -> (u32, u32) {
    let mut in_count = 0;
    let mut out_count = 0;
    for pred in graph.neighbors_directed(node, Direction::Incoming) {
        if filter_match(&pred, node_filter)
           && filter_edge(*graph.edge_weight(pred, node).unwrap(), edge_filter)
        {
            in_count += 1;
        }
    }
    for succ in graph.neighbors_directed(node, Direction::Outgoing) {
        if filter_match(&succ, node_filter)
           && filter_edge(*graph.edge_weight(node, succ).unwrap(), edge_filter)
        {
            out_count += 1;
        }
    }
    (in_count, out_count)
}

pub fn changed_edges_for_type(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                              node: PackageIdentIntern,
                              deps: &[PackageIdentIntern],
                              edgetype: EdgeType)
                              -> (Vec<PackageIdentIntern>, Vec<PackageIdentIntern>) {
    let new_edges: HashSet<PackageIdentIntern> = HashSet::from_iter(deps.iter().copied());
    let current_edges: HashSet<PackageIdentIntern> =
        HashSet::from_iter(graph.neighbors_directed(node, Direction::Outgoing)
                                .filter(|succ| {
                                    *graph.edge_weight(node, *succ).unwrap() == edgetype
                                }));
    let added: Vec<PackageIdentIntern> = (new_edges.difference(&current_edges)).cloned().collect();
    let removed: Vec<PackageIdentIntern> =
        (current_edges.difference(&new_edges)).cloned().collect();

    (added, removed)
}

pub fn update_edges_for_type(graph: &mut DiGraphMap<PackageIdentIntern, EdgeType>,
                             node: PackageIdentIntern,
                             added: &[PackageIdentIntern],
                             removed: &[PackageIdentIntern],
                             edgetype: EdgeType) {
    for &e in added.iter() {
        graph.add_edge(node, e, edgetype);
    }
    for &e in removed.iter() {
        graph.remove_edge(node, e);
    }
}

pub fn revise_edges_for_type(graph: &mut DiGraphMap<PackageIdentIntern, EdgeType>,
                             node: PackageIdentIntern,
                             deps: &[PackageIdentIntern],
                             edgetype: EdgeType) {
    let (added, removed) = changed_edges_for_type(&graph, node, deps, edgetype);
    update_edges_for_type(graph, node, &added, &removed, edgetype)
}

// Right now this has a hardcoded assumption that we only care about runtime edge cycles, but that
// isn't going to be true always.
// TODO: examine whether the reverse dep scan is faster (most nodes are leaf nodes)
pub fn detect_cycles(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                     node: PackageIdentIntern,
                     added: &[PackageIdentIntern])
                     -> bool {
    // An empty 'added' pretty much is a no-op through here, might check and early exit if it's
    // really common.
    let mut seen: HashSet<PackageIdentIntern> = HashSet::from_iter(added.iter().cloned());
    let mut worklist: VecDeque<PackageIdentIntern> = VecDeque::from_iter(added.iter().cloned());

    // Detect if we have an edge to ourself (yes plan build might let this slip by)
    if seen.contains(&node) {
        return true;
    }

    // count incoming runtime edges
    // Count the number of packages that have declared a runtime dependency on us (incoming)
    // and the number of packages we have declared a runtime dependency on (outgoing).
    //
    // If no packages have declared a runtime dependency on us (0 incoming edges), then we
    // we cannot be part of a cycle because you need both incoming and outgoing edges to
    // create a cycle. Updating this package cannot introduce a cycle.
    let (incoming, _outgoing) =
        count_edges_filtered(&graph, node, None, Some(EdgeType::RuntimeDep));
    if incoming == 0 {
        return false;
    }

    while !worklist.is_empty() {
        let current_node = worklist.pop_back().unwrap();

        for succ in graph.neighbors_directed(current_node, Direction::Outgoing) {
            if *graph.edge_weight(current_node, succ).unwrap() == EdgeType::RuntimeDep {
                if succ == node {
                    return true;
                }
                if seen.insert(succ) {
                    worklist.push_back(succ);
                }
            }
        }
    }

    false
}

// Output a human readable, machine parsable form of the graph; useful for debugging
pub fn dump_graph_raw(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                      filename: &str,
                      origin_filter: Option<&str>) {
    let path = Path::new(filename);
    let mut file = File::create(&path).unwrap();

    // iterate through nodes
    for node in graph.nodes().sorted() {
        let (in_count, out_count) = count_edges(&graph, node);
        let orphaned = (in_count == 0) && (out_count == 0);

        if filter_match(&node, origin_filter) && !orphaned {
            let node_name = node.to_string();
            let mut bdeps = Vec::new();
            let mut rdeps = Vec::new();
            let mut sdeps = Vec::new();
            for succ in graph.neighbors_directed(node, Direction::Outgoing) {
                let edge_weight = graph.edge_weight(node, succ).unwrap();
                match edge_weight {
                    EdgeType::BuildDep => bdeps.push(succ),
                    EdgeType::RuntimeDep => rdeps.push(succ),
                    EdgeType::StrongBuildDep => sdeps.push(succ),
                    EdgeType::ExternalConstraint => {
                        unimplemented!("External Constraints should not appear here")
                    }
                }
            }
            bdeps.sort();
            rdeps.sort();
            sdeps.sort();
            let bdeps_join = join_idents(",", &bdeps);
            let rdeps_join = join_idents(",", &rdeps);
            let sdeps_join = join_idents(",", &sdeps);
            writeln!(&mut file,
                     "{};\t{};{};\t{};\t{};\t{}",
                     node_name, in_count, out_count, rdeps_join, bdeps_join, sdeps_join).unwrap();
        }
    }
}

pub fn emit_graph_as_dot(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                         filename: &str,
                         origin_filter: Option<&str>) {
    let path = Path::new(filename);
    let mut file = File::create(&path).unwrap();
    // This might be simpler to implement by creating a filtered graph, and then emiting it.

    writeln!(&mut file, "// Filtered by {:?}", origin_filter).unwrap();
    writeln!(&mut file, "digraph \"{}\" {{", filename).unwrap();
    writeln!(&mut file, "    rankdir=\"UD\";").unwrap();

    // iterate through nodes
    for node in graph.nodes() {
        let (in_count, out_count) = count_edges(&graph, node);
        let orphaned = (in_count == 0) && (out_count == 0);
        if orphaned {
            debug!("{} is orphaned", node);
        }

        if filter_match(&node, origin_filter) && !orphaned {
            let node_name = node.to_string();
            writeln!(&mut file, "    \"{}\"", node_name).unwrap();
        }
    }

    // iterate through regular edges
    writeln!(&mut file, "//######## RUN TIME EDGES ######").unwrap();
    writeln!(&mut file, "    edge [ weight = 10; constraint = true ];").unwrap();

    write_edges(&graph, &mut file, EdgeType::RuntimeDep, origin_filter);

    writeln!(&mut file, "//######## BUILD TIME EDGES ######").unwrap();
    writeln!(&mut file,
             "    edge [ color = \"blue\" style = \"dashed\" constraint = false ];").unwrap();

    // iterate through build edges
    write_edges(&graph, &mut file, EdgeType::BuildDep, origin_filter);

    // close
    writeln!(&mut file, "}}").unwrap();
}

fn write_edges(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
               file: &mut File,
               edge_type: EdgeType,
               origin_filter: Option<&str>) {
    for (src, dst, edge) in graph.all_edges() {
        if *edge == edge_type && filter_match(&src, origin_filter) {
            let src_name = src.to_string();
            let dst_name = dst.to_string();
            write_edge(file, &src_name, &dst_name, Some(edge_type));
        }
    }
}

fn write_edge(file: &mut File, src: &str, dst: &str, edge_type: Option<EdgeType>) {
    match edge_type {
        Some(etype) => {
            writeln!(file,
                     "    \"{}\" -> \"{}\" [type=\"{}\"];",
                     src,
                     dst,
                     edgetype_to_abbv(etype)).unwrap()
        }
        None => writeln!(file, "    \"{}\" -> \"{}\"", src, dst).unwrap(),
    }
}

fn edgetype_to_abbv(edge: EdgeType) -> &'static str {
    match edge {
        EdgeType::RuntimeDep => "R",
        EdgeType::BuildDep => "B",
        EdgeType::StrongBuildDep => "S",
        EdgeType::ExternalConstraint => "X",
    }
}

// Prune the SCC results using the rebuild_set
// A component elements should either all be inside the set, or not at all
// If a component is partially in the set this is an error
//
pub fn filtered_scc(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                    rebuild_set: &[PackageIdentIntern])
                    -> Vec<Vec<PackageIdentIntern>> {
    // tarjan_scc a returns a vector of components, each of which
    // contains a vector of nodes in the component. A component
    // may only contain a single node, when that node has no back
    // edges/ cyclic dependencies. These nodes are returned in
    // topological sort order. All we need to do to compute a
    // valid build ordering is to take the components and sort
    // them in runtime edge topological order
    let scc: Vec<Vec<PackageIdentIntern>> = tarjan_scc(&graph);

    let mut rebuild_node_idents: HashSet<PackageIdentIntern> = HashSet::new();
    for ident in rebuild_set {
        if graph.contains_node(*ident) {
            rebuild_node_idents.insert(*ident);
        }
    }

    // Most common case is core, which is a substantial fraction of the total packages we would
    // automatically rebuild, so we choose a size on the larger end to avoid
    // reallocation.
    let mut filtered_set = Vec::with_capacity(scc.len());
    for component in scc {
        // Count how many elements of the component are in the rebuild set.
        // Maybe there's a more idomatic way of writing the filter body?
        let result = component.iter().fold(0, |count, node_ident| {
                                         if rebuild_node_idents.contains(node_ident) {
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

// This is an extension of the concept of rdeps; we take a starting set
// (the 'seed') and transitively expand the set to include the things that depend
// on any member of seed. The 'origin' argument restricts this to remain inside a single origin
pub fn flood_deps_in_origin(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                            seed: &[PackageIdentIntern],
                            origin: Option<&str>)
                            -> Vec<PackageIdentIntern> {
    debug!("CRS: starting with origin {}",
           origin.unwrap_or("No origin specified"));
    debug!("CRS: touched set {}", join_idents(", ", &seed));

    // Flood reverse dependency graph, filtering by origin
    let mut seen: HashSet<PackageIdentIntern> = HashSet::new();
    let mut worklist: VecDeque<PackageIdentIntern> = VecDeque::new();

    // Insert 'touched' nodes into worklist
    for &node_ident in seed {
        worklist.push_back(node_ident);
    }

    while !worklist.is_empty() {
        let node_ident = worklist.pop_front().unwrap();
        debug!("CBS: processing {}", node_ident);
        seen.insert(node_ident);

        // loop through everyone who has a build or runtime dep on this package
        // Note: if a package is missing from the graph, we get an empty iterator
        for pred_ident in graph.neighbors_directed(node_ident, Direction::Incoming) {
            debug!("CBS: Checking {}", pred_ident);
            if !seen.contains(&pred_ident) && filter_match(&pred_ident, origin) {
                debug!("CBS: adding from {} the node {}", node_ident, pred_ident);
                worklist.push_back(pred_ident);
            }
        }
    }
    Vec::from_iter(seen)
}

// This recursively expands the deps of a set of packages, to compute the transitive dep set.
// Similar to flood deps, but in reverse with more options, might be worth attempting to unify
pub fn transitive_deps(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                       seed: &[PackageIdentIntern],
                       origin: Option<&str>,
                       include_build_deps: bool)
                       -> HashSet<PackageIdentIntern> {
    debug!("TDEP: starting with origin {}",
           origin.unwrap_or("No origin specified"));
    debug!("TDEP: seed set {}", join_idents(", ", &seed));

    let mut seen: HashSet<PackageIdentIntern> = HashSet::new();
    let mut worklist: VecDeque<PackageIdentIntern> = VecDeque::new();

    // Insert 'touched' nodes into worklist
    for &node_ident in seed {
        worklist.push_back(node_ident);
    }

    while !worklist.is_empty() {
        let node_ident = worklist.pop_front().unwrap();
        debug!("CBS: processing {}", node_ident);
        seen.insert(node_ident);

        // loop through everyone who has a build or runtime dep on this package
        for succ_ident in graph.neighbors_directed(node_ident, Direction::Outgoing) {
            debug!("TDEP: Checking {}", succ_ident);
            let &edge = graph.edge_weight(node_ident, succ_ident).unwrap(); // unwrap safe because of neighbors finding it a moment ago
            let edge_ok = match edge {
                EdgeType::RuntimeDep => true,
                EdgeType::BuildDep => include_build_deps,
                EdgeType::StrongBuildDep => include_build_deps,
                EdgeType::ExternalConstraint => {
                    unimplemented!("External Constraints should not appear here")
                }
            };

            if edge_ok && !seen.contains(&succ_ident) && filter_match(&succ_ident, origin) {
                debug!("TDEP: adding from {} the node {}", node_ident, succ_ident);
                worklist.push_back(succ_ident);
            }
        }
    }
    seen
}

// All work is filtered by an origin
// Computing the rebuild set is done in three phases
// 1) we take the set of touched packages, and then flood the graph to find all the packages that
// depend them, filtered by an origin 2) we find the packages in that set that are unbuildable
// 3) we flood the graph to find all the packages that are rendered unbuildable because a dep is
// unbuilable.
pub fn compute_rebuild_set(
    graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
    unbuildable: &dyn Unbuildable,
    touched: &[PackageIdentIntern],
    origin: Option<&str>,
    target: PackageTarget)
    -> (Vec<PackageIdentIntern>, HashMap<PackageIdentIntern, UnbuildableReason>) {
    // Note: consider making these APIs use HashSet all the way through
    let rebuild = flood_deps_in_origin(&graph, touched, origin);

    let unbuildable = unbuildable.filter_unbuildable(&rebuild, target).unwrap();

    let mut unbuildable_reasons: HashMap<PackageIdentIntern, UnbuildableReason> = HashMap::new();
    for &package in unbuildable.iter() {
        unbuildable_reasons.insert(package, UnbuildableReason::Direct);
    }

    let unbuildable = flood_deps_in_origin(&graph, &unbuildable, origin);
    for &package in unbuildable.iter() {
        unbuildable_reasons.entry(package)
                           .or_insert(UnbuildableReason::Indirect);
    }

    let rebuild: HashSet<PackageIdentIntern> = HashSet::from_iter(rebuild);
    let unbuildable: HashSet<PackageIdentIntern> = HashSet::from_iter(unbuildable);

    let difference: HashSet<_> = rebuild.difference(&unbuildable).collect();

    let rebuild_set = difference.into_iter().cloned().collect();
    (rebuild_set, unbuildable_reasons)
}

// This could be implmented by creating a subgraph in PetGraph, but my initial experiments had
// issues with NodeIndex changing in the new graph, which scrambled our system for tracking
// things via NodeIndex. Now we use GraphMap, which would remove the
// need to track, and thus enable the use of subgraphs.
pub fn compute_build_order(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                           rebuild_set: &[PackageIdentIntern])
                           -> Vec<Vec<PackageIdentIntern>> {
    let scc = filtered_scc(&graph, rebuild_set);

    let mut node_order: Vec<Vec<PackageIdentIntern>> = Vec::new();
    for component in scc {
        let ordered_component = tsort_subgraph(&graph, &component);
        node_order.push(ordered_component)
    }

    node_order
}

// for each component in SCC we sort it in topological order by runtime dep edges
//
// We could extract a subgraph containing only the
// component nodes and the runtime edges, and run the
// petgraph tsort over the subgraph. However constructing
// subgraphs is a little bit messy due to our current
// graph implementation choices. It may be worth
// simplifying the graph implementation (in particular
// looking at the GraphMap struct) to let us use the built
// in tsort.
//
// However for now, we're going to implement our own tsort.
// NOTE: An alternate construction would be to add in build deps that don't create a cycle,
// to provide a stronger build order than rt_deps alone. This would ensure fewer build deps carry
// over, and possibly provide orderings that avoid the need for strong build deps.
pub fn tsort_subgraph(graph: &DiGraphMap<PackageIdentIntern, EdgeType>,
                      component: &[PackageIdentIntern])
                      -> Vec<PackageIdentIntern> {
    let mut result: Vec<PackageIdentIntern> = Vec::new();

    // Basic worklist algorithm for tsort
    let mut worklist: VecDeque<PackageIdentIntern> = VecDeque::new();
    let mut unsatisfied: HashMap<PackageIdentIntern, usize> = HashMap::new();

    // We pre-fill this to allow us to efficiently test for membership below
    for node_ident in component {
        unsatisfied.insert(*node_ident, usize::max_value());
    }
    // First, walk through all the nodes, and count how many things they depend on
    // If they have no runtime deps in the
    for &node_ident in component {
        let mut dep_count = 0;
        for succ_ident in graph.neighbors_directed(node_ident, Direction::Outgoing) {
            let &edge = graph.edge_weight(node_ident, succ_ident).unwrap(); // unwrap safe because of neighbors finding it a moment ago
            if edge == EdgeType::RuntimeDep || edge == EdgeType::StrongBuildDep {
                if edge == EdgeType::StrongBuildDep {
                    debug!("StrongEdge between {} {}", node_ident, succ_ident);
                }
                // We assume runtime deps that aren't part of the component are already built
                // and safe to ignore.
                if unsatisfied.contains_key(&succ_ident) {
                    dep_count += 1;
                }
            }
        }
        unsatisfied.insert(node_ident, dep_count);
    }

    // Add things with no unsatisfied deps to worklist
    for (node_ident, dep_count) in &unsatisfied {
        assert!(*dep_count != usize::max_value());
        if *dep_count == 0 {
            worklist.push_back(*node_ident)
        }
    }

    // Termination properties and complexity
    // As long as the runtime dep graph is a DAG (no cycles), a node should be put on and
    // removed from the worklist exactly once each. A cycle will create a situation
    // where the unsatisified count will never drop to zero, and we would not visit
    // every node. So this outer loop should execute exactly component.len() times.
    // The inner loop only executes once for each edge, so our total complexity is
    // O(N*mean_edge_count) -> O(E)
    let mut iter_count = 0;
    while !worklist.is_empty() {
        iter_count += 1;
        let node_ident = worklist.pop_front().unwrap(); // always safe because not empty
        result.push(node_ident);

        // go through the things that depend on me and mark one less dependency needed.
        // If I was the last dependency, we are ready to go, and can be added to the worklist.
        for pred_index in graph.neighbors_directed(node_ident, Direction::Incoming) {
            // unwrap safe because of neighbors finding it a moment ago
            let &edge = graph.edge_weight(pred_index, node_ident).unwrap();
            if edge == EdgeType::RuntimeDep || edge == EdgeType::StrongBuildDep {
                unsatisfied.entry(pred_index).and_modify(|count| {
                                                 *count -= 1;
                                                 if *count == 0 {
                                                     worklist.push_back(pred_index);
                                                 }
                                             });
            }
        }
    }

    assert_eq!(iter_count, component.len());
    result
}
