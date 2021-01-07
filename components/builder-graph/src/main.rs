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

#[macro_use]
extern crate bitflags;
use clap::clap_app;
#[macro_use]
extern crate features;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;

extern crate diesel;
extern crate internment;
extern crate serde;
extern crate serde_json;

use habitat_builder_db as db;
use habitat_builder_protocol as protocol;

use habitat_core as hab_core;

#[macro_use]
pub mod package_ident_intern;
pub mod config;
pub mod data_store;
pub mod error;
pub mod graph_helpers;
pub mod package_build_manifest_graph;
pub mod package_graph;
pub mod package_graph_target;
pub mod package_graph_trait;
pub mod package_info;
pub mod rdeps;
pub mod util;

use std::{collections::HashMap,
          fs::File,
          io::Write,
          iter::FromIterator,
          path::Path,
          str::FromStr,
          time::Instant};

use builder_core::config::ConfigFile;
use clap::{App,
           AppSettings,
           Arg,
           ArgMatches};
use copperline::Copperline;

use crate::{config::Config,
            data_store::{DataStore,
                         DataStoreTrait,
                         SerializedDatabase},
            hab_core::package::{PackageIdent,
                                PackageTarget},
            package_graph::PackageGraph,
            package_ident_intern::PackageIdentIntern};

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

struct State {
    datastore: Option<Box<dyn DataStoreTrait>>,
    graph:     PackageGraph,
    done:      bool,
    cli:       clap::App<'static, 'static>,
}

// TODO See if we can remove this before final merge, much refactoring has happened.
#[allow(clippy::cognitive_complexity)]
fn main() {
    env_logger::init();

    let matches =
        App::new("bldr-graph").version(VERSION)
                              .about("Habitat Graph Dev Tool")
                              .arg(Arg::with_name("config").help("Filepath to configuration file")
                                                           .required(false)
                                                           .index(1))
                              .arg(Arg::with_name("internal_command").multiple(true)
                                                                     .last(true)
                                                                     .help("Internal CLI command \
                                                                            to run"))
                              .get_matches();

    let config = match matches.value_of("config") {
        Some(cfg_path) => Config::from_file(cfg_path).unwrap(),
        None => Config::default(),
    };

    // Split on commas
    let external_args = match matches.values_of("internal_command") {
        Some(values) => split_command(values.collect()),
        None => Vec::<Vec<String>>::new(),
    };

    enable_features(&config);

    let mut cl = Copperline::new();
    let graph = PackageGraph::new();

    let mut state = State { datastore: None,
                            graph,
                            done: false,
                            cli: make_clap_cli() };

    // This is meant to ease testing of this command and provide a quick one-off access to the CLI
    //
    if !external_args.is_empty() {
        for command in external_args {
            let command: Vec<&str> = command.iter().map(|s| &**s).collect();
            println!("Cmd> {:?}", command);
            state.process_command(&command);
        }
        state.done = true
    } else {
        state.cli.print_help().unwrap();

        while !state.done {
            let prompt = format!("{}: command> ", state.graph.current_target());
            let line = cl.read_line_utf8(&prompt).ok();
            if line.is_none() {
                continue;
            }
            let cmd = line.expect("Could not get line");
            cl.add_history(cmd.clone());

            let v: Vec<&str> = cmd.trim_end().split_whitespace().collect();
            if !v.is_empty() {
                state.process_command(&v);
            }
        }
    }
}

