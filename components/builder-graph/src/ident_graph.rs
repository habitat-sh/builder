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

//

use petgraph::{algo::{tarjan_scc,
                      toposort},
               graph::NodeIndex,
               visit::{depth_first_search,
                       DfsEvent},
               Direction,
               Graph};

use std::collections::{HashMap,
                       HashSet,
                       VecDeque};

use std::{cmp,
          fs::File,
          io::prelude::*,
          iter::FromIterator,
          path::Path,
          str::FromStr};

use crate::{hab_core::package::PackageIdent,
            util::*};

type IdentIndex = usize;

#[derive(Default)]
struct IdentMemo {
    // It would be nice not to have two copies of Ident
    idents:    Vec<PackageIdent>,
    ident_map: HashMap<PackageIdent, IdentIndex>,
}

// BiMap between Ident and a index.
impl IdentMemo {
    pub fn index_for_ident(&mut self, ident: &PackageIdent) -> IdentIndex {
        if self.ident_map.contains_key(ident) {
            self.ident_map[ident]
        } else {
            let index = self.idents.len();
            self.idents.push(ident.clone());
            self.ident_map.insert(ident.clone(), index);
            index
        }
    }

    pub fn get_ident(&self, index: IdentIndex) -> &PackageIdent { &self.idents[index] }

    pub fn get_index(&self, ident: &PackageIdent) -> Option<IdentIndex> {
        if self.ident_map.contains_key(ident) {
            Some(self.ident_map[ident])
        } else {
            None
        }
    }

    // TODO: maybe helper fn to compare/sort by index.
}

struct IdentGraphElement<Value> {
    ident_index: IdentIndex,
    node_index:  NodeIndex,
    value:       Value,
}

// IdentGraph allows us to map an Ident to a graph node, and update a value for that node
// petgraph doesn't allow value updating...
#[derive(Default)]
pub struct IdentGraph<Value> {
    data:       Vec<IdentGraphElement<Value>>,
    pub graph:  Graph<IdentIndex, EdgeType>, // TODO Fix the 'pub' hack
    ident_memo: IdentMemo,
}

impl<Value> IdentGraph<Value> where Value: Default + Copy
{
    pub fn new() -> Self { IdentGraph::default() }

    pub fn get_node_by_id(&mut self, ident: &PackageIdent) -> (IdentIndex, NodeIndex, Value) {
        let ident_index = self.ident_memo.index_for_ident(ident);

        if ident_index == self.data.len() {
            let node_index = self.graph.add_node(ident_index);
            assert_eq!(node_index.index(), ident_index);

            let value = Default::default();
            self.data.push(IdentGraphElement { ident_index,
                                               node_index,
                                               value });
            (ident_index, node_index, value)
        } else {
            let IdentGraphElement { ident_index: expected_index,
                                    node_index,
                                    value, } = self.data[ident_index];
            assert_eq!(expected_index, ident_index);
            (ident_index, node_index, value)
        }
    }

    pub fn get_node(&mut self, ident: &PackageIdent) -> (NodeIndex, Value) {
        let (_ident_index, node_index, value) = self.get_node_by_id(&ident);
        (node_index, value)
    }

    // We should rename/refactor this and the above one so that readonly is the default
    pub fn get_node_if_exists(&self, ident: &PackageIdent) -> (NodeIndex, Value) {
        if let Some(ident_index) = self.ident_memo.get_index(ident) {
            let IdentGraphElement { ident_index: _expected_index,
                                    node_index,
                                    value, } = self.data[ident_index];
            (node_index, value)
        } else {
            panic!("Couldn't find node, and this should never happen")
        }
    }

    pub fn upsert_node(&mut self, ident: &PackageIdent, value: Value) -> (IdentIndex, NodeIndex) {
        // Replace node in place (update value) We replace nodes in
        // place because deleting a node can cause renumbering, and it
        // preserves the in-edges for free.
        let (ident_index, node_index, _value) = self.get_node_by_id(ident);
        self.data[ident_index].value = value;
        (ident_index, node_index)
    }

