// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
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

use hab_core::package::{FromArchive, PackageArchive};
use protocol::originsrv::OriginPackage;
use std::path::{Path, PathBuf};

pub struct FileWalker {}

impl FileWalker {
    pub fn new<T: AsRef<Path>>(_path: T) -> Self {
        FileWalker {}
    }
}

pub fn extract_package<T: AsRef<Path>>(path: T) -> Option<OriginPackage> {
    let mut archive = PackageArchive::new(PathBuf::from(path.as_ref()));

    match archive.ident() {
        Ok(_) => match OriginPackage::from_archive(&mut archive) {
            Ok(p) => {
                return Some(p);
            }
            Err(e) => {
                error!("Error parsing package from archive: {:?}", e);
                return None;
            }
        },
        Err(e) => {
            error!("Error reading, archive={:?} error={:?}", &archive, &e);
            return None;
        }
    }
}