impl State {
    fn process_command(&mut self, v: &[&str]) {
        let match_result = self.cli.get_matches_from_safe_borrow(v);

        match match_result {
            Ok(matches) => {
                match matches.subcommand() {
                    ("help", _) => do_help(&matches, &mut self.cli), // This
                    // doesn't work something is eating the help output
                    //                   ("build_levels", Some(m)) => do_build_levels(&self.graph,
                    // &m),
                    ("build_order", Some(m)) => {
                        if let Some(datastore) = &self.datastore {
                            do_dump_build_order(datastore.as_ref(), &mut self.graph, &m);
                        } else {
                            println!("'build_order' requires a database connection; See \
                                      'db_connect'");
                        }
                    }
                    ("check", Some(m)) => {
                        if let Some(datastore) = &self.datastore {
                            do_check(datastore.as_ref(), &self.graph, &m)
                        } else {
                            println!("'check' requires a database connection; See 'db_connect'")
                        }
                    }
                    ("compute_attributed_deps", Some(m)) => {
                        do_compute_attributed_deps(&self.graph, &m)
                    }
                    ("db_connect", Some(m)) => self.do_db_connect(&m),
                    // TODO RENAME THIS COMMAND
                    ("serialized_db_connect", Some(m)) => self.do_serialized_db_connect(&m),
                    ("db_deps", Some(m)) => {
                        if let Some(datastore) = &self.datastore {
                            do_db_deps(datastore.as_ref(), &self.graph, &m);
                        } else {
                            println!("'db_deps' requires a database connection; See 'db_connect'");
                        }
                    }
                    ("deps", Some(m)) => do_deps(&self.graph, &m),
                    // Probably not publically useful
                    ("diagnostics", Some(m)) => do_dump_diagnostics(&self.graph, &m),
                    ("dot", Some(m)) => do_dot(&self.graph, &m),
                    // TODO TEST
                    ("find", Some(m)) => do_find(&self.graph, &m),
                    ("quit", _) => self.done = true,
                    ("raw", Some(m)) => do_raw(&self.graph, &m),
                    // TODO: add filter as option for this command, make sure we do filtering
                    // uniformly; this is broken
                    ("rdeps", Some(m)) => do_rdeps(&self.graph, &m),
                    ("resolve", Some(m)) => do_resolve(&self.graph, &m),
                    ("save_file", Some(m)) => {
                        if let Some(datastore) = &self.datastore {
                            do_save_file(datastore.as_ref(), &self.graph, &m)
                        } else {
                            println!("'db_deps' requires a database connection; See 'db_connect'");
                        }
                    }
                    // TODO TEST
                    ("scc", Some(m)) => do_scc(&self.graph, &m),
                    ("stats", Some(m)) => do_stats(&self.graph, &m),
                    // TODO TEST
                    ("target", Some(m)) => do_target(&mut self.graph, &m),
                    ("top", Some(m)) => do_top(&self.graph, &m),
                    name => println!("Unknown  {:?} {:?}", matches, name),
                }
            }
            // Ideally we'd match the various errors and do something more
            // clever, e.g. Err(HelpDisplayed) => self.cli.print_help(UNKNOWN_ARGUMENTS)
            // But I've not totally figured that out yet.
            Err(x) => {
                println!("ClapError {:?} {:?}", x.kind, x);
                println!("ClapError Msg: {}", x.message);
                println!("ClapError Info: {:?}", x.info);
            }
        }
    }

    fn build_graph(&mut self) {
        println!("Building graph... please wait.");

        let start_time = Instant::now();
        let packages = self.datastore
                           .as_ref()
                           .unwrap()
                           .get_job_graph_packages()
                           .unwrap();

        let fetch_time = start_time.elapsed().as_secs_f64();
        println!("OK: fetched {} packages ({} sec)",
                 packages.len(),
                 fetch_time);

        let start_time = Instant::now();
        let (ncount, ecount) = self.graph
                                   .build(packages.into_iter(), feat::is_enabled(feat::BuildDeps));
        println!("OK: {} nodes, {} edges ({} sec)",
                 ncount,
                 ecount,
                 start_time.elapsed().as_secs_f64());

        let targets = self.graph.targets();
        let target_as_string: Vec<String> = targets.iter().map(|t| t.to_string()).collect();

        println!("Found following targets {}", target_as_string.join(", "));
        println!("Default target is {}", self.graph.current_target());
    }

    fn do_db_connect(&mut self, matches: &ArgMatches) {
        let config = match matches.value_of("CONFIG_FILE") {
            Some(cfg_path) => Config::from_file(cfg_path).unwrap(),
            None => Config::default(),
        };
        println!("Connecting to {}", config.datastore.database);

        let datastore = DataStore::new(&config);
        datastore.setup().unwrap();
        self.datastore = Some(Box::new(datastore));

        self.build_graph();
    }

