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

use std::{borrow::BorrowMut,
          collections::HashMap,
          str::FromStr};

use crate::{acyclic_package_graph::AcyclicPackageGraph,
            cyclic_package_graph::CyclicPackageGraph,
            hab_core::package::PackageTarget,
            package_graph_trait::PackageGraphTrait,
            protocol::originsrv};

pub struct TargetGraphStats {
    pub target:     PackageTarget,
    pub node_count: usize,
    pub edge_count: usize,
}

#[derive(Default)]
pub struct TargetGraph {
    graphs: HashMap<PackageTarget, Box<dyn PackageGraphTrait>>,
}

impl TargetGraph {
    pub fn new(use_cyclic_graph: bool) -> Self {
        let mut graphs: HashMap<PackageTarget, Box<dyn PackageGraphTrait>> = HashMap::new();

        // We only support the following targets currently
        for target_str in &["x86_64-linux", "x86_64-linux-kernel2", "x86_64-windows"] {
            let target = PackageTarget::from_str(target_str).unwrap();
            let graph: Box<dyn PackageGraphTrait> = if use_cyclic_graph {
                Box::new(CyclicPackageGraph::new(target))
            } else {
                Box::new(AcyclicPackageGraph::new(target))
            };
            graphs.insert(target, graph);
        }

        TargetGraph { graphs }
    }

    pub fn graph(&self, target_str: &str) -> Option<&dyn PackageGraphTrait> {
        match PackageTarget::from_str(target_str) {
            Ok(target) => self.graphs.get(&target).map(|x| x.as_ref()),
            Err(err) => {
                error!("Invalid target specified for TargetGraph: {}! Err: {}",
                       target_str, err);
                None
            }
        }
    }

    pub fn graph_for_target(&self, target: PackageTarget) -> Option<&dyn PackageGraphTrait> {
        self.graphs.get(&target).map(|x| x.as_ref())
    }

    pub fn graph_mut(&mut self,
                     target_str: &str)
                     -> Option<&mut (dyn PackageGraphTrait + 'static)> {
        match PackageTarget::from_str(target_str) {
            Ok(target) => self.graphs.get_mut(&target).map(|x| x.borrow_mut()),
            Err(err) => {
                error!("Invalid target specified for TargetGraph: {}! Err: {}",
                       target_str, err);
                None
            }
        }
    }

    pub fn build(&mut self,
                 packages: &[originsrv::OriginPackage],
                 use_build_deps: bool)
                 -> Vec<TargetGraphStats> {
        for p in packages {
            if let Some(ref mut graph) = self.graph_mut(p.get_target()) {
                graph.extend(&p, use_build_deps);
            }
        }

        let mut target_stats = Vec::new();
        for (target, graph) in self.graphs.iter() {
            let stats = graph.stats();
            let ts = TargetGraphStats { target:     *target,
                                        node_count: stats.node_count,
                                        edge_count: stats.edge_count, };
            target_stats.push(ts);
        }

        target_stats
    }
}