    pub fn ident_for_node(&self, node: NodeIndex) -> &PackageIdent {
        self.ident_memo.get_ident(node.index())
    }

    pub fn add_edge(&mut self, node: NodeIndex, dest_node: &PackageIdent, edge: EdgeType) {
        let (_, dest_node_index, _) = self.get_node_by_id(dest_node);
        self.graph.add_edge(node, dest_node_index, edge);
    }

    pub fn drop_outgoing(&mut self, node: NodeIndex) {
        let sucessors: Vec<NodeIndex> = self.graph.neighbors(node).collect();
        for succ in sucessors {
            let edge = self.graph.find_edge(node, succ).unwrap();
            self.graph.remove_edge(edge);
        }
    }

    pub fn counts(&self) -> (usize, usize) { (self.graph.node_count(), self.graph.edge_count()) }

    // Output a human readable, machine parsable form of the graph; useful for debugging
    pub fn dump_graph_raw(&self, filename: &str, origin_filter: Option<&str>) {
        let path = Path::new(filename);
        let mut file = File::create(&path).unwrap();

        // iterate through nodes
        for node_index in self.graph.node_indices() {
            let (in_count, out_count) = self.count_edges(node_index);
            let orphaned = (in_count == 0) && (out_count == 0);

            let node = self.ident_for_node(node_index);
            if filter_match(node, origin_filter) && !orphaned {
                let node_name = node.to_string();
                let mut bdeps = Vec::new();
                let mut rdeps = Vec::new();
                for succ_index in self.graph
                                      .neighbors_directed(node_index, Direction::Outgoing)
                {
                    let edge_index = self.graph.find_edge(node_index, succ_index).unwrap();
                    match self.graph.edge_weight(edge_index).unwrap() {
                        EdgeType::BuildDep => bdeps.push(succ_index),
                        EdgeType::RuntimeDep => rdeps.push(succ_index),
                    }
                }
                let bdeps_join = self.join_nodes(&bdeps, ",");
                let rdeps_join = self.join_nodes(&rdeps, ",");
                writeln!(&mut file,
                         "{};\t{};{};\t{};\t{}",
                         node_name, in_count, out_count, rdeps_join, bdeps_join).unwrap();
            }
        }
    }

    pub fn emit_graph_as_dot(&self, filename: &str, origin_filter: Option<&str>) {
        let path = Path::new(filename);
        let mut file = File::create(&path).unwrap();

        // This might be simpler to implement by creating a filtered graph, and then emiting it.
        // Uncertain how filter graphs rewrite node_index; we depend on that remaining constant.
        // Investigate whether graph map would work better.

        writeln!(&mut file, "// Filtered by {:?}", origin_filter).unwrap();
        writeln!(&mut file, "digraph \"{}\" {{", filename).unwrap();
        writeln!(&mut file, "    rankdir=\"UD\";").unwrap();

        // iterate through nodes
        for node_index in self.graph.node_indices() {
            let node = self.ident_for_node(node_index);
            let (in_count, out_count) = self.count_edges(node_index);
            let orphaned = (in_count == 0) && (out_count == 0);
            if orphaned {
                println!("{} is orphaned", node);
            }

            if filter_match(node, origin_filter) && !orphaned {
                let node_name = node.to_string();
                writeln!(&mut file, "    \"{}\"", node_name).unwrap();
            }
        }

        // iterate through regular edges
        writeln!(&mut file, "//######## RUN TIME EDGES ######").unwrap();
        writeln!(&mut file, "    edge [ weight = 10; constraint = true ];").unwrap();

        self.write_edges(&mut file, EdgeType::RuntimeDep, origin_filter);

        writeln!(&mut file, "//######## BUILD TIME EDGES ######").unwrap();
        writeln!(&mut file,
                 "    edge [ color = \"blue\" style = \"dashed\" constraint = false ];").unwrap();

        // iterate through build edges
        self.write_edges(&mut file, EdgeType::BuildDep, origin_filter);

        // close
        writeln!(&mut file, "}}").unwrap();
    }

