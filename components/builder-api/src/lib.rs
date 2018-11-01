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

extern crate actix;
extern crate actix_web;
extern crate base64;
#[macro_use]
extern crate bitflags;
extern crate builder_core as bldr_core;
extern crate bytes;
extern crate chrono;
extern crate constant_time_eq;
extern crate diesel;
extern crate diesel_full_text_search;
#[macro_use]
extern crate features;
extern crate github_api_client;
extern crate habitat_builder_db as db;
extern crate habitat_builder_protocol as protocol;
extern crate habitat_core as hab_core;
extern crate habitat_http_client as http_client;
extern crate habitat_net as hab_net;
extern crate hex;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate oauth_client;
extern crate openssl;
extern crate protobuf;
extern crate segment_api_client;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate futures;
extern crate memcache;
extern crate num_cpus;
extern crate r2d2;
extern crate regex;
extern crate rusoto_core as rusoto;
extern crate rusoto_s3;
extern crate sha2;
extern crate tempfile;
extern crate time;
extern crate toml;
extern crate url;
extern crate uuid;
extern crate zmq;

pub mod config;
pub mod server;
