// Copyright (c) 2016-2022 Chef Software Inc. and/or applicable contributors
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
extern crate diesel;
#[macro_use]
extern crate diesel_derive_enum;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate log;
extern crate postgres;
#[macro_use]
extern crate postgres_types;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate paste;

use builder_core as bldr_core;
use habitat_builder_protocol as protocol;
use habitat_core as hab_core;

pub mod config;
pub mod diesel_pool;
pub mod error;
pub mod functions;
pub mod metrics;
pub mod migration;
// https://github.com/rust-lang/rust-clippy/issues/9014
// until clippy's fix for the above false positive is live
#[allow(clippy::extra_unused_lifetimes)]
pub mod models;
pub mod schema;
pub mod test;

pub use crate::diesel_pool::DbPool;
