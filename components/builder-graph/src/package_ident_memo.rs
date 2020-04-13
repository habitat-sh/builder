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

use std::io::prelude::*;

use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    str::FromStr,
    string::ToString,
};

use itertools::Itertools;

use crate::{
    hab_core::{
        error as herror,
        package::{ident::Identifiable, PackageIdent},
    },
    util::*,
};

pub struct Tables {
    idents: Vec<Rc<PackageIdent>>;
    

}



#[derive(Debug)]
pub struct Stats {
    pub node_count: usize,
    pub edge_count: usize,
    pub connected_comp: usize,
    pub is_cyclic: bool,
}

#[derive(Eq)]
struct HeapEntry {
    pkg_index: usize,
    rdep_count: usize,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &HeapEntry) -> Ordering {
        self.rdep_count.cmp(&other.rdep_count)
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &HeapEntry) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &HeapEntry) -> bool {
        self.pkg_index == other.pkg_index
    }
}

// Note: We need to filter by target when doing the walk.
type PackageIndex = usize;
#[derive(Debug)]
struct PackageInfo {
    ident: PackageIdent,
    // We may need to create the info record before we see the package data...
    package: Option<PackageWithVersionArray>,

    no_deps: bool,
    plan_deps: Vec<PackageIdent>,
    plan_bdeps: Vec<PackageIdent>,

    full_graph_index: NodeIndex,
}

impl PackageInfo {
    pub fn write(&self) {
        println!("PackageIdent: {}, no_deps: {}", self.ident, self.no_deps);
        if let Some(package_data) = &self.package {
            println!("Target:\t{}", package_data.target.0);
            println!(
                "Deps:\t{}",
                package_data.deps.iter().format_with(", ", |x, f| f(&x.0))
            );
            println!(
                "BDeps:\t{}",
                package_data
                    .build_deps
                    .iter()
                    .format_with(", ", |x, f| f(&x.0))
            );

            println!("Plan Deps:\t{}", join_idents(", ", &self.plan_deps));
            println!("Plan BDeps:\t{}", join_idents(", ", &self.plan_bdeps));
        }
    }

    pub fn extract_plan_deps(&mut self, verbose: bool) {
        // Hoist to lazy static
        lazy_static! {
            // linux builds use backticks around dependency list, while windows doesn't,
            static ref NO_DEPS_RE: Regex =
                Regex::new(r"no (build|runtime) dependencies or undefined").unwrap();
            static ref GEN_DEP_RE: Regex =
                Regex::new(r"^\s*\* __(?P<dtype>Build)?\s*Dependencies__: `?(?P<deps>[^`]*)`?\s*$")
                    .unwrap();
        }

        let package = self.package.as_ref().unwrap();
        let mut found_deps = false;
        let mut found_bdeps = false;

        // investigate RegexSet usage here instead of looping over lines
        for line in package.manifest.lines() {
            if let Some(cap) = GEN_DEP_RE.captures(line) {
                if verbose {
                    println!("{} matched line {}", package.ident.0, line);
                }
                let deplist = cap.name("deps").unwrap().as_str();
                if verbose {
                    println!("{} extracted deps {}", package.ident.0, deplist);
                }
                // Maybe match against regex 'no (build|runtime) dependencies or undefined'
                let mut deps_as_ident = if !deplist.contains("dependencies or undefined") {
                    let deps_conv: herror::Result<Vec<PackageIdent>> = deplist
                        .split_whitespace()
                        .map(PackageIdent::from_str)
                        .collect();

                    deps_conv.unwrap_or_else(|e| {
                        println!("{} ill formed deps {:?}", package.ident.0, e);
                        Vec::new()
                    })
                } else {
                    Vec::new()
                };

                let typeflag;
                if let Some(_deptype) = cap.name("dtype") {
                    typeflag = "B";
                    found_bdeps = true;
                    if verbose {
                        println!(
                            "{} {}: {:?}",
                            package.ident.0,
                            typeflag,
                            join_idents(", ", &deps_as_ident)
                        )
                    };
                    self.plan_bdeps.append(&mut deps_as_ident);
                } else {
                    typeflag = "R";
                    found_deps = true;
                    if verbose {
                        println!(
                            "{} {}: {:?}",
                            package.ident.0,
                            typeflag,
                            join_idents(", ", &deps_as_ident)
                        );
                    };
                    self.plan_deps.append(&mut deps_as_ident);
                }
            }

            // early out; manifests can be large...
            if found_deps && found_bdeps {
                break;
            }
        }

        if !(found_deps && found_bdeps) {
            // Not every package has deps. There are a few classes this falls into:
            // 1) True 'no deps' packages. core/cacerts is a good example
            // 2) User packages statically built using system dependencies. While not 'pure' habitat
            // packages, this is a supported use case and common in windows.
            // 3) Bad packages... There are a lot of packages that don't list deps in their
            // manifest, but have them in their plan file. Or don't list them in their plan file,
            // but use them some how.
            println!(
                "{}: Partial or no deps found for package B: {} R: {}",
                package.ident.0, found_bdeps, found_deps
            );
            self.no_deps = true;
        } else {
            self.no_deps = false;
        }
    }

