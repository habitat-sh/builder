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
extern crate bodyparser;
extern crate builder_core as bldr_core;
extern crate builder_http_gateway as http_gateway;
extern crate crypto;
extern crate futures;
extern crate github_api_client;
extern crate habitat_builder_protocol as protocol;
extern crate habitat_core as hab_core;
extern crate habitat_depot_client as depot_client;
extern crate habitat_net as hab_net;
extern crate hyper;
extern crate iron;
extern crate iron_test;
extern crate libc;
#[macro_use]
extern crate log;
extern crate mount;
extern crate persistent;
extern crate protobuf;
extern crate r2d2;
extern crate regex;
extern crate router;
extern crate rusoto_core as rusoto;
extern crate rusoto_s3;
extern crate segment_api_client;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate tempfile;
extern crate time;
extern crate tokio_core;
extern crate toml;
extern crate unicase;
extern crate url;
extern crate uuid;
extern crate walkdir;
extern crate zmq;

pub mod backend;
pub mod config;
pub mod error;
pub mod handlers;
pub mod metrics;
pub mod server;
pub mod upstream;

pub use self::config::Config;
pub use self::error::{Error, Result};

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use hab_core::package::{Identifiable, PackageArchive, PackageTarget};
use iron::typemap;

pub trait DepotUtil {
    fn archive<T: Identifiable>(&self, ident: &T, target: &PackageTarget)
        -> Option<PackageArchive>;
    fn archive_name<T: Identifiable>(&self, ident: &T, target: &PackageTarget) -> PathBuf;
    fn packages_path(&self) -> PathBuf;
    fn write_archive(filename: &PathBuf, body: Vec<u8>) -> Result<PackageArchive>;
}

impl DepotUtil for config::Config {
    // Return a PackageArchive representing the given package. None is returned if Builder
    // doesn't have an archive for the given package.
    // TODO: This doesn't actually get called by the depot server process any longer
    // It might be good to validate whether or not this is even required anymore
    // and if it's not, remove the heck out of it!
    fn archive<T>(&self, ident: &T, target: &PackageTarget) -> Option<PackageArchive>
    where
        T: Identifiable,
    {
        let file = self.archive_name(ident, target);
        match fs::metadata(&file) {
            Ok(_) => Some(PackageArchive::new(file)),
            Err(_) => None,
        }
    }

    // Return a formatted string representing the filename of an archive for the given package
    // identifier pieces.
    fn archive_name<T: Identifiable>(&self, ident: &T, target: &PackageTarget) -> PathBuf {
        PathBuf::from(format!(
            "{}-{}-{}-{}-{}-{}.hart",
            ident.origin(),
            ident.name(),
            ident.version().unwrap(),
            ident.release().unwrap(),
            target.architecture,
            target.platform
        ))
    }

    fn write_archive(filename: &PathBuf, body: Vec<u8>) -> Result<PackageArchive> {
        let file = match File::create(&filename) {
            Ok(f) => f,
            Err(e) => {
                warn!(
                    "Unable to create archive file for {:?}, err={:?}",
                    filename, e
                );
                return Err(Error::IO(e));
            }
        };
        let mut write = BufWriter::new(file);
        if let Err(e) = write.write_all(&body) {
            warn!("Unable to write archive for {:?}, err={:?}", filename, e);
            return Err(Error::IO(e));
        }
        Ok(PackageArchive::new(filename))
    }

    fn packages_path(&self) -> PathBuf {
        Path::new(&self.path).join("pkgs")
    }
}

impl typemap::Key for config::Config {
    type Value = Self;
}
