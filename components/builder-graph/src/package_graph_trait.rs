// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
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

use crate::hab_core::package::{PackageIdent,
                               PackageTarget};

#[derive(Debug)]
pub struct Stats {
    pub node_count:     usize,
    pub edge_count:     usize,
    pub connected_comp: usize,
    pub is_cyclic:      bool,
}

pub trait PackageDepsTrait {
    fn package_deps(&self, package: &PackageIdent, target: &PackageTarget) -> Vec<PackageIdent>;
}

pub trait PackageGraphTrait: Send + Sync {
    fn build(&mut self,
             packages: &[originsrv::OriginPackage],
             use_build_deps: bool)
             -> (usize, usize);
    fn extend(&mut self,
              package: &originsrv::OriginPackage,
              use_build_deps: bool)
              -> (usize, usize);
    fn check_extend(&mut self, package: &originsrv::OriginPackage, use_build_deps: bool) -> bool;
    fn rdeps(&self, name: &str) -> Option<Vec<(String, String)>>;
    fn rdeps_group(&self,
                   name: &str,
                   package_deps: &dyn PackageDepsTrait)
                   -> Option<Vec<Vec<PackageIdent>>>;
    fn resolve(&self, name: &str) -> Option<String>;
    fn stats(&self) -> Stats;
}
