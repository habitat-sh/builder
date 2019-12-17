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

#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate features;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

extern crate diesel;

//use builder_core as bldr_core;
use habitat_builder_db as db;
use habitat_builder_protocol as protocol;
use habitat_core as hab_core;

pub mod config;
pub mod data_store;
pub mod error;
pub mod ident_graph;
pub mod package_graph;
pub mod rdeps;
pub mod util;

use std::{collections::HashMap,
          fs::File,
          io::Write,
          iter::FromIterator,
          time::Instant,
          str::FromStr};

use clap::{App, Arg};
use copperline::Copperline;

use crate::{
    config::Config,
    data_store::DataStore,
    hab_core::config::ConfigFile,
    hab_core::package::{PackageIdent, PackageTarget},
    package_graph::PackageGraph,
};

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

fn main() {
    env_logger::init();

    let matches = App::new("bldr-graph")
        .version(VERSION)
        .about("Habitat Graph Dev Tool")
        .arg(
            Arg::with_name("config")
                .help("Filepath to configuration file")
                .required(false)
                .index(1),
        )
        .get_matches();

    let config = match matches.value_of("config") {
        Some(cfg_path) => Config::from_file(cfg_path).unwrap(),
        None => Config::default(),
    };

    enable_features(&config);

    let mut cl = Copperline::new();

    println!("Connecting to {}", config.datastore.database);

    let datastore = DataStore::new(&config);
    datastore.setup().unwrap();

    println!("Building graph... please wait.");

    let mut graph = PackageGraph::new();
    let start_time = Instant::now();
    let packages = datastore.get_job_graph_packages().unwrap();
    let fetch_time = start_time.elapsed().as_secs();
    println!("OK: fetched {} packages ({} sec)",
             packages.len(),
             fetch_time);

    let start_time = Instant::now();
    let (ncount, ecount) = graph.build(packages.into_iter(), feat::is_enabled(feat::BuildDeps));
    println!("OK: {} nodes, {} edges ({} sec)",
             ncount,
             ecount,
             start_time.elapsed().as_secs());

    let targets = graph.targets();
    let target_as_string: Vec<String> = targets.iter().map(|t| t.to_string()).collect();

    println!("Found following targets {}", target_as_string.join(", "));
    println!("Default target is {}", graph.current_target());

    println!(
        "\nAvailable commands: help, stats, top, find, resolve, filter, rdeps, deps, check, \
         exit\n",
    );

    let mut filter = String::from("");
    let mut done = false;

    while !done {
        let prompt = format!("{}: command> ", graph.current_target());
        let line = cl.read_line_utf8(&prompt).ok();
        if line.is_none() {
            continue;
        }
        let cmd = line.expect("Could not get line");
        cl.add_history(cmd.clone());

        let v: Vec<&str> = cmd.trim_end().split_whitespace().collect();

        if !v.is_empty() {
            match v[0].to_lowercase().as_str() {
                "help" => do_help(),
                "all_stats" => do_all_stats(&graph),
                "stats" => do_stats(&graph),
                "top" => {
                    let count = if v.len() < 2 {
                        10
                    } else {
                        v[1].parse::<usize>().unwrap()
                    };
                    do_top(&graph, count);
                }
                "filter" => {
                    if v.len() < 2 {
                        filter = String::from("");
                        println!("Removed filter\n");
                    } else {
                        filter = String::from(v[1]);
                        println!("New filter: {}\n", filter);
                    }
                }
                "find" => {
                    if v.len() < 2 {
                        println!("Missing search term\n")
                    } else {
                        let max = if v.len() > 2 {
                            v[2].parse::<usize>().unwrap()
                        } else {
                            10
                        };
                        do_find(&graph, v[1].to_lowercase().as_str(), max)
                    }
                }
                "dot" => {
                    if v.len() < 2 {
                        println!("Missing search term\n")
                    } else {
                        let origin = if v.len() > 2 { Some(v[2]) } else { None };
                        do_dot(&graph, v[1], origin)
                    }
                }
                "latest_dot" => {
                    if v.len() < 3 {
                        println!("Missing search term\n")
                    } else {
                        let origin = if v.len() > 2 { Some(v[2]) } else { None };
                        do_latest_dot(&graph, v[1], origin)
                    }
                }
                "resolve" => {
                    if v.len() < 2 {
                        println!("Missing package name\n")
                    } else {
                        do_resolve(&graph, v[1].to_lowercase().as_str())
                    }
                }
                "rdeps" => {
                    if v.len() < 2 {
                        println!("Missing package name\n")
                    } else {
                        let max = if v.len() > 2 {
                            v[2].parse::<usize>().unwrap()
                        } else {
                            10
                        };
                        do_rdeps(&graph, v[1].to_lowercase().as_str(), &filter, max)
                    }
                }
                "deps" => {
                    if v.len() < 2 {
                        println!("Missing package name\n")
                    } else {
                        do_deps(&datastore, &graph, v[1].to_lowercase().as_str(), &filter)
                    }
                }
                "check" => {
                    if v.len() < 2 {
                        println!("Missing package name\n")
                    } else {
                        do_check(&datastore, &graph, v[1].to_lowercase().as_str(), &filter)
                    }
                }
                "export" => {
                    if v.len() < 2 {
                        println!("Missing file name\n")
                    } else {
                        do_export(&graph, v[1].to_lowercase().as_str(), &filter)
                    }
                }
                "target" => do_target(&mut graph, v[1]),
                "quit" | "exit" => done = true,
                _ => println!("Unknown command\n"),
            }
        }
    }
}