    #[allow(dead_code)]
    // It might be useful comparing this with the deps extracted from
    // the manifest and alerting when there are differences, as it is
    // either a broken package or something wrong in the code.
    pub fn infer_plan_deps(&mut self) {
        let package = self.package.as_ref().unwrap();

        // It would be more correct to parse the manifest and use that to fill plan_deps/bdeps
        // That may belong inside PackageWithVersionArray
        // For now, just approximate with truncated ident
        for dep in &package.deps {
            self.plan_deps.push(short_ident(&dep.0, false));
        }
        for dep in &package.build_deps {
            self.plan_bdeps.push(short_ident(&dep.0, false));
        }
    }
}

#[derive(Default)]
pub struct PackageGraphForTarget {
    target: Option<PackageTarget>,
    // This is the master data store; all packages live here.
    packages: Vec<RefCell<PackageInfo>>,
    // Maps package ident to position in packages vector above.
    package_map: HashMap<PackageIdent, PackageIndex>,

    // Map from truncated ident to latest matching; it could be origin/packagename, or
    // origin/packagename/version
    latest_map: HashMap<PackageIdent, PackageIndex>,

    // Possible refactor would be to put packageinfo in graph structure; complication is in
    // multigraph situations
    full_graph: StableGraph<PackageIndex, EdgeType>,
    full_graph_node_index_map: HashMap<NodeIndex, PackageIndex>,

    // We build this alongside the full graph
    latest_graph: IdentGraph<PackageIndex>,
}

impl PackageGraphForTarget {
    pub fn new(target: PackageTarget) -> Self {
        let mut pg = PackageGraphForTarget::default();
        pg.target = Some(target);
        pg
    }

    // This is used for testing
    pub fn build<T>(&mut self, packages: T, use_build_deps: bool) -> (usize, usize)
    where
        T: Iterator<Item = PackageWithVersionArray>,
    {
        for p in packages {
            self.extend(&p, use_build_deps);
        }

        (self.full_graph.node_count(), self.full_graph.edge_count())
    }

    fn update_latest<'a>(&'a mut self, id: &'a PackageIdent, index: PackageIndex) {
        let just_package = short_ident(id, false);
        self.update_if_newer(just_package, index);