    fn do_serialized_db_connect(&mut self, matches: &ArgMatches) {
        if let Some(data_file) = matches.value_of("CONFIG_FILE") {
            println!("Reading Serialized DB from file {}", data_file);
            let start_time = Instant::now();
            let datastore = SerializedDatabase::read_from_file(data_file).unwrap();
            self.datastore = Some(Box::new(datastore));
            let file_duration = start_time.elapsed().as_secs_f64();
            println!("Reading Serialized DB from file {} in {} secs",
                     data_file, file_duration);
            self.build_graph();
        } else {
            println!("No Dummy DB file provided")
        }
    }
}

fn do_help(_matches: &ArgMatches, cli: &mut clap::App<'static, 'static>) {
    cli.print_long_help().unwrap(); // print_help might be more usable
}

fn do_stats(graph: &PackageGraph, matches: &ArgMatches) {
    if matches.is_present("ALL") {
        do_all_stats_sub(graph);
    } else {
        do_stats_sub(graph);
    }
}

fn do_all_stats_sub(graph: &PackageGraph) {
    let stats = graph.all_stats();
    for (target, stat) in stats {
        println!("Target: {}", target);
        println!("  Node count: {}", stat.node_count);
        println!("  Edge count: {}", stat.edge_count);
        println!("  Connected components: {}", stat.connected_comp);
        println!("  Is cyclic: {}", stat.is_cyclic)
    }
}

fn do_stats_sub(graph: &PackageGraph) {
    if let Some(stats) = graph.stats() {
        println!("Node count: {}", stats.node_count);
        println!("Edge count: {}", stats.edge_count);
        println!("Connected components: {}", stats.connected_comp);
        println!("Is cyclic: {}", stats.is_cyclic);
    } else {
        println!("No graph loaded!");
    }
}

fn do_top(graph: &PackageGraph, matches: &ArgMatches) {
    let count = count_from_matches(matches).unwrap();
    let start_time = Instant::now();
    let top = graph.top(count);

    println!("OK: {} items ({} sec)\n",
             top.len(),
             start_time.elapsed().as_secs_f64());

    for (name, count) in top {
        println!("{}: {}", name, count);
    }
    println!();
}

