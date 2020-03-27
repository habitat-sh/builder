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

use crate::{hab_core::package::PackageIdent,
            util::*};

use crate::{ident_graph::IdentGraph,
            util::*};

type IdentIndex = usize;

pub struct PackageBuild {
    ident: PackageIdent,
    bdeps: Vec<PackageIdent>,
    rdeps: Vec<PackageIdent>,
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
pub fn compute_build(graph: IdentGraph<Value>,
                     filter: &str,
                     base_set: &Vec<PackageIdent>,
                     touched: &Vec<PackageIdent>,
                     converge_count: uint)
                     -> Vec<Build> {
    let rebuild_set = compute_rebuild_set(graph, touched, origin);

    let build_order = compute_build_order(graph, origin);

    let mut latest = HashMap::<PackageIdent, PackageIdent>::new();
    let mut built = HashMap::<PackageIdent, PackageBuild>::new();

    for component in build_order {
        for i in [1..converge_count] {
            for node in component {
                let build = build_package(node, latest);
                latest.update(util::short_ident(build.ident), build.ident.clone());
                built.update(build.ident.clone(), build);
            }
        }
    }

    let build_actual = prune_tsort(built);

    build_actual
}

// Compute order as a level diagram; each package depends on weights only lower than it. This is
// a variant of a toplogical ordering, and uses the SCC to collapse cycles.
// We compute two types of ordering:
// * The first uses all edges but sets all members of an SCC as equal, and hence avoids issues with
//   cycles
// * The second uses only runtime edges, which by definition
// avoids cycles.  A more nuanced (and stricter) choice would be
// to include build time edges that are not back edges, but in
// irreducible graphs (which these are) the choice of back vs
// cross edges depends on the exact DFS order, and so is somewhat
// arbitrary.
// Note, this is a candidate to be extracted and generalized, as it only needs the graph to
// work.
pub fn compute_levels<Value>(graph: IdentGraph<Value>) -> HashMap<NodeIndex, (u32, u32)>
    where Value: Default + Copy
{
    let mut levels: HashMap<NodeIndex, (u32, u32)> = HashMap::new();
    // Compute SCC map. We use this to determine what component we're in.
    let scc_map = graph.scc_map();

    // Right now the worklist is a simple FIFO queue with no deduplication. Could use a
    // BTreeSet, but that does potentially screwy things with the ordering.
    let mut worklist: VecDeque<NodeIndex> = VecDeque::new();

    // Phase one; assign 'seed' weights of zero, and add to the worklist.

    for node_index in graph.node_indices() {
        levels.insert(node_index, (0, 0));
        // potential minor optimization: nodes w/o dependencies should not be added to worklist.
        worklist.push_back(node_index)
    }

    // Phase two; iterate over worklist updating node heights
    let mut visits = 0;
    let mut max_scc_level = 0;
    let mut max_rt_level = 0;

    while !worklist.is_empty() {
        visits += 1;

        let node_index = worklist.pop_front().unwrap();
        let mut new_scc_level = 0;
        let mut new_rt_level = 0;

        for succ_index in graph.neighbors_directed(node_index, Direction::Outgoing) {
            let edge = graph.find_edge(node_index, succ_index).unwrap();

            // If we are in the same SCC, we don't increment the index
            let scc_increment = if scc_map[&node_index] == scc_map[&succ_index] {
                0
            } else {
                1
            };
            new_scc_level = cmp::max(new_scc_level, levels[&succ_index].0 + scc_increment);

            if graph.edge_weight(edge) == Some(&EdgeType::RuntimeDep) {
                new_rt_level = cmp::max(new_rt_level, levels[&succ_index].1 + 1);
            }
        }

        max_scc_level = cmp::max(new_scc_level, max_scc_level);
        max_rt_level = cmp::max(new_rt_level, max_rt_level);

        if (new_scc_level > levels[&node_index].0) || (new_rt_level > levels[&node_index].1) {
            // update mygraph
            levels.insert(node_index, (new_scc_level, new_rt_level));

            // Put everybody who depends on me back on the worklist (this is where dedup would
            // be nice) Also, we're a bit too aggressive; technically rt_level
            // updates only propagate to runtime edges.
            for pred_index in graph.graph
                                   .neighbors_directed(node_index, Direction::Incoming)
            {
                let edge = graph.find_edge(pred_index, node_index).unwrap();
                if graph.edge_weight(edge) == Some(&EdgeType::RuntimeDep) {
                    worklist.push_back(pred_index)
                }
            }
        }
    }
    println!("Levels computed, {} nodes {} visits, max scc level {}, max rt level {}",
             graph.node_count(),
             visits,
             max_scc_level,
             max_rt_level);

    levels
}

// Compute the set of packages that needs rebuilding, starting with a base kernel of 'touched
// packages'
pub fn compute_rebuild<Value>(graph: &IdentGraph<Value>,
                              touched: &Vec<PackageIdent>,
                              origin: &str)
                              -> Vec<PackageIdent>
    where Value: Default + Copy
{
    // Flood reverse dependency graph, filtering by origin
    let mut seen: HashSet<NodeIndex> = HashSet::new();
    let mut worklist: VecDeque<NodeIndex> = VecDeque::new();

    // Insert 'touched' nodes into worklist

    // Iterate over worklist,

    Vec::<PackageIdent>::new()
}
