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

use config::Config;
use hab_core::package::PackageIdent;
use memcache;
use std::sync::Arc;

pub struct MemCache {
    cli: memcache::Client,
}

impl MemCache {
    pub fn new(config: Arc<Config>) -> Self {
        let memcache_host_strings: Vec<String> =
            config.memcached.iter().map(|h| format!("{}", h)).collect();
        let memcache_hosts: Vec<&str> = memcache_host_strings.iter().map(AsRef::as_ref).collect();
        MemCache {
            cli: memcache::Client::new(memcache_hosts).unwrap(),
        }
    }

    pub fn set_package(&mut self, package: PackageIdent, channel: &str) {
        if package.version.is_none() || package.release.is_none() {
            debug!("Can't insert non-fully qualified package ident into the cache");
            return;
        }
        let keys = vec![
            format!("{}/{}/{}", channel, package.origin, package.name),
            format!(
                "{}/{}/{}/{}",
                channel,
                package.origin,
                package.name,
                package.clone().version.unwrap()
            ),
        ];
        for key in keys {
            match self.cli.set(&key, package.to_string(), 15 * 60) {
                Ok(_) => debug!("Saved {:?} to memcached", package.to_string()),
                Err(e) => warn!(
                    "Failed to save {:?} to memcached: {}",
                    package.to_string(),
                    e
                ),
            };
        }
    }

    pub fn get_package(&mut self, package: PackageIdent, channel: &str) -> Option<String> {
        match package.version {
            Some(v) => {
                // Sometimes PackageIdent returns Some("") so we trim off any tailing "/" here
                // TED TODO: Find where that transformation happens and make it return a None
                debug!(
                    "Searching for versioned cache key: {}/{}/{}/{}",
                    channel, package.origin, package.name, v
                );
                self.cli
                    .get(
                        &format!("{}/{}/{}/{}", channel, package.origin, package.name, v)
                            .trim_right_matches("/"),
                    )
                    .unwrap()
            }
            None => {
                debug!(
                    "Searching for cache key: {}/{}/{}",
                    channel, package.origin, package.name
                );
                self.cli
                    .get(&format!("{}/{}/{}", channel, package.origin, package.name))
                    .unwrap()
            }
        }
    }

    pub fn delete_package(&mut self, package: PackageIdent, channel: &str) {
        if package.version.is_none() || package.release.is_none() {
            debug!("Can't insert non-fully qualified package ident into the cache");
            return;
        }
        let keys = vec![
            format!("{}/{}/{}", channel, package.origin, package.name),
            format!(
                "{}/{}/{}/{}",
                channel,
                package.origin,
                package.name,
                package.clone().version.unwrap()
            ),
        ];
        for key in keys {
            match self.cli.delete(&key) {
                Ok(_) => debug!("Deleted {:?} from memcached", package.to_string()),
                Err(e) => warn!(
                    "Failed to delete {:?} from memcached: {}",
                    package.to_string(),
                    e
                ),
            };
        }
    }
}
