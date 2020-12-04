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

use crate::hab_core::package::Identifiable;

use crate::{package_ident_intern::PackageIdentIntern,
            util::*};

use petgraph::graphmap::DiGraphMap;

use std::{collections::{HashMap,
                        HashSet},
          fmt};

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
    // For now, we're only marking missing if it wasn't in the touched set; that covers the above
    // case, but might never happen otherwise.
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

impl UnresolvedPackageIdent {
    pub fn ident(&self) -> PackageIdentIntern {
        match self {
            UnresolvedPackageIdent::ExternalLatestVersion(ident)
            | UnresolvedPackageIdent::ExternalPinnedVersion(ident)
            | UnresolvedPackageIdent::ExternalFullyQualified(ident) => *ident,
            UnresolvedPackageIdent::InternalNode(ident, _)
            | UnresolvedPackageIdent::InternalVersionedNode(ident, _) => *ident,
        }
    }

    pub fn to_unbuilt_ident(&self) -> PackageIdentIntern {
        match self {
            UnresolvedPackageIdent::ExternalLatestVersion(ident)
            | UnresolvedPackageIdent::ExternalPinnedVersion(ident)
            | UnresolvedPackageIdent::ExternalFullyQualified(ident) => *ident,
            UnresolvedPackageIdent::InternalNode(ident, n) => {
                PackageIdentIntern::new(ident.origin(),
                                        ident.name(),
                                        Some("(LATEST)"),
                                        Some(&format!("(UNBUILT_INSTANCE)-{}", n)))
            }
            UnresolvedPackageIdent::InternalVersionedNode(ident, n) => {
                PackageIdentIntern::new(ident.origin(),
                                        ident.name(),
                                        ident.version(),
                                        Some(&format!("(UNBUILT_INSTANCE)-{}", n)))
            }
        }
    }

    pub fn is_internal_node(&self) -> bool {
        match self {
            UnresolvedPackageIdent::InternalNode(..)
            | UnresolvedPackageIdent::InternalVersionedNode(..) => true,
            _ => false,
        }
    }
}

impl fmt::Display for UnresolvedPackageIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnresolvedPackageIdent::ExternalLatestVersion(ident)
            | UnresolvedPackageIdent::ExternalPinnedVersion(ident)
            | UnresolvedPackageIdent::ExternalFullyQualified(ident) => write!(f, "Ext:{}", ident),
            UnresolvedPackageIdent::InternalNode(ident, version)
            | UnresolvedPackageIdent::InternalVersionedNode(ident, version) => {
                write!(f, "Int:{}:{}", ident, version)
            }
        }
    }
}

pub struct PackageBuild {
    pub name:                 UnresolvedPackageIdent,
    pub runtime_deps:         Vec<UnresolvedPackageIdent>,
    pub build_deps:           Vec<UnresolvedPackageIdent>,
    pub strong_deps:          Vec<UnresolvedPackageIdent>,
    pub external_constraints: Vec<UnresolvedPackageIdent>,
}

impl PackageBuild {
    pub fn all_deps(&self) -> impl Iterator<Item = &UnresolvedPackageIdent> {
        self.runtime_deps.iter().chain(self.build_deps
                                           .iter()
                                           .chain(self.strong_deps
                                                      .iter()
                                                      .chain(self.external_constraints.iter())))
    }

    // Excludes various types of synthetic deps
    pub fn natural_deps(&self) -> impl Iterator<Item = &UnresolvedPackageIdent> {
        self.runtime_deps.iter().chain(self.build_deps.iter())
    }

    pub fn format_for_shell(&self) -> String {
        let short_ident = &self.name.ident().short_ident().to_string();
        let deps: Vec<UnresolvedPackageIdent> = self.all_deps().cloned().collect();
        format!("{}\t{}\t{}\n",
                short_ident,
                self.name,
                join_idents(",", &deps))
    }
}