fn do_help() {
    println!("Commands:");
    println!("  help                    Print this message");
    println!("  stats                   Print graph statistics");
    println!("  top     [<count>]       Print nodes with the most reverse dependencies");
    println!("  filter  [<origin>]      Filter outputs to the specified origin");
    println!("  resolve <name>          Find the most recent version of the package 'origin/name'");
    println!("  find    <term> [<max>]  Find packages that match the search term, up to max items");
    println!("  rdeps   <name> [<max>]  Print the reverse dependencies for the package, up to max");
    println!("  deps    <name>|<ident>  Print the forward dependencies for the package");
    println!("  check   <name>|<ident>  Validate the latest dependencies for the package");
    println!("  export  <filename>      Export data from graph to specified file");
    println!("  exit                    Exit the application\n");
}

fn do_all_stats(graph: &PackageGraph) {
    let stats = graph.all_stats();
    for (target, stat) in stats {
        println!("Target: {}", target);
        println!("  Node count: {}", stat.node_count);
        println!("  Edge count: {}", stat.edge_count);
        println!("  Connected components: {}", stat.connected_comp);
        println!("  Is cyclic: {}", stat.is_cyclic)
    }
}

fn do_stats(graph: &PackageGraph) {
    let stats = graph.stats();

    println!("Node count: {}", stats.node_count);
    println!("Edge count: {}", stats.edge_count);
    println!("Connected components: {}", stats.connected_comp);
    println!("Is cyclic: {}", stats.is_cyclic);
}

fn do_top(graph: &PackageGraph, count: usize) {
    let start_time = Instant::now();
    let top = graph.top(count);

    println!("OK: {} items ({} sec)\n",
             top.len(),
             start_time.elapsed().as_secs());

    for (name, count) in top {
        println!("{}: {}", name, count);
    }
    println!();
}

fn do_find(graph: &PackageGraph, phrase: &str, max: usize) {
    let start_time = Instant::now();
    let mut v = graph.search(phrase);

    println!("OK: {} items ({} sec)\n",
             v.len(),
             start_time.elapsed().as_secs());

    if v.is_empty() {
        println!("No matching packages found")
    } else {
        if v.len() > max {
            v.drain(max..);
        }
        for s in v {
            println!("{}", s);
        }
    }

    println!();
}

fn do_latest_dot(graph: &PackageGraph, filename: &str, origin: Option<&str>) {
    let start_time = PreciseTime::now();
    graph.dump_latest_graph(filename, origin);
    let end_time = PreciseTime::now();
    println!(
        "Wrote latest graph to file {} filtered by {:?} TBI in {} sec",
        filename,
        origin,
        start_time.to(end_time)
    );
}

fn do_dot(graph: &PackageGraph, filename: &str, origin: Option<&str>) {
    let start_time = PreciseTime::now();
    graph.emit_graph(filename, None, true, None);
    let end_time = PreciseTime::now();
    println!(
        "Wrote graph to file {} filtered by {:?} TBI in {} sec",
        filename,
        origin,
        start_time.to(end_time)
    );
}

fn do_resolve(graph: &PackageGraph, name: &str) {
    let start_time = Instant::now();
    let result = graph.resolve(name);

    println!("OK: ({} sec)\n", start_time.elapsed().as_secs());

    match result {
        Some(s) => println!("{}", s),
        None => println!("No matching packages found"),
    }

    println!();
}

fn do_rdeps(graph: &PackageGraph, name: &str, filter: &str, max: usize) {
    let start_time = Instant::now();

    let ident = PackageIdent::from_str(name).unwrap();

    match graph.rdeps(&ident) {
        Some(rdeps) => {
            let duration_secs = start_time.elapsed().as_secs();
            let mut filtered: Vec<(String, String)> =
                rdeps.into_iter()
                     .filter(|&(ref x, _)| x.starts_with(filter))
                     .collect();

            println!("OK: {} items ({} sec)\n", filtered.len(), duration_secs);

            if filtered.len() > max {
                filtered.drain(max..);
            }

            if !filter.is_empty() {
                println!("Results filtered by: {}", filter);
            }

            for (s1, s2) in filtered {
                println!("{} ({})", s1, s2);
            }
        }
        None => println!("No entries found"),
    }

    println!();
}

