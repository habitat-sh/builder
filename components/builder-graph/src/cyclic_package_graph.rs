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

use std::{str::FromStr,
          string::ToString};

use crate::protocol::originsrv;

use crate::{data_store::Unbuildable,
            error::Result,
            hab_core::package::{PackageIdent,
                                PackageTarget},
            package_build_manifest_graph::PackageBuildManifest,
            package_graph_target::PackageGraphForTarget,
            package_graph_trait::{PackageGraphTrait,
                                  Stats},
            package_ident_intern::PackageIdentIntern,
            package_info::PackageInfo};

pub struct CyclicPackageGraph {
    graph: PackageGraphForTarget,
}

impl CyclicPackageGraph {
    pub fn new(target: PackageTarget) -> Self {
        CyclicPackageGraph { graph: PackageGraphForTarget::new(target), }
    }
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

    fn rdeps(&self, name: &str) -> Option<Vec<(String, String)>> {
        let ident = PackageIdentIntern::from_str(name);
        ident.ok().map(|r| {
                      self.graph
                          .rdeps(r, None)
                          .iter()
                          .map(|(dep, fq_dep)| (dep.to_string(), fq_dep.to_string()))
                          .collect()
                  })
    }

    fn resolve(&self, name: &str) -> Option<String> {
        let ident = PackageIdent::from_str(name);
        ident.ok()
             .map(|r| self.graph.resolve(r.as_ref()).map(|r| r.to_string()))
             .flatten()
    }

    fn stats(&self) -> Stats { self.graph.stats() }

    fn compute_build(&self,
                     touched: &[PackageIdentIntern],
                     unbuildable: &dyn Unbuildable)
                     -> Result<PackageBuildManifest> {
        Ok(self.graph.compute_build(touched, unbuildable))
    }

    //  maybe look to implement this as part of serialization
    fn as_json(&self) -> String { self.graph.as_json() }
}