/// Represents the transformed graph of packages
///
/// In the presence of cycles we need to transform the graph to make it buildable. At this point
/// we've unrolled the loops and otherwise fixed up any hidden dependencies. A topological sort
/// of the runtime dependency edges will yield a correct build order.
///
/// This is the main output of our build order resolution phase, and is consumed by the
/// scheduler.

#[derive(Debug, Default)]
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
    pub fn new() -> Self { PackageBuildManifest::default() }

    pub fn build_order(&self) -> Vec<PackageBuild> {
        let mut order: Vec<PackageBuild> = Vec::new();

        // doing this for the free topological sort, not for any SCC data
        // This had better be a DAG by now or we in very deep trouble
        for component in petgraph::algo::tarjan_scc(&self.graph) {
            assert_eq!(component.len(), 1);

            match component.first().unwrap() {
                ident @ UnresolvedPackageIdent::InternalNode(..)
                | ident @ UnresolvedPackageIdent::InternalVersionedNode(..) => {
                    let package_build = self.package_build_from_unresolved_ident(*ident);
                    order.push(package_build);
                }
                _ => (),
            }
        }

        order
    }

    /// Fixup for strict package build ordering.
    // The execution ordering of the base graph is specified only by the direct dependencies of the
    // package. However, our build workers don't use the dependencies to select what package to use,
    // instead taking the latest package in the channel. This creates an antidependency (read
    // before write) as it is possible that we build the next iteration of a package before all
    // of the consumers of the last iteration have started; if that happens those packages might
    // pick up the wrong iteration. This is likely harmless, except it makes the process
    // nondeterministic and hard to debug. Constraining this will protect against this
    // nondeterminism at the cost of some parallelism.
    //
    // To counter this, we will add extra dependencies to the graph. A package iteration n
    // (InternalVersionedNode) will now have dependencies on all of the consumers of iteration n-1,
    // guaranteeing they complete before it starts
    //
    // This fixup will not be necessary once we have build workers that can take exact dependencies.
    //
    pub fn constrain_package_cycles(&mut self) {
        // Phase one: Identify all of the nodes needing constraint. This will be all
        // InternalVersionedNode with version > 1
        let mut fixup_targets = Vec::new();
        for node in self.graph.nodes() {
            match node {
                UnresolvedPackageIdent::InternalVersionedNode(_, n) if n > 1 => {
                    fixup_targets.push(node);
                }
                _ => (),
            }
        }

        let mut edges_added = 0;
        // Phase two: For each identified node, find the n-1 th version and make each package that
        // depends on the n-1th node a dependency of the nth node.
        for node in fixup_targets.iter() {
            if let UnresolvedPackageIdent::InternalVersionedNode(ident, n) = node {
                // always matches...
                let prev_node = UnresolvedPackageIdent::InternalVersionedNode(*ident, n - 1);
                // Modifying the graph while iterating over edges isn't ok.
                let consumers: Vec<UnresolvedPackageIdent> =
                    self.graph
                        .neighbors_directed(prev_node, petgraph::EdgeDirection::Incoming)
                        .collect();
                for consumer in consumers {
                    self.graph
                        .add_edge(*node, consumer, EdgeType::ExternalConstraint);
                    edges_added += 1;
                }
            }
        }

        info!("constrain_package_cycles added {} fixup edges", edges_added)
    }

    fn package_build_from_unresolved_ident(&self, name: UnresolvedPackageIdent) -> PackageBuild {
        let mut runtime_deps = Vec::new();
        let mut build_deps = Vec::new();
        let mut strong_deps = Vec::new();
        let mut external_constraints = Vec::new();

        for dep in self.graph
                       .neighbors_directed(name, petgraph::EdgeDirection::Outgoing)
        {
            match self.graph.edge_weight(name, dep).unwrap() {
                EdgeType::RuntimeDep => runtime_deps.push(dep),
                EdgeType::BuildDep => build_deps.push(dep),
                EdgeType::StrongBuildDep => strong_deps.push(dep),
                EdgeType::ExternalConstraint => external_constraints.push(dep),
            }
        }

        PackageBuild { name,
                       runtime_deps,
                       build_deps,
                       strong_deps,
                       external_constraints }
    }
}