fn resolve_name(graph: &PackageGraph, name: &str) -> String {
    let parts: Vec<&str> = name.split('/').collect();
    if parts.len() == 2 {
        match graph.resolve(name) {
            Some(s) => s,
            None => String::from(name),
        }
    } else {
        String::from(name)
    }
}

fn do_deps(datastore: &DataStore, graph: &PackageGraph, name: &str, filter: &str) {
    let start_time = Instant::now();
    let ident = resolve_name(graph, name);
    let target = "x86_64-linux";

    println!("Dependencies for: {}", ident);

    match datastore.get_job_graph_package(&ident, &target) {
        Ok(package) => {
            println!("OK: {} items ({} sec)\n",
                     package.get_deps().len(),
                     start_time.elapsed().as_secs());
            if !filter.is_empty() {
                println!("Results filtered by: {}\n", filter);
            }

            for dep in package.get_deps() {
                if dep.to_string().starts_with(filter) {
                    println!("{}", dep)
                }
            }
        }
        Err(_) => println!("No matching package found"),
    }

    println!();
}

fn short_name(ident: &str) -> String {
    let parts: Vec<&str> = ident.split('/').collect();
    assert!(parts.len() >= 2);
    format!("{}/{}", parts[0], parts[1])
}

fn do_check(datastore: &DataStore, graph: &PackageGraph, name: &str, filter: &str) {
    let start_time = Instant::now();
    let mut deps_map = HashMap::new();
    let mut new_deps = Vec::new();
    let ident = resolve_name(graph, name);
    let target = "x86_64-linux";

    match datastore.get_job_graph_package(&ident, &target) {
        Ok(package) => {
            if !filter.is_empty() {
                println!("Checks filtered by: {}\n", filter);
            }

            println!("Dependecy version updates:");
            for dep in package.get_deps() {
                if dep.to_string().starts_with(filter) {
                    let dep_name = short_name(&dep.to_string());
                    let dep_latest = resolve_name(graph, &dep_name);
                    deps_map.insert(dep_name, dep_latest.clone());
                    new_deps.push(dep_latest.clone());
                    println!("{} -> {}", dep, dep_latest);
                }
            }

            println!();

            for new_dep in new_deps {
                check_package(datastore, &mut deps_map, &new_dep, filter);
            }
        }
        Err(_) => println!("No matching package found"),
    }

    println!("\nTime: {} sec\n", start_time.elapsed().as_secs());
}

fn check_package(
    datastore: &DataStore,
    deps_map: &mut HashMap<String, String>,
    ident: &str,
    filter: &str,
) {
    let target = "x86_64-linux";
    match datastore.get_job_graph_package(ident, target) {
        Ok(package) => {
            for dep in package.get_deps() {
                if dep.to_string().starts_with(filter) {
                    let name = short_name(&dep.to_string());
                    {
                        let entry = deps_map.entry(name).or_insert_with(|| dep.to_string());
                        if *entry != dep.to_string() {
                            println!("Conflict: {}", ident);
                            println!("  {}", *entry);
                            println!("  {}", dep);
                        }
                    }
                    check_package(datastore, deps_map, &dep.to_string(), filter);
                }
            }
        }
        Err(_) => println!("No matching package found for {}", ident),
    };
}

fn do_export(graph: &PackageGraph, filename: &str, filter: &str) {
    let start_time = Instant::now();
    let latest = graph.latest();
    println!("\nTime: {} sec\n", start_time.elapsed().as_secs());

    let mut file = File::create(filename).expect("Failed to initialize file");

    if !filter.is_empty() {
        println!("Checks filtered by: {}\n", filter);
    }

    for ident in latest {
        if ident.starts_with(filter) {
            file.write_fmt(format_args!("{}\n", ident)).unwrap();
        }
    }
}

fn do_target(graph: &mut PackageGraph, target: &str) {
    match PackageTarget::from_str(target) {
        Ok(package_target) => graph.set_target(package_target),
        Err(_) => println!("{} is not a valid target", target),
    }
}

fn enable_features(config: &Config) {
    let features: HashMap<_, _> = HashMap::from_iter(vec![("BUILDDEPS", feat::BuildDeps)]);
    let features_enabled = config
        .features_enabled
        .split(',')
        .map(|f| f.trim().to_uppercase());

    for key in features_enabled {
        if features.contains_key(key.as_str()) {
            info!("Enabling feature: {}", key);
            feat::enable(features[key.as_str()]);
        }
    }

    if feat::is_enabled(feat::List) {
        println!("Listing possible feature flags: {:?}", features.keys());
        println!("Enable features by populating 'features_enabled' in config");
    }
}

features! {
    pub mod feat {
        const List = 0b0000_0001,
        const BuildDeps = 0b0000_0010
    }
}
