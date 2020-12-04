// Copyright (c) 2016-2020 Chef Software Inc. and/or applicable contributors
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
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate features;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

use builder_core as bldr_core;
use habitat_builder_db as db;
use habitat_builder_graph as builder_graph;
use habitat_builder_protocol as protocol;
use habitat_core as hab_core;
use rusoto_core as rusoto;

pub mod config;
pub mod data_store;
pub mod error;
pub mod scheduler_datastore;
pub mod server;
#[cfg(test)]
#[cfg(feature = "postgres_tests")]
// cargo test --features postgres_tests to enable
// from root
// cargo test -p habitat_builder_jobsrv --features=postgres_tests
// --manifest-path=components/builder-jobsrv/Cargo.toml
mod test_helpers;

pub use crate::{config::Config,
                error::{Error,
                        Result}};

pub const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));