fn do_find(graph: &PackageGraph, matches: &ArgMatches) {
    let phrase = search_from_matches(matches);
    let max = count_from_matches(matches).unwrap(); // WIP Rework command loop to handle result
    let start_time = Instant::now();
    let mut v = graph.search(&phrase);

    println!("OK: {} items ({} sec)\n",
             v.len(),
             start_time.elapsed().as_secs_f64());

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

fn do_save_file(datastore: &dyn DataStoreTrait, graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let filter = matches.value_of("FILTER");
    let filename = required_filename_from_matches(matches);

    datastore.serialize(filename,
                        filter.unwrap_or("core"),
                        "stable",
                        graph.current_target())
             .unwrap();

    let duration_secs = start_time.elapsed().as_secs_f64();
    println!("Wrote packages to file {} filtered by {:?} (TBI) in {} sec",
             filename, filter, duration_secs);
}

fn do_dump_diagnostics(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let filter = matches.value_of("FILTER");
    let filename = required_filename_from_matches(matches);

    graph.dump_diagnostics(filename, filter);

    let duration_secs = start_time.elapsed().as_secs_f64();
    println!("Wrote packages to file {} filtered by {:?} (TBI) in {} sec",
             filename, filter, duration_secs);
}
fn do_dump_build_order(datastore: &dyn DataStoreTrait,
                       graph: &mut PackageGraph,
                       matches: &ArgMatches) {
    let start_time = Instant::now();
    let filter = str_from_matches(matches, "FILTER", "core");
    let filename = required_filename_from_matches(matches);
    let touched = ident_from_matches(matches).unwrap();

    println!("Computing build order for origin {} to file {}",
             filter, filename);

    let touched = vec![touched.into()]; // TODO use a real set, huh?
                                        // let touched = vec![touched];
                                        //

    let manifest = graph.compute_build(&touched, datastore.as_unbuildable());
    println!("-------------------");

    let mut file = File::create(&filename).expect("Failed to initialize file");
    for pkg in &manifest.build_order() {
        file.write_all(pkg.format_for_shell().as_bytes()).unwrap();
    }
    println!("-------------------");

    let duration_secs = start_time.elapsed().as_secs_f64();

    println!("Generated build order for '{}' and wrote to file file {} filtered by {:?} in {} sec",
             touched.first().unwrap(),
             filename,
             filter,
             duration_secs);
}

fn do_dot(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let origin = origin_from_matches(matches);
    let filename = required_filename_from_matches(matches);

    graph.dump_latest_graph_as_dot(&filename, origin.as_deref());
    let duration_secs = start_time.elapsed().as_secs_f64();

    println!("Wrote latest graph to file {} filtered by {:?} TBI in {} sec",
             filename, origin, duration_secs);
}

fn do_raw(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let origin = origin_from_matches(matches);
    let filename = required_filename_from_matches(matches);
    let graph_type;
    if matches.is_present("LATEST") {
        graph_type = "latest";
        graph.dump_latest_graph_raw(&filename, origin.as_deref());
    } else {
        graph_type = "current";
        if let Some(o) = origin {
            println!("Origin {} ignored", o)
        }
        println!("Raw graph dump TBI for full graph");
        // graph.emit_graph_raw(&filename, None, true, None);
    }
    let duration_secs = start_time.elapsed().as_secs_f64();

    println!("Wrote {} raw graph to file {} filtered by {:?} TBI in {} sec",
             graph_type, filename, origin, duration_secs);
}

fn do_scc(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let origin = origin_from_matches(matches);
    let filename = required_filename_from_matches(matches);

    graph.dump_scc(filename, origin);
    let duration_secs = start_time.elapsed().as_secs_f64();
    println!("Wrote SCC of latest information to file {} filtered by {:?} TBI in {} sec",
             filename, origin, duration_secs);
}

#[allow(dead_code)]
fn do_build_levels(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let origin = origin_from_matches(matches);
    let filename = required_filename_from_matches(matches);
    graph.dump_build_levels(filename, origin);
    let duration_secs = start_time.elapsed().as_secs_f64();
    println!("Wrote Build levels information to file {} filtered by {:?} TBI in {} sec",
             filename, origin, duration_secs);
}

fn do_resolve(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let ident = ident_from_matches(matches).unwrap();
    let result = graph.resolve(&ident);
    println!("OK: ({} sec)\n", start_time.elapsed().as_secs_f64());

    match result {
        Some(s) => println!("{}", s),
        None => println!("No matching packages found"),
    }

    println!();
}

fn do_rdeps(graph: &PackageGraph, matches: &ArgMatches) {
    // These are safe because we have validators on the args
    let filename = required_filename_from_matches(matches);
    let ident: PackageIdentIntern = ident_from_matches(matches).unwrap().into();
    let origin = origin_from_matches(matches);

    let rdeps = graph.rdeps(&ident, origin);
    let mut file = File::create(&filename).unwrap();

    writeln!(&mut file, "{}", ident).unwrap();
    for (dep, _) in &rdeps {
        if *dep != ident {
            writeln!(&mut file, "  {}", dep).unwrap();
        }
    }
}

fn resolve_name(graph: &PackageGraph, ident: &PackageIdent) -> PackageIdent {
    let parts: Vec<&str> = ident.iter().collect();
    if parts.len() == 2 {
        match graph.resolve(ident) {
            Some(s) => s,
            None => ident.clone(),
        }
    } else {
        ident.clone()
    }
}

/// Recursively expand package's deps from database, verifying that they all exist
/// This might need some rethinking in the new graph..
/// We are leaving this for now, as there is a interesting kernel of an idea here.
/// NOTE THIS IS ASKING THE WHAT IF QUESTION around if deps were updated, but does it in an
/// incorrect way See below
/// There are two commands we probably want.
/// 1) take multiple existing packages and determine if their deps conflict
/// 2) take an existing package and see if it is 'buildable' given the plan deps, possibly as a set
/// with other    packages to see if they resolve to a compatible set of packages
fn do_check(datastore: &dyn DataStoreTrait, graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let mut deps_map = HashMap::new();
    let idents = idents_from_matches(matches).unwrap();
    let filter = filter_from_matches(matches);
    let resolved_idents = idents.iter().map(|ident| resolve_name(graph, &ident));
    let target = graph.current_target();

    let mut conflicts = 0;
    for ident in resolved_idents {
        let mut new_deps = Vec::new();
        match datastore.get_job_graph_package(&ident, target) {
            Ok(package) => {
                if !filter.is_empty() {
                    println!("Checks filtered by: {}\n", filter);
                }

                println!("Dependency version updates for {} {}:",
                         ident, package.ident.0);
                for dep in package.deps {
                    if dep.to_string().starts_with(&filter) {
                        // BUG need to actually respect pinned plan deps rather than just use the
                        // short name For example this would say rethinkdb
                        // is ok even though it pins deps and is incompatible
                        // with core/gcc
                        let dep_name = util::short_ident(&(dep.0), false);
                        let dep_latest = resolve_name(graph, &dep_name);
                        deps_map.insert(dep_name.clone(), dep_latest.clone());
                        new_deps.push(dep_latest.clone());
                        println!("{} -> {}", dep.0, dep_latest);
                    }
                }

                println!();

                for new_dep in new_deps {
                    conflicts +=
                        check_package(Some(datastore), target, 0, &mut deps_map, &new_dep, &filter);
                }
            }
            Err(_) => println!("No matching package found"),
        }
    }

    println!("\n{} conflicts found in time: {} sec\n",
             conflicts,
             start_time.elapsed().as_secs_f64());
}

fn do_deps(graph: &PackageGraph, matches: &ArgMatches) {
    let ident = ident_from_matches(matches).unwrap(); // safe because we validate this arg
    println!("Dependencies for: {}", ident);
    graph.write_deps(&ident);
}

fn do_db_deps(datastore: &dyn DataStoreTrait, graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let ident = ident_from_matches(matches).unwrap(); // safe because we validate this arg
    let filter = filter_from_matches(matches);
    let ident = resolve_name(graph, &ident);
    let target = graph.current_target();

    println!("Dependencies for: {}", ident);

    match datastore.get_job_graph_package(&ident, target) {
        // Thinking about whether we want to check the build deps as well.
        Ok(package) => {
            println!("OK: {} items ({} sec)\n",
                     package.deps.len(),
                     start_time.elapsed().as_secs_f64());
            if !filter.is_empty() {
                println!("Results filtered by: {}\n", filter);
            }

            for dep in package.deps {
                if dep.to_string().starts_with(&filter) {
                    println!("{}", dep.0)
                }
            }
        }
        Err(_) => println!("No matching package found"),
    }

    println!();
}

fn do_compute_attributed_deps(graph: &PackageGraph, matches: &ArgMatches) {
    let in_file_opt = matches.value_of("IN_FILENAME");
    let out_file = required_filename_from_matches(matches);
    let include_build_deps = matches.is_present("BUILD_DEPS");

    let mut idents: Vec<PackageIdentIntern> = if let Some(in_file) = in_file_opt {
        util::file_into_idents(in_file).unwrap()
                                       .iter()
                                       .map(|x| x.into())
                                       .collect()
    } else {
        Vec::new()
    };
    let mut packages: Vec<PackageIdentIntern> =
        interned_idents_from_matches(matches).unwrap_or_else(|_| Vec::new())
                                             .to_vec();
    idents.append(&mut packages);

    let a_deps = graph.compute_attributed_deps(&idents, include_build_deps);

    let path = Path::new(out_file);
    let mut file = File::create(&path).unwrap();

    //
    println!("Expanded {} to {} deps, with (including build deps = {}), writing to {}",
             if let Some(in_file) = in_file_opt {
                 format!("input file {}", in_file)
             } else {
                 "args".to_string()
             },
             a_deps.len(),
             include_build_deps,
             out_file);

    let mut keys: Vec<PackageIdentIntern> = a_deps.keys().copied().collect();
    keys.sort_by(package_ident_intern::display_ordering_cmp);

    for package in keys.iter() {
        let deps_list: Vec<PackageIdentIntern> = a_deps[package].iter().copied().collect();
        writeln!(&mut file,
                 "{}\t{}",
                 package,
                 util::join_idents(", ", &deps_list)).unwrap();
    }
}

#[allow(clippy::map_entry)]
fn check_package(datastore: Option<&dyn DataStoreTrait>,
                 target: PackageTarget,
                 depth: usize,
                 deps_map: &mut HashMap<PackageIdent, PackageIdent>,
                 ident: &PackageIdent,
                 filter: &str)
                 -> u32 {
    let mut conflicts = 0;
    if let Some(datastore) = datastore {
        println!("{}{}", " ".repeat(depth * 2), ident);
        match datastore.get_job_graph_package(ident, target) {
            Ok(package) => {
                for dep in package.deps {
                    if dep.to_string().starts_with(filter) {
                        let name = util::short_ident(&dep, false);
                        {
                            if deps_map.contains_key(&name) {
                                let value = deps_map.get(&name).unwrap();
                                if *value != dep.0 {
                                    conflicts += 1;
                                    println!("Conflict: {}", ident);
                                    println!("  {}", value);
                                    println!("  {}", dep.0);
                                } else {
                                    println!("{}{} seen", " ".repeat((depth + 1) * 2), dep.0);
                                }
                            } else {
                                deps_map.insert(name, dep.0.clone());
                                conflicts += check_package(Some(datastore),
                                                           target,
                                                           depth + 1,
                                                           deps_map,
                                                           &dep.0,
                                                           filter);
                            }
                        }
                    }
                }
            }
            Err(_) => println!("No matching package found for {}", ident),
        }
    } else {
        println!("Not connected to a database. See 'db_connect --help'");
    };
    conflicts
}

fn do_target(graph: &mut PackageGraph, matches: &ArgMatches) {
    match target_from_matches(matches) {
        Ok(package_target) => graph.set_target(package_target),
        Err(msg) => println!("{}", msg),
    }
}

fn enable_features(config: &Config) {
    let features: HashMap<_, _> = HashMap::from_iter(vec![("BUILDDEPS", feat::BuildDeps)]);
    let features_enabled = config.features_enabled
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

// Arg parsing using clap
//

fn make_clap_cli() -> App<'static, 'static> {
    App::new("Interactive graph explorer")
        .about("Interactive graph explorer")
        .version(VERSION)
        .author("\nThe Habitat Maintainers <humans@habitat.sh>\n")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DisableHelpSubcommand)
        //        .setting(AppSettings::HelpRequired) // panic if no help string spec'd
        .setting(AppSettings::NoBinaryName)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(build_levels_subcommand())
        .subcommand(build_order_subcommand())
        .subcommand(check_subcommand())
        .subcommand(compute_attributed_deps_subcommand())
        .subcommand(db_connect_subcommand())
        .subcommand(serialized_db_connect_subcommand())
        .subcommand(db_deps_subcommand())
        .subcommand(diagnostics_subcommand())
        .subcommand(deps_subcommand())
        .subcommand(dot_subcommand())
        .subcommand(find_subcommand())
        .subcommand(load_from_db_subcommand())
        .subcommand(load_from_file_subcommand())
        .subcommand(save_to_file_subcommand())
        .subcommand(help_subcommand())
        .subcommand(quit_subcommand())
        .subcommand(raw_subcommand())
        .subcommand(rdeps_subcommand())
        .subcommand(resolve_subcommand())
        .subcommand(scc_subcommand())
        .subcommand(stats_subcommand())
        .subcommand(target_subcommand())
        .subcommand(top_subcommand())
}

// All of these basically filter the graph in some fashion and dump to a file; may be worth
// combining in some fashion
fn build_levels_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand build_levels =>
              (about: "Dump build levels of packages")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg ORIGIN: "Restrict to this origin")
    )
}

