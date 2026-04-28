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

#[macro_use]
extern crate log;

use std::{fmt,
          path::PathBuf,
          process};

use builder_core::config::ConfigFile;
use clap::{Parser,
           Subcommand};
use habitat_builder_api as bldr_api;

use crate::bldr_api::{config::Config,
                      server};

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

#[derive(Parser, Debug)]
#[command(version = VERSION, about = "Habitat builder-api", subcommand_required = true, arg_required_else_help = true)]
struct BuilderApi {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the builder-api server
    Start {
        /// Filepath to configuration file.
        #[arg(short, long)]
        config: Option<String>,

        /// Filepath to store packages, keys, and other artifacts.
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Listen port. [default: 9636]
        #[arg(long)]
        port: Option<u16>,
    },
}

#[actix_rt::main]
async fn main() {
    env_logger::init();
    let cli = BuilderApi::parse();
    debug!("CLI: {:?}", cli);
    match server::run(config_from_args(cli)).await {
        Ok(_) => std::process::exit(0),
        Err(e) => exit_with(e, 1),
    }
}

fn config_from_args(cli: BuilderApi) -> Config {
    match cli.command {
        Commands::Start { config, path, port } => {
            let mut cfg = match config {
                Some(cfg_path) => Config::from_file(cfg_path).unwrap(),
                None => Config::default(),
            };

            if let Some(p) = port {
                cfg.http.port = p;
            }

            if let Some(p) = path {
                cfg.api.data_path = p;
            }

            cfg
        }
    }
}

fn exit_with<T>(err: T, code: i32)
    where T: fmt::Display
{
    println!("{}", err);
    process::exit(code)
}