    // This probably could be completely generic to graph
    fn count_edges(&self, node_index: NodeIndex) -> (u32, u32) {
        let mut in_count = 0;
        let mut out_count = 0;
        for _pred_index in self.graph
                               .neighbors_directed(node_index, Direction::Incoming)
        {
            in_count += 1;
        }
        for _succ_index in self.graph
                               .neighbors_directed(node_index, Direction::Outgoing)
        {
            out_count += 1;
        }
        (in_count, out_count)
    }

    fn write_edges(&self, file: &mut File, edge_type: EdgeType, origin_filter: Option<&str>) {
        for edge_index in self.graph.edge_indices() {
            if let Some(&edge) = self.graph.edge_weight(edge_index) {
                if edge == edge_type {
                    if let Some((src_idx, dst_idx)) = self.graph.edge_endpoints(edge_index) {
                        let src = self.ident_for_node(src_idx);
                        if filter_match(src, origin_filter) {
                            let src_name = src.to_string();
                            let dst_name = self.ident_for_node(dst_idx).to_string();
                            write_edge(file, &src_name, &dst_name, Some(edge_type));
                        }
                    }
                }
            }
        }
    }

    // Compute order as a level diagram; each package depends on weights only lower than it. This is
    // a variant of a toplogical ordering, and uses the SCC to collapse cycles.
    // We compute two types of ordering:
    // * The first uses all edges but sets all members of an SCC as equal, and hence avoids issues
    //   with cycles
    // * The second uses only runtime edges, which by definition
    // avoids cycles.  A more nuanced (and stricter) choice would be
    // to include build time edges that are not back edges, but in
    // irreducible graphs (which these are) the choice of back vs
    // cross edges depends on the exact DFS order, and so is somewhat
    // arbitrary.
    // Note, this is a candidate to be extracted and generalized, as it only needs the graph to
    // work.
    pub fn compute_levels(&self) -> HashMap<NodeIndex, (u32, u32)> {
        let mut levels: HashMap<NodeIndex, (u32, u32)> = HashMap::new();
        // Compute SCC map. We use this to determine what component we're in.
        let scc_map = self.scc_map();

        // Right now the worklist is a simple FIFO queue with no deduplication. Could use a
        // BTreeSet, but that does potentially screwy things with the ordering.
        let mut worklist: VecDeque<NodeIndex> = VecDeque::new();

        // Phase one; assign 'seed' weights of zero, and add to the worklist.

        for node_index in self.graph.node_indices() {
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

            for succ_index in self.graph
                                  .neighbors_directed(node_index, Direction::Outgoing)
            {
                let edge = self.graph.find_edge(node_index, succ_index).unwrap();

                // If we are in the same SCC, we don't increment the index
                let scc_increment = if scc_map[&node_index] == scc_map[&succ_index] {
                    0
                } else {
                    1
                };
                new_scc_level = cmp::max(new_scc_level, levels[&succ_index].0 + scc_increment);

                if self.graph.edge_weight(edge) == Some(&EdgeType::RuntimeDep) {
                    new_rt_level = cmp::max(new_rt_level, levels[&succ_index].1 + 1);
                }
            }

            max_scc_level = cmp::max(new_scc_level, max_scc_level);
            max_rt_level = cmp::max(new_rt_level, max_rt_level);

            if (new_scc_level > levels[&node_index].0) || (new_rt_level > levels[&node_index].1) {
                // update myself
                levels.insert(node_index, (new_scc_level, new_rt_level));

                // Put everybody who depends on me back on the worklist (this is where dedup would
                // be nice) Also, we're a bit too aggressive; technically rt_level
                // updates only propagate to runtime edges.
                for pred_index in self.graph
                                      .neighbors_directed(node_index, Direction::Incoming)
                {
                    let edge = self.graph.find_edge(pred_index, node_index).unwrap();
                    if self.graph.edge_weight(edge) == Some(&EdgeType::RuntimeDep) {
                        worklist.push_back(pred_index)
                    }
                }
            }
        }
        println!("Levels computed, {} nodes {} visits, max scc level {}, max rt level {}",
                 self.graph.node_count(),
                 visits,
                 max_scc_level,
                 max_rt_level);

        levels
    }

