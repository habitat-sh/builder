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

use std::time::SystemTime;

use config::MemcacheCfg;
use hab_core::package::PackageIdent;
use memcache;
use protobuf;
use protobuf::Message;
use protocol::originsrv::Session;

pub struct MemcacheClient {
    cli: memcache::Client,
    ttl: u32,
}

impl MemcacheClient {
    pub fn new(config: MemcacheCfg) -> Self {
        let memcache_host_strings: Vec<String> =
            config.hosts.iter().map(|h| format!("{}", h)).collect();
        let memcache_hosts: Vec<&str> = memcache_host_strings.iter().map(AsRef::as_ref).collect();
        MemcacheClient {
            cli: memcache::Client::new(memcache_hosts).unwrap(),
            ttl: config.ttl,
        }
    }

    pub fn set_package(
        &mut self,
        ident: PackageIdent,
        pkg_json: &str,
        channel: &str,
        target: &str,
    ) {
        let namespace = self.namespace(&ident.origin, &ident.name);

        match self.cli.set(
            &format!("{}/{}/{}:{}", target, channel, ident.to_string(), namespace),
            pkg_json,
            self.ttl * 60,
        ) {
            Ok(_) => trace!(
                "Saved {}/{}/{} to memcached",
                target,
                channel,
                ident.to_string()
            ),
            Err(e) => warn!(
                "Failed to save {}/{}/{} to memcached: {:?}",
                target,
                channel,
                ident.to_string(),
                e
            ),
        };
    }

    pub fn get_package(
        &mut self,
        package: PackageIdent,
        channel: &str,
        target: &str,
    ) -> Option<String> {
        let namespace = self.namespace(&package.origin, &package.name);
        trace!(
            "Getting {}/{}/{} from memcached",
            target,
            channel,
            package.to_string()
        );

        match self.get_string(&format!(
            "{}/{}/{}:{}",
            target,
            channel,
            package.to_string(),
            namespace
        )) {
            Some(json_body) => Some(json_body),
            None => None,
        }
    }

    pub fn clear_cache_for_package(&mut self, package: PackageIdent) {
        let epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let namespace = format!("{}/{}", package.origin, package.name);
        self.cli.set(&namespace, epoch, self.ttl * 60).unwrap()
    }

    pub fn get_session(&mut self, user: &str) -> Option<Session> {
        trace!("Getting session for user {} from memcached", user);

        match self.get_bytes(&format!("{}", user)) {
            Some(session) => Some(protobuf::parse_from_bytes(&session).unwrap()),
            None => None,
        }
    }

    pub fn delete_key(&mut self, key: &str) {
        match self.cli.delete(key) {
            Ok(b) => trace!("Deleted key {}, {:?}", key, b),
            Err(e) => warn!("Failed to delete key {}: {}", key, e),
        };
    }

    pub fn set_session(&mut self, user: &str, session: &Session) {
        match self.cli.set(
            &format!("{}", user),
            session.write_to_bytes().unwrap().as_slice(),
            self.ttl * 60,
        ) {
            Ok(_) => trace!("Saved session for {} to memcached", user),
            Err(e) => warn!("Failed to save user {} to memcached: {}", user, e),
        };
    }

    fn namespace(&mut self, origin: &str, name: &str) -> String {
        let namespace_key = format!("{}/{}", origin, name);
        match self.get_string(&namespace_key) {
            Some(value) => value,
            None => {
                let epoch = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                self.cli.set(&namespace_key, epoch, self.ttl * 60).unwrap();
                format!("{}", epoch)
            }
        }
    }

    // These are to make the compiler happy
    fn get_bytes(&mut self, key: &str) -> Option<Vec<u8>> {
        self.cli.get(key).unwrap()
    }

    fn get_string(&mut self, key: &str) -> Option<String> {
        self.cli.get(key).unwrap()
    }
}