fn check_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand check =>
              (about: "Check package")
              (@arg IDENT: ... +required +takes_value {valid_ident} "Package ident to resolve")
    )
}

fn compute_attributed_deps_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand compute_attributed_deps =>
              (about: "Compute transitive deps from input, with attribution of the user(s)")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg IN_FILENAME: --infile +takes_value "Filename to read deps from")
              (@arg IDENT: --idents ... +takes_value {valid_ident} "Package ident to resolve" )
              (@arg FILTER: --filter +takes_value "Filter to this origin")
              (@arg BUILD_DEPS: --build "Expand to build deps as well")
    )
}

fn db_connect_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand db_connect =>
            (about: "Connect to bldr datastore")
            (@arg CONFIG_FILE: +takes_value "Configuration file to load. Takes precedence over remaining options")
            (@arg HOST: +takes_value default_value("127.0.0.1:5432") "Host to connect to")
            (@arg DATABASE: +takes_value default_value("bldr") "Database name to use")
            (@arg USER: +takes_value default_value("hab") "Username to connect as")
            (@arg PASSWORD: +takes_value "Password for USER"))
}

fn serialized_db_connect_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand serialized_db_connect =>
            (about: "Connect to serialized copy of bldr datastore")
            (@arg CONFIG_FILE: +takes_value "File to load database from"))
}

