// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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
extern crate clap;
#[macro_use]
extern crate log;

use std::{fmt,
          path::PathBuf,
          process,
          str::FromStr};

use habitat_builder_api as bldr_api;
use habitat_core as hab_core;

use crate::{bldr_api::{config::Config,
                       server},
            hab_core::config::ConfigFile};

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));
const CFG_DEFAULT_PATH: &str = "/hab/svc/builder-api/config/config.toml";

#[actix_rt::main]
fn main() {
    env_logger::init();
    let matches = app().get_matches();
    debug!("CLI matches: {:?}", matches);
    match server::run(config_from_args(&matches)).await {
        Ok(_) => std::process::exit(0),
        Err(e) => exit_with(e, 1),
    }
}

fn app<'a, 'b>() -> clap::App<'a, 'b> {
    clap_app!(BuilderApi =>
        (version: VERSION)
        (about: "Habitat builder-api")
        (@setting VersionlessSubcommands)
        (@setting SubcommandRequiredElseHelp)
        (@subcommand start =>
            (about: "Run the builder-api server")
            (@arg config: -c --config +takes_value
                "Filepath to configuration file. [default: /hab/svc/builder-api/config/config.toml]")
            (@arg path: -p --path +takes_value
                "Filepath to store packages, keys, and other artifacts.")
            (@arg port: --port +takes_value "Listen port. [default: 9636]")
        )
    )
}

fn config_from_args(matches: &clap::ArgMatches) -> Config {
    let cmd = matches.subcommand_name().unwrap();
    let args = matches.subcommand_matches(cmd).unwrap();
    let mut config = match args.value_of("config") {
        Some(cfg_path) => Config::from_file(cfg_path).unwrap(),
        None => Config::from_file(CFG_DEFAULT_PATH).unwrap_or_default(),
    };

    if let Some(port) = args.value_of("port") {
        u16::from_str(port).map(|p| config.http.port = p)
                           .expect("Specified port must be a valid u16");
    }

    if let Some(path) = args.value_of("path") {
        config.api.data_path = PathBuf::from(path);
    }

    config
}

fn exit_with<T>(err: T, code: i32)
    where T: fmt::Display
{
    println!("{}", err);
    process::exit(code)
}