    pub fn dump_build_levels(&self, filename: &str, origin_filter: Option<&str>) {
        let path = Path::new(filename);
        let mut file = File::create(&path).unwrap();

        let levels = self.compute_levels();
        for (node, (scc_level, rt_level)) in levels {
            let ident = self.ident_for_node(node);
            if filter_match(ident, origin_filter) {
                writeln!(&mut file, "{}\t{}\t{}", scc_level, rt_level, ident).unwrap();
            }
        }
    }

    // this could be extracted
    pub fn scc_map(&self) -> HashMap<NodeIndex, u32> {
        let mut scc_index: HashMap<NodeIndex, u32> = HashMap::new();
        let scc = tarjan_scc(&self.graph);

        for (cluster_number, cluster) in scc.into_iter().enumerate() {
            for node in cluster {
                scc_index.insert(node, cluster_number as u32);
            }
        }
        scc_index
    }

    // Produce strongly coupled cluster list.
    pub fn dump_scc(&self, filename: &str, _origin_filter: Option<&str>) {
        let path = Path::new(filename);
        let mut file = File::create(&path).unwrap();

        let scc = tarjan_scc(&self.graph);

        for (cluster_number, cluster) in scc.into_iter().enumerate() {
            for node in cluster {
                writeln!(&mut file,
                         "{}\t{}",
                         cluster_number,
                         self.ident_for_node(node)).unwrap();
            }
        }
    }

    pub fn join_nodes(&self, nodes: &[NodeIndex], sep: &str) -> String {
        let strings: Vec<String> = nodes.iter()
                                        .map(|x| self.ident_for_node(*x).to_string())
                                        .collect();
        strings.join(sep)
    }