fn db_deps_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand db_deps =>
              (about: "Dump package deps from db")
              (@arg IDENT: +required +takes_value {valid_ident} "Package ident to resolve")
              (@arg FILTER: +takes_value default_value("") "Filter value")
    )
}

fn deps_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand deps =>
              (about: "Dump package deps from graph")
              (@arg IDENT: +required +takes_value {valid_ident} "Package ident to resolve")
    )
}

fn dot_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand dot =>
              (about: "Dump DOT format graph of packages")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg ORIGIN: "Restrict to this origin")
    )
}

fn help_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand help =>
              (about: "Help on help")
    )
}

fn scc_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand scc =>
              (about: "Dump SCC information for latest graph packages")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg ORIGIN: "Restrict to this origin")
    )
}

fn raw_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand raw =>
              (about: "Dump raw (simple edge representation) graph of packages")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg ORIGIN: "Restrict to this origin")
              (@arg LATEST: --latest -l "Write latest graph")
    )
}

fn find_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand find =>
              (about: "Find packages")
              (@arg SEARCH: +takes_value "Search term to use")
              (@arg COUNT: {valid_numeric::<usize>} default_value("10") "Number of packages to show")
    )
}

fn load_from_file_subcommand() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand load_file =>
                        (about: "Load packages from file into graph")
                        (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
                        (@arg FILTER: +takes_value "Filter value")
    );
    sub
}