        let package_version = short_ident(id, true);
        self.update_if_newer(package_version, index);
    }

    fn update_if_newer(&mut self, id: PackageIdent, index: PackageIndex) {
        match self.latest_map.get(&id) {
            Some(&old_index) => {
                if self.packages[index].borrow().ident > self.packages[old_index].borrow().ident {
                    self.latest_map.insert(id, index);
                }
            }
            None => {
                self.latest_map.insert(id, index);
            }
        };
    }

    fn generate_id_for_package(
        &mut self,
        package: &PackageWithVersionArray,
    ) -> (PackageIndex, NodeIndex) {
        let (pi, ni) = self.generate_id(&package.ident.0);

        let mut package_info = self.packages[pi].borrow_mut();

        package_info.package = Some(package.clone());

        package_info.extract_plan_deps(false);

        (pi, ni)
    }

    fn generate_id<'a>(&'a mut self, ident: &PackageIdent) -> (PackageIndex, NodeIndex) {
        if self.package_map.contains_key(&ident) {
            let package_index = self.package_map[&ident];
            let node_index: NodeIndex = self.packages[package_index].borrow().full_graph_index;

            (package_index, node_index)
        } else {
            let package_index = self.packages.len();

            let node_index = self.full_graph.add_node(package_index);
            self.full_graph_node_index_map
                .insert(node_index, package_index);

            let package_info = PackageInfo {
                ident: ident.clone(),
                package: None,
                no_deps: false,
                plan_deps: Vec::new(),
                plan_bdeps: Vec::new(),
                full_graph_index: node_index,
            };

            self.packages.push(RefCell::new(package_info));
            self.package_map.insert(ident.clone(), package_index);
            assert_eq!(self.packages[package_index].borrow().ident, *ident);

            (package_index, node_index)
        }
    }

    pub fn emit_graph(
        &self,
        file: &str,
        origin_filter: Option<&str>,
        latest: bool,
        edge_type: Option<EdgeType>,
    ) {
        let mut file = std::fs::File::create(file).unwrap();
        let filtered_graph: Graph<usize, EdgeType> = self.full_graph.filter_map(
            |node_index, node_data| {
                self.emit_node_filter(node_index, *node_data, origin_filter, latest)
            },
            |edge_index, edge_data| self.emit_edge_filter(edge_index, *edge_data, edge_type),
        );

        file.write_all(
            format!(
                "{:?}",
                Dot::with_config(&filtered_graph, &[Config::EdgeNoLabel])
            )
            .as_bytes(),
        )
        .unwrap();
    }

    pub fn emit_node_filter(
        &self,
        _node_index: NodeIndex,
        node_data: usize,
        _origin_filter: Option<&str>,
        _latest_only: bool,
    ) -> Option<usize> {
        // TODO something smarter here.
        Some(node_data)
    }

    pub fn emit_edge_filter(
        &self,
        _edge_index: EdgeIndex,
        edge_data: EdgeType,
        _wanted_edge: Option<EdgeType>,
    ) -> Option<EdgeType> {
        Some(edge_data)
    }

    // TODO: Need to implement a non-polynomial time check-extend method
    #[allow(clippy::map_entry)]
    pub fn extend(
        &mut self,
        package: &PackageWithVersionArray,
        use_build_deps: bool,
    ) -> ((usize, usize), (usize, usize)) {
        // debug only
        {
            if package.ident.0.name == "gcc" {
                println!("E: {}", package.ident.0);
            }
        }
        let (pkg_id, pkg_node) = self.generate_id_for_package(package);

        assert_eq!(self.target.unwrap(), package.target.0);
        assert_eq!(pkg_id, pkg_node.index());

        // First, add to full graph
        // The full graph should never have
        // cycles, because we forbid them being added abuild things
        // after cutting cycles, but a user might get 'clever' so we
        // still need to check.
        {
            let ident = &package.ident.0;
            assert_eq!(self.packages[pkg_id].borrow().ident, *ident);
            self.update_latest(ident, pkg_id);
        }

        let short_name = short_ident(&self.packages[pkg_id].borrow().ident, false);

        for dep in &package.deps {
            let (_, dep_node) = self.generate_id(&dep.0);
            self.add_edge(pkg_node, dep_node, EdgeType::RuntimeDep);
        }

        if use_build_deps {
            for dep in &package.build_deps {
                let (_, dep_node) = self.generate_id(&dep.0);
                self.add_edge(pkg_node, dep_node, EdgeType::BuildDep);
            }
        }

        // Next, add to latest graph. We overwrite any prior ident, so that any incoming
        // dependencies are preserved.
        //

        // Are we the newest? Ignore older versions of the package
        if self.latest_map[&short_name] == pkg_id {
            // We start by adding a new node with dependencies pointing to the latest of each ident
            let (_, src_node_index) = self.latest_graph.upsert_node(&short_name, pkg_id);

            // Get rid of old edges; the new package version may changed its dependencies.

            // we will need to be checking for cycles here...
            // This drop/re-add process is likely too naive for a future on-line fast cycle
            // detection algorithm; it will introduce too much churn. We'll probably
            // have to move to a difference based system.
            self.latest_graph.drop_outgoing(src_node_index);

            let package_info = self.packages[pkg_id].borrow();

            // I'd like to write this as below, but borrow in closure is problematic.
            // package_info
            //     .plan_deps
            //     .iter()
            //     .for_each(|dep| self.add_edge_to_latest(src_node_index, dep,
            // EdgeType::RuntimeDep));
            for dep in &package_info.plan_deps {
                // skip fully qualified idents in the graph; they never will be t, so they only add
                // noise to the dependency graph
                if !dep.fully_qualified() {
                    self.latest_graph
                        .add_edge(src_node_index, dep, EdgeType::RuntimeDep);
                }
            }

            if use_build_deps {
                for dep in &package_info.plan_bdeps {
                    // skip fully qualified idents in the graph; they never will be t, so they only
                    // add noise to the dependency graph
                    if !dep.fully_qualified() {
                        self.latest_graph
                            .add_edge(src_node_index, dep, EdgeType::BuildDep);
                    }
                }
            }
        }
        (
            (self.full_graph.node_count(), self.full_graph.edge_count()),
            self.latest_graph.counts(),
        )
    }

    pub fn add_edge(&mut self, nfrom: NodeIndex, nto: NodeIndex, etype: EdgeType) {
        self.full_graph.add_edge(nfrom, nto, etype);
        // At some point we'll want to check for cycles, but not for now.
    }

    pub fn rdeps(&self, name: &PackageIdent) -> Option<Vec<(String, String)>> {
        let mut v: Vec<(String, String)> = Vec::new();

        match self.package_map.get(name) {
            Some(&pkg_index) => {
                let pkg_node = self.packages[pkg_index].borrow().full_graph_index;
                match rdeps(&self.full_graph, pkg_node) {
                    Ok(deps) => {
                        for n in deps {
                            let name = &self.packages[n].borrow().ident;
                            let ident = format!("{}", self.latest_map[&name]);
                            let namestr = format!("{}", name);
                            v.push((namestr, ident));
                        }
                    }
                    Err(e) => panic!("Error: {:?}", e),
                }
            }
            None => return None,
        }

        Some(v)
    }

    pub fn write_packages_json(&self, filename: &str, filter: Option<&str>) {
        let mut output: Vec<PackageWithVersionArray> = Vec::new();
        let mut keep = 0;
        let mut m = 0;
        for package_ref in &self.packages {
            if filter_match(&package_ref.borrow().ident, filter) {
                m += 1;
                if package_ref.borrow().ident.name == "gcc" {
                    println!("M: {}", &package_ref.borrow().ident)
                }
                if let Some(_p) = &package_ref.borrow().package {
                    keep += 1;
                    output.push(package_ref.borrow().package.as_ref().unwrap().clone())
                }
            }
        }
        debug!(
            "Wrote {}/{}/{} K/M/T packages with filter {:?}",
            keep,
            m,
            self.packages.len(),
            filter
        );
        write_packages_json(output.into_iter(), filename)
    }

    pub fn read_packages_json(&mut self, filename: &str, use_build_edges: bool) {
        let u = read_packages_json(filename);
        self.build(u.into_iter(), use_build_edges);
    }

    // Mostly for debugging
    pub fn rdeps_dump(&self) {
        debug!("Reverse dependencies:");

        for (pkg_ident, &pkg_index) in &self.package_map {
            let pkg_node = self.packages[pkg_index].borrow().full_graph_index;
            debug!("{}", pkg_ident);

            match rdeps(&self.full_graph, pkg_node) {
                Ok(v) => {
                    for n in v {
                        debug!("|_ {}", self.packages[n].borrow().ident);
                    }
                }
                Err(e) => panic!("Error: {:?}", e),
            }
        }
    }

    pub fn search(&self, phrase: &str) -> Vec<String> {
        let v: Vec<String> = self
            .package_map
            .keys()
            .map(|id| format!("{}", id))
            .filter(|s| s.contains(phrase))
            .collect();

        v
    }

    pub fn latest(&self) -> Vec<String> {
        self.latest_map.values().map(|x| format!("{}", x)).collect()
    }

    // Given an identifier in 'origin/name' format, returns the
    // most recent version (fully-qualified package ident string)
    pub fn resolve(&self, ident: &PackageIdent) -> Option<PackageIdent> {
        let index = self.latest_map.get(ident);
        index.map(|x| self.packages[*x].borrow().ident.clone())
    }

    pub fn stats(&self) -> Stats {
        Stats {
            node_count: self.full_graph.node_count(),
            edge_count: self.full_graph.edge_count(),
            connected_comp: connected_components(&self.full_graph),
            is_cyclic: is_cyclic_directed(&self.full_graph),
        }
    }

    // Who has the most things depending on them?
    pub fn top(&self, max: usize) -> Vec<(String, usize)> {
        let mut v = Vec::new();
        let mut heap = BinaryHeap::new();

        for &package_index in self.latest_map.values() {
            let node_id = self.packages[package_index].borrow().full_graph_index;

            match rdeps(&self.full_graph, node_id) {
                Ok(v) => {
                    let he = HeapEntry {
                        pkg_index: package_index,
                        rdep_count: v.len(),
                    };
                    heap.push(he);
                }
                Err(e) => panic!("Error: {:?}", e),
            }
        }

        let mut i = 0;
        while (i < max) && !heap.is_empty() {
            let he = heap.pop().unwrap();
            v.push((
                self.packages[he.pkg_index].borrow().ident.to_string(),
                he.rdep_count,
            ));
            i += 1;
        }

        v
    }

    pub fn write_deps(&self, ident: &PackageIdent) {
        let maybe_package_index = if ident.fully_qualified() {
            self.package_map.get(&ident)
        } else {
            self.latest_map.get(&ident)
        };
        match maybe_package_index {
            Some(&pi) => self.packages[pi].borrow().write(),
            None => println!("Couldn't find match for {}", ident),
        }
    }

    pub fn dump_graph(&self, _file: &str) {
        println!("dump_graph unimplemented");
    }

    // Output a human readable, machine parsable form of the graph; useful for debugging
    pub fn dump_latest_graph_raw(&self, file: &str, origin: Option<&str>) {
        self.latest_graph.dump_graph_raw(file, origin)
    }

    // The built in Dot utility wasn't flexible for what I wanted, so implemented our own.
    pub fn dump_latest_graph_as_dot(&self, file: &str, origin: Option<&str>) {
        self.latest_graph.emit_graph_as_dot(file, origin)
    }

    pub fn dump_build_levels(&self, file: &str, origin: Option<&str>) {
        self.latest_graph.dump_build_levels(file, origin)
    }

    pub fn dump_scc(&self, file: &str, origin: Option<&str>) {
        self.latest_graph.dump_scc(file, origin)
    }

    // COALFACE
    pub fn dump_diagnostics(&self, file: &str, _origin: Option<&str>) {
        let mut file = std::fs::File::create(file).unwrap();

        let mut package_count = HashMap::<PackageIdent, usize>::new();
        file.write("============package list ======n".as_bytes())
            .unwrap();
        for package in &self.packages {
            file.write(format!("{}\n", package.borrow().ident).as_bytes())
                .unwrap();
            let count = package_count
                .entry(short_ident(&package.borrow().ident, false))
                .or_insert(0);
            *count += 1;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util;

    #[test]
    fn write_restore_packages() {
        let empty: [&str; 0] = [];

        let package1 = util::mk_package_with_versionarray(
            "foo/bar/1/2",
            "x86_64-linux",
            &["foo/baz/1/2"],
            &empty,
        );

        let mut vec = Vec::new();
        vec.push(package1);

        let tmpfile = "/tmp/junk"; // Do this smarter
        let mut graph =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        graph.build(vec.into_iter(), true);

        graph.write_packages_json(tmpfile, None);

        let mut graph2 =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        graph2.read_packages_json(tmpfile, true);
        let stats = graph2.stats();
        assert_eq!(stats.node_count, 2);
    }

    use habitat_builder_db::models::package::{
        BuilderPackageIdent, BuilderPackageTarget, PackageVisibility, PackageWithVersionArray,
    };

    // TODO RE-ENABLE once circular deps detection is fixed.
    // However that's going to require some serious rework of the code.
    // #[test]
    #[allow(dead_code)]
    fn disallow_circular_dependency() {
        let mut graph =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        let mut packages = Vec::new();
        let empty: [&str; 0] = [];

        let package1 = util::mk_package_with_versionarray(
            "foo/bar/1/2",
            "x86_64-linux",
            &["foo/baz/1/2"],
            &empty,
        );
        packages.push(package1);
        let package2 = util::mk_package_with_versionarray(
            "foo/baz/1/2",
            "x86_64-linux",
            &["foo/bar/1/2"],
            &empty,
        );
        packages.push(package2);

        let (ncount, ecount) = graph.build(packages.into_iter(), true);

        assert_eq!(ncount, 2);
        assert_eq!(ecount, 1); // only the first edge added

        let stats = graph.stats();
        assert_eq!(stats.is_cyclic, false);

        // let pre_check = graph.check_extend(&package2, true);
        // assert_eq!(pre_check, false);
    }

    #[test]
    fn pre_check_with_dep_not_present() {
        let mut graph =
            PackageGraphForTarget::new(PackageTarget::from_str("x86_64-linux").unwrap());
        let empty: [&str; 0] = [];

        let package1 = util::mk_package_with_versionarray(
            "foo/bar/1/2",
            "x86_64-linux",
            &["foo/baz/1/2"],
            &empty,
        );

        let package2 = util::mk_package_with_versionarray(
            "foo/baz/1/2",
            "x86_64-linux",
            &["foo/xyz/1/2"],
            &empty,
        );

        // let pre_check1 = graph.check_extend(&package1, true);
        // assert_eq!(pre_check1, true);

        let (..) = graph.extend(&package1, true);

        // let pre_check2 = graph.check_extend(&package2, true);
        // assert_eq!(pre_check2, true);

        let (..) = graph.extend(&package2, true);
    }
}