    pub fn compute_rebuild_set(&self,
                               touched: &Vec<PackageIdent>,
                               origin: &str)
                               -> Vec<PackageIdent>
        where Value: Default + Copy
    {
        let debug = false;

        if debug {
            debug!("CRS: starting with origin {}", origin);
            debug!("CRS: touched set {}", join_idents(", ", &touched));
        }

        // Flood reverse dependency graph, filtering by origin
        let mut seen: HashSet<NodeIndex> = HashSet::new();
        let mut worklist: VecDeque<NodeIndex> = VecDeque::new();

        // Insert 'touched' nodes into worklist
        for ident in touched {
            let (node_index, _) = self.get_node_if_exists(ident);
            worklist.push_back(node_index);
        }

        while !worklist.is_empty() {
            let node_index = worklist.pop_front().unwrap();
            if debug {
                debug!("CBS: processing {} {:?}",
                       self.ident_for_node(node_index),
                       node_index);
            }
            seen.insert(node_index);

            // loop through everyone who has a build or runtime dep on this package
            for pred_index in self.graph
                                  .neighbors_directed(node_index, Direction::Incoming)
            {
                if !seen.contains(&pred_index) {
                    let ident = self.ident_for_node(pred_index);
                    if filter_match(ident, Some(origin)) {
                        if debug {
                            debug!("CBS: adding from {:?} the node {} {:?}",
                                   node_index,
                                   self.ident_for_node(pred_index),
                                   pred_index)
                        }
                        worklist.push_back(pred_index);
                    }
                }
            }
        }

        seen.iter()
            .map(|node_index| self.ident_for_node(*node_index).clone())
            .collect()
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
    pub fn tsort_subgraph(&self, component: &Vec<NodeIndex>) -> Vec<NodeIndex> {
        let mut result: Vec<NodeIndex> = Vec::new();

        // Basic worklist algorithm for tsort
        let mut worklist: VecDeque<NodeIndex> = VecDeque::new();
        let mut unsatisfied: HashMap<NodeIndex, usize> = HashMap::new();

        // We pre-fill this to allow us to efficiently test for membership below
        for node_index in component {
            unsatisfied.insert(*node_index, usize::max_value());
        }
        // First, walk through all the nodes, and count how many things they depend on
        // If they have no runtime deps in the
        for node_index in component {
            let mut dep_count = 0;
            for succ_index in self.graph
                                  .neighbors_directed(*node_index, Direction::Outgoing)
            {
                let edge = self.graph.find_edge(*node_index, succ_index).unwrap();
                if self.graph.edge_weight(edge) == Some(&EdgeType::RuntimeDep) {
                    // We assume runtime deps that aren't part of the component are already built
                    // and safe to ignore.
                    if unsatisfied.contains_key(&succ_index) {
                        dep_count += 1;
                    }
                }
            }
            unsatisfied.insert(*node_index, dep_count);
        }

        // Add things with no unsatisfied deps to worklist
        for (node_index, dep_count) in &unsatisfied {
            assert!(*dep_count != usize::max_value());
            if *dep_count == 0 {
                worklist.push_back(*node_index)
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
            let node_index = worklist.pop_front().unwrap(); // always safe because not empty
            result.push(node_index);

            // go through the things that depend on me and mark one less dependency needed.
            // If I was the last dependency, we are ready to go, and can be added to the worklist.
            for pred_index in self.graph
                                  .neighbors_directed(node_index, Direction::Incoming)
            {
                let edge = self.graph.find_edge(pred_index, node_index).unwrap();
                if self.graph.edge_weight(edge) == Some(&EdgeType::RuntimeDep) {
                    unsatisfied.entry(pred_index).and_modify(|count| {
                                                     *count -= 1;
                                                     if *count == 0 {
                                                         worklist.push_back(pred_index);
                                                     }
                                                 });
                }
            }
        }
        assert!(iter_count == component.len());
        result
    }

    // Build edge aware tsort; this attempts to choose an order respecting build edges whenever
    // possible
    pub fn tsort_subgraph_with_build_edges(&self, component: &Vec<NodeIndex>) -> Vec<NodeIndex> {
        // steps
        // 1) Compute subgraph
        let component_set = HashSet::<NodeIndex>::from_iter(component.iter().cloned());
        let mut subgraph = self.graph.filter_map(|ni, n| {
                                                     if component_set.contains(&ni) {
                                                         Some(n)
                                                     } else {
                                                         None
                                                     }
                                                 },
                                                 |ei, e| Some(e));
        // 2) DFS walk, finding back edges

        // Start with a package with no dependencies inside the component; we may have a choice
        let mut starting_points = Vec::new();
        for node_index in subgraph.node_indices() {
            let (_in_count, out_count) = self.count_edges(node_index);
            if out_count == 0 {
                starting_points.push(node_index)
            }
        }
        let start_node = starting_points[0];
        // Doesn't work because all of this is referencing the original graph, not our subgraph
        // println!("TSWBE: choices {}", self.join_nodes(&starting_points, ","));
        //
        // println!("TSWBE: start {}",
        //         self.ident_for_node(start_node).to_string());

        let mut back_edges = Vec::new();
        depth_first_search(&subgraph, Some(start_node), |event| {
            if let DfsEvent::BackEdge(u, v) = event {
                back_edges.push((u, v));
            }
        });

        // 3) Remove back edges
        for be in back_edges {
            let (u, v) = be;
            let e = subgraph.find_edge(u, v).unwrap();
            subgraph.remove_edge(e);
        }

        // 4) tsort what's left
        if let Ok(vec) = toposort(&subgraph, None) {
            vec
        } else {
            unimplemented!("We should have removed all those cycles")
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
    }
}
