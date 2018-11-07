// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
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

extern crate builder_core;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate env_proxy;
extern crate serde_json;
extern crate url;

pub mod a2;
pub mod active_directory;
pub mod azure_ad;
pub mod bitbucket;
pub mod client;
pub mod config;
pub mod error;
pub mod github;
pub mod gitlab;
pub mod metrics;
pub mod okta;
pub mod types;