fn save_to_file_subcommand() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand save_file =>
                        (about: "Write packages into graph for current target to file")
                        (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
                        (@arg FILTER: +takes_value "Filter value")
    );
    sub
}

fn load_from_db_subcommand() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand load_db =>
                        (about: "Read packages from DB into graph")
    );
    sub
}

fn build_order_subcommand() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand build_order =>
                        (about: "Write build order to file")
                        (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
                        (@arg FILTER: +required +takes_value "Filter value")
                        (@arg IDENT: +required +takes_value "Packages that changed")
    );
    sub
}

fn diagnostics_subcommand() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand diagnostics =>
                        (about: "Write diagnostics about current graph to file")
                        (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
                        (@arg FILTER: +takes_value "Filter value")
    );
    sub
}

fn rdeps_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand rdeps =>
              (about: "Find rdeps of a package")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg IDENT: +required +takes_value {valid_ident} "Package ident to resolve")
              (@arg ORIGIN: "Restrict to this origin")
    )
}

fn resolve_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand resolve =>
              (about: "Resolve packages")
              (@arg IDENT: +required +takes_value {valid_ident} "Package ident to resolve")
    )
}

fn quit_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand quit =>
              (about: "quit this shell")
    ).aliases(&["q", "exit"])
}

fn stats_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand stats =>
              (about: "Show graph stats for targets")
              (@arg ALL: --all "Stats for all targets")
              (@arg TARGET: +takes_value {valid_target} "Target architecture (e.g. x86_64-linux)")
    )
}

fn target_subcommand() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand target =>
                        (about: "Set target architecture to use")
                    (@arg TARGET: +required +takes_value {valid_target} "Target architecture (e.g. x86_64-linux)")
    );
    sub
}

fn top_subcommand() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand top =>
                        (about: "Show top packages, by usage")
                        (@arg COUNT: {valid_numeric::<usize>} default_value("10") "Number of packages to show")
    );
    sub
}

// This was lifted from the habitat CLI
//
#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_ident(val: String) -> Result<(), String> {
    match PackageIdent::from_str(&val) {
        Ok(_) => Ok(()),
        Err(_) => {
            Err(format!("'{}' is not valid. Package identifiers have the \
                         form origin/name[/version[/release]]",
                        &val))
        }
    }
}

