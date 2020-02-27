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

use crate::hab_core;
use crate::hab_core::package::PackageIdent;

use habitat_builder_db::models::package::PackageWithVersionArray;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeType {
    RuntimeDep,
    BuildDep,
}

impl Default for EdgeType {
    fn default() -> Self {
        EdgeType::RuntimeDep
    }
}

pub fn short_ident(ident: &PackageIdent, use_version: bool) -> PackageIdent {
    let parts: Vec<&str> = ident.iter().collect();
    if use_version {
        PackageIdent::new(parts[0], parts[1], Some(parts[2]), None)
    } else {
        PackageIdent::new(parts[0], parts[1], None, None)
    }
}

pub fn join_idents(sep: &str, identlist: &[PackageIdent]) -> String {
    let strings: Vec<String> = identlist.iter().map(PackageIdent::to_string).collect();
    strings.join(sep).to_string()
}
