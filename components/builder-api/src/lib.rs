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

extern crate base64;
#[macro_use]
extern crate bitflags;
extern crate bodyparser;
extern crate builder_core as bldr_core;
extern crate constant_time_eq;
#[macro_use]
extern crate features;
extern crate github_api_client;
extern crate habitat_builder_protocol as protocol;
extern crate habitat_core as hab_core;
extern crate habitat_http_client as http_client;
extern crate habitat_net as hab_net;
extern crate hex;
#[macro_use]
#[macro_use]
extern crate log;
extern crate oauth_client;
extern crate openssl;
extern crate params;
extern crate protobuf;
extern crate router;
extern crate segment_api_client;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate crypto;
extern crate futures;
extern crate libc;
extern crate num_cpus;
extern crate regex;
extern crate rusoto_core as rusoto;
extern crate rusoto_s3;
extern crate staticfile;
extern crate tempfile;
extern crate time;
extern crate toml;
extern crate typemap;
extern crate unicase;
extern crate url;
extern crate urlencoded;
extern crate uuid;
extern crate walkdir;
extern crate zmq;

pub mod backend;
pub mod config;
pub mod conn;
// pub mod depot;
pub mod error;
//pub mod github;
//pub mod handlers;
//pub mod headers;
//pub mod helpers;
pub mod metrics;
//pub mod middleware;
//pub mod net_err;
pub mod server;
mod types;
//pub mod upstream;

pub use self::config::Config;
pub use self::error::{Error, Result};

features! {
    pub mod feat {
        const List = 0b00000001,
        const Jobsrv = 0b00000010,
        const Upstream = 0b00000100
    }
}

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use hab_core::package::{PackageArchive, PackageIdent, PackageTarget};

pub trait DepotUtil {
    fn archive_name(ident: &PackageIdent, target: &PackageTarget) -> PathBuf;
    fn write_archive(filename: &PathBuf, body: &[u8]) -> Result<PackageArchive>;
    fn packages_path(&self) -> PathBuf;
}

impl DepotUtil for config::Config {
    // Return a formatted string representing the filename of an archive for the given package
    // identifier pieces.
    fn archive_name(ident: &PackageIdent, target: &PackageTarget) -> PathBuf {
        PathBuf::from(ident.archive_name_with_target(target).expect(&format!(
            "Package ident should be fully qualified, ident={}",
            &ident
        )))
    }

    fn write_archive(filename: &PathBuf, body: &[u8]) -> Result<PackageArchive> {
        let mut file = match File::create(&filename) {
            Ok(f) => f,
            Err(e) => {
                warn!(
                    "Unable to create archive file for {:?}, err={:?}",
                    filename, e
                );
                return Err(Error::IO(e));
            }
        };
        if let Err(e) = file.write_all(body) {
            warn!("Unable to write archive for {:?}, err={:?}", filename, e);
            return Err(Error::IO(e));
        }
        Ok(PackageArchive::new(filename))
    }

    fn packages_path(&self) -> PathBuf {
        Path::new(&self.api.data_path).join("pkgs")
    }
}
