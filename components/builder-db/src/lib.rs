// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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
extern crate diesel;
#[macro_use]
extern crate diesel_derive_enum;
extern crate diesel_full_text_search;
#[macro_use]
extern crate diesel_migrations;
extern crate builder_core as bldr_core;
extern crate fallible_iterator;
extern crate fnv;
extern crate habitat_builder_protocol as protocol;
extern crate habitat_core as hab_core;
#[macro_use]
extern crate log;
extern crate chrono;
extern crate num_cpus;
#[macro_use]
extern crate postgres;
#[macro_use]
extern crate postgres_derive;
extern crate postgres_shared;
extern crate protobuf;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate threadpool;
extern crate time;
extern crate url;

pub mod config;
pub mod diesel_pool;
pub mod error;
pub mod metrics;
pub mod migration;
pub mod models;
pub mod pool;
pub mod schema;

pub use diesel_pool::DbPool;
