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

use crate::protocol::originsrv;

use crate::{package_graph_target::PackageGraphForTarget,
            package_graph_trait::{PackageGraphTrait,
                                  Stats},
            package_info::PackageInfo};

struct CyclicPackageGraph {
    graph: PackageGraphForTarget,
}

impl PackageGraphTrait for CyclicPackageGraph {
    fn build(&mut self,
             packages: &[originsrv::OriginPackage],
             use_build_deps: bool)
             -> (usize, usize) {
        let package_info: Vec<PackageInfo> =
            packages.iter().cloned().map(PackageInfo::from).collect();
        self.graph.build(package_info.into_iter(), use_build_deps)
    }

    fn extend(&mut self,
              package: &originsrv::OriginPackage,
              use_build_deps: bool)
              -> (usize, usize) {
        let package_info = PackageInfo::from(package.clone());
        self.graph.extend(&package_info, use_build_deps)
    }

    fn check_extend(&mut self, package: &originsrv::OriginPackage, use_build_deps: bool) -> bool {
        let package_info = PackageInfo::from(package.clone());
        self.graph.check_extend(&package_info, use_build_deps)
    }

    fn rdeps(&self, _name: &str) -> Option<Vec<(String, String)>> {
        unimplemented!("Need to implement a compatible rdeps");
    }

    fn resolve(&self, _name: &str) -> Option<String> {
        unimplemented!("Need to implement a compatible resolve");
    }

    fn stats(&self) -> Stats { self.graph.stats() }
}
