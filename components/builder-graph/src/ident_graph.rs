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
//
//

use petgraph::{graph::NodeIndex, Graph};

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use crate::hab_core::package::PackageIdent;
use crate::util::*;

type IdentIndex = usize;

#[derive(Default)]
struct IdentMemo {
    // It would be nice not to have two copies of Ident
    idents: Vec<PackageIdent>,
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

    pub fn get_ident<'a>(&'a self, index: IdentIndex) -> &'a PackageIdent {
        &self.idents[index]
    }

    // TODO: maybe helper fn to compare/sort by index.
}

struct IdentGraphElement<Value> {
    ident_index: IdentIndex,
    node_index: NodeIndex,
    value: Value,
}

// IdentGraph allows us to map an Ident to a graph node, and update a value for that node
// petgraph doesn't allow value updating...
#[derive(Default)]
pub struct IdentGraph<Value> {
    data: Vec<IdentGraphElement<Value>>,
    graph: Graph<IdentIndex, EdgeType>,
    ident_memo: IdentMemo,
}

impl<Value> IdentGraph<Value>
where
    Value: Default + Copy,
{
    pub fn new() -> Self {
        IdentGraph::default()
    }

    fn get_node_by_id(&mut self, ident: &PackageIdent) -> (IdentIndex, NodeIndex, Value) {
        let ident_index = self.ident_memo.index_for_ident(ident);

        if ident_index == self.data.len() {
            let node_index = self.graph.add_node(ident_index);
            assert_eq!(node_index.index(), ident_index);

            let value = Default::default();
            self.data.push(IdentGraphElement {
                ident_index,
                node_index,
                value,
            });
            (ident_index, node_index, value)
        } else {
            let IdentGraphElement {
                ident_index: expected_index,
                node_index,
                value,
            } = self.data[ident_index];
            assert_eq!(expected_index, ident_index);
            (ident_index, node_index, value)
        }
    }

    pub fn get_node(&mut self, ident: &PackageIdent) -> (NodeIndex, Value) {
        let (_ident_index, node_index, value) = self.get_node_by_id(&ident);
        (node_index, value)
    }

    pub fn upsert_node(&mut self, ident: &PackageIdent, value: Value) -> (IdentIndex, NodeIndex) {
        // Replace node in place (update value) We replace nodes in
        // place because deleting a node can cause renumbering, and it
        // preserves the in-edges for free.
        let (ident_index, node_index, _value) = self.get_node_by_id(ident);
        self.data[ident_index].value = value;
        (ident_index, node_index)
    }

    pub fn ident_for_node<'a>(&'a self, node: NodeIndex) -> &'a PackageIdent {
        //             let ident_index = self.graph.node_weight(node_index).unwrap();
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

    pub fn counts(&self) -> (usize, usize) {
        (self.graph.node_count(), self.graph.edge_count())
    }

    pub fn emit_graph(&self, filename: &str, origin_filter: Option<&str>) {
        let path = Path::new(filename);
        let mut file = File::create(&path).unwrap();

        writeln!(&mut file, "// Filtered by {:?}", origin_filter).unwrap();

        writeln!(&mut file, "digraph "\{}\" {{", filename).unwrap();
        writeln!(&mut file, "    rankdir=\"UD\";").unwrap();
        // iterate through nodes

        for node_index in self.graph.node_indices() {
            let node = self.ident_for_node(node_index);
            if filter_match(node, origin_filter) {
                let node_name = node.to_string();
                writeln!(&mut file, "    \"{}\"", node_name).unwrap();
            }
        }

        // iterate through regular edges
        writeln!(&mut file, "//######## RUN TIME EDGES ######").unwrap();
        self.write_edges(&mut file, EdgeType::RuntimeDep, origin_filter);

        writeln!(&mut file, "//######## BUILD TIME EDGES ######").unwrap();
        writeln!(
            &mut file,
            "    edge [ color = \"blue\" style = \"dashed\" constraint = false ];"
        )
        .unwrap();

        // iterate through build edges
        self.write_edges(&mut file, EdgeType::BuildDep, origin_filter);

        // close
        writeln!(&mut file, "}}").unwrap();
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
}

fn write_edge(file: &mut File, src: &str, dst: &str, edge_type: Option<EdgeType>) {
    match edge_type {
        Some(etype) => writeln!(
            file,
            "    \"{}\" -> \"{}\" [type=\"{}\"];",
            src,
            dst,
            edgetype_to_abbv(etype)
        )
        .unwrap(),
        None => writeln!(file, "    \"{}\" -> \"{}\"", src, dst).unwrap(),
    }
}

fn edgetype_to_abbv(edge: EdgeType) -> &'static str {
    match edge {
        EdgeType::RuntimeDep => "R",
        EdgeType::BuildDep => "B",
    }
}

fn filter_match(ident: &PackageIdent, filter: Option<&str>) -> bool {
    if let Some(origin) = filter {
        if ident.origin == origin {
            return true;
        }
    }
    false
}