// This was lifted from the habitat CLI
//
#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_target(val: String) -> Result<(), String> {
    match PackageTarget::from_str(&val) {
        Ok(_) => Ok(()),
        Err(_) => {
            let targets: Vec<_> = PackageTarget::targets().map(std::convert::AsRef::as_ref)
                                                          .collect();
            Err(format!("'{}' is not valid. Valid targets are in the form \
                         architecture-platform (currently Habitat allows \
                         the following: {})",
                        &val,
                        targets.join(", ")))
        }
    }
}

// This was lifted from the habitat CLI
//
#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_numeric<T: FromStr>(val: String) -> Result<(), String> {
    match val.parse::<T>() {
        Ok(_) => Ok(()),
        Err(_) => Err(format!("'{}' is not a valid number", &val)),
    }
}

fn count_from_matches(matches: &ArgMatches) -> Result<usize, String> {
    let count = matches.value_of("COUNT").unwrap();
    count.parse()
         .map_err(|_| format!("{} not valid integer for count", count))
}

fn required_filename_from_matches<'a>(matches: &'a ArgMatches) -> &'a str {
    // Required option, so always present
    matches.value_of("REQUIRED_FILENAME").unwrap()
}

fn filter_from_matches(matches: &ArgMatches) -> String {
    matches.value_of("FILTER")
           .map_or_else(|| String::from(""), |x| x.to_string())
}

fn origin_from_matches<'a>(matches: &'a ArgMatches) -> Option<&'a str> {
    matches.value_of("ORIGIN")
}

fn target_from_matches(matches: &ArgMatches) -> Result<PackageTarget, String> {
    let target = matches.value_of("TARGET").unwrap(); // is target mandatory?
    PackageTarget::from_str(target).map_err(|_| format!("{} is not a valid target", target))
}

fn search_from_matches(matches: &ArgMatches) -> String {
    str_from_matches(matches, "SEARCH", "").to_string()
}

fn str_from_matches<'a>(matches: &'a ArgMatches, name: &str, default: &'a str) -> &'a str {
    matches.value_of(name).unwrap_or(default)
}

fn ident_from_matches(matches: &ArgMatches) -> Result<PackageIdent, String> {
    let ident_str: &str = matches.value_of("IDENT")
                                 .ok_or_else(|| String::from("Ident required"))?;
    PackageIdent::from_str(ident_str).map_err(|e| format!("Expected ident gave error {:?}", e))
}

fn idents_from_matches(matches: &ArgMatches) -> Result<Vec<PackageIdent>, String> {
    let ident_strings = matches.values_of("IDENT")
                               .ok_or_else(|| String::from("Ident required"))?;
    let idents =
        ident_strings.map(|s| {
                         PackageIdent::from_str(s).map_err(|e| {
                                                      format!("Expected ident gave error {:?}", e)
                                                  })
                     });
    idents.collect()
}

fn interned_idents_from_matches(matches: &ArgMatches) -> Result<Vec<PackageIdentIntern>, String> {
    let ident_strings = matches.values_of("IDENT")
                               .ok_or_else(|| String::from("Ident required"))?;
    let idents = ident_strings.map(|s| {
                                  PackageIdentIntern::from_str(s).map_err(|e| {
                                                                     format!("Expected ident gave \
                                                                              error {:?}",
                                                                             e)
                                                                 })
                              });
    idents.collect()
}

fn split_command(values: Vec<&str>) -> Vec<Vec<String>> {
    let mut result = Vec::<Vec<String>>::new();

    let mut command = Vec::<String>::new();
    for word in values {
        if word.contains(',') {
            let split: Vec<String> = word.to_string().split(',').map(|s| s.to_string()).collect();
            if !split[0].is_empty() {
                command.push(split[0].to_string().clone());
            }
            let post = split[1].to_string().clone();
            result.push(command);
            command = Vec::<String>::new();
            if !post.is_empty() {
                command.push(post);
            }
        } else {
            command.push(word.to_string().clone())
        }
    }
    result.push(command);
    result
}
