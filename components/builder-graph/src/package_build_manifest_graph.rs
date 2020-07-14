// Copyright (c) 2020-2020 Chef Software Inc. and/or applicable contributors
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

use crate::hab_core::package::{FullyQualifiedPackageIdent,
                               PackageIdent};

use crate::{package_ident_intern::PackageIdentIntern,
            util::*};

use petgraph::graphmap::DiGraphMap;

use std::collections::{HashMap,
                       HashSet};

pub struct PackageBuild {
    pub ident:   PackageIdent,
    pub bt_deps: Vec<PackageIdent>,
    pub rt_deps: Vec<PackageIdent>,
}

impl PackageBuild {
    pub fn format_for_shell(&self) -> String {
        let short_ident = short_ident(&self.ident, false).to_string();
        let deps: Vec<PackageIdent> = self.bt_deps
                                          .iter()
                                          .chain(self.rt_deps.iter())
                                          .cloned()
                                          .collect();
        format!("{}\t{}\t{}\n",
                short_ident,
                self.ident,
                join_idents(",", &deps))
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum UnbuildableReason {
    // Plan not buildable because it is marked as unbuildable in the db
    Direct,
    // Plan depends on something that isn't buildable, but otherwise should be rebuilt
    Indirect,
    // Plan not found in the graph. We quite possibly should *never* mark unbuildable for this
    // reason, but instead forge ahead and treat it as an independent 'treelet' in the graph.
    // This can happen for legitimate reasons, for example a new plan linked to a repo that
    // has never been built and uploaded will never have created a package and so jobsrv will
    // never have included it in its view of the graph. We should still make a best effort build.
    // It may have dependencies, and there is a posibility that the graph isn't technically
    // correct, but still a best effort build will provide dependency info for later builds.
    // It shouldn't have anyone depending on it, unless the graph is outdated, as all
    // dependencies in the graph will create nodes even if their packages haven't been seen.
    Missing,
}

// database entry
// package_build_table {
//     build_ident:      serial,
//     package_ident:    serial,
//     placeholder_name: &str, // or something more strcture
//     actual_name:      &str, // known once built
//     dependencies:     [serial)], /* these may be a mix of placeholders and
//                              * resolved or placeholder only */
//     build_status:     &str, /* external_package, unbuilt_package, in_flight, built_successfully,
//                              * failed */
//     priority: int32,
// }

// This is how nodes in the rebuild graph refer to each other
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, PartialOrd, Ord)]
pub enum UnresolvedPackageIdent {
    Undefined,
    // External nodes (nodes not being rebuilt)
    // These might need some sort of resolution from the latest info to compute the FQPI

    // latest version/latest_release (we may have enough info in the graph to resolve this
    // exactly, but pins complicate)
    ExternalLatestVersion(PackageIdentIntern),
    // pinned_verson/latest_release (cyclic graph might know enough to resolve)
    ExternalPinnedVersion(PackageIdentIntern),
    //  pinned_version/pinned_release (cyclic graph might know enough to resolve)
    ExternalFullyQualified(PackageIdentIntern),
    // Internal nodes (nodes being rebuilt)
    // latest_version/placeholder_release (we won't necessarily know the version, might be updated
    // in plan)
    // The second field refers to the generation; this starts with one, and the max value is
    // likely 3
    InternalNode(PackageIdentIntern, u8),
    InternalVersionedNode(PackageIdentIntern, u8),
}

#[derive(Debug)]
pub struct PackageBuildManifest {
    pub graph: DiGraphMap<UnresolvedPackageIdent, EdgeType>,

    // These amounts to materialized views of the graph above; in that they can be
    // extracted, at a O(n) cost from the graph
    pub external_dependencies: HashSet<PackageIdentIntern>, /* maybe unneeded? New model can
                                                             * find by walking graph */
    // Forensics
    pub input_set:             HashSet<PackageIdentIntern>,
    pub unbuildable_reasons:   HashMap<PackageIdentIntern, UnbuildableReason>,
}

impl PackageBuildManifest {
    pub fn new() -> Self {
        unimplemented!();
    }

    pub fn add_ident(&self, _ident: UnresolvedPackageIdent) { unimplemented!() }

    pub fn list_base_deps() -> PackageIdent { unimplemented!() }

    pub fn resolve_base_dep(_completed: PackageIdent, _package_name: FullyQualifiedPackageIdent) {
        unimplemented!()
    }

    pub fn get_buildable_package() -> PackageBuild { unimplemented!() }

    // Resolved package build record (with all placeholders filled in)
    pub fn mark_package_complete(_completed: PackageIdent,
                                 _package_name: FullyQualifiedPackageIdent) {
        unimplemented!()
    }

    pub fn serialize() -> Vec<PackageBuild> { unimplemented!() }

    pub fn deserialze(_db: Vec<PackageBuild>) { unimplemented!() }
}
