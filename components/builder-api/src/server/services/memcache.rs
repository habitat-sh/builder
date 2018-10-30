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
use sha2::{Digest, Sha512};

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
        let package_namespace = self.package_namespace(&ident.origin, &ident.name);
        let channel_namespace = self.channel_namespace(&ident.origin, channel);

        match self.cli.set(
            &format!(
                "{}/{}/{}:{}:{}",
                target,
                channel,
                ident.to_string(),
                channel_namespace,
                package_namespace
            ),
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
        let package_namespace = self.package_namespace(&package.origin, &package.name);
        let channel_namespace = self.channel_namespace(&package.origin, channel);

        trace!(
            "Getting {}/{}/{} from memcached",
            target,
            channel,
            package.to_string()
        );

        match self.get_string(&format!(
            "{}/{}/{}:{}:{}",
            target,
            channel,
            package.to_string(),
            channel_namespace,
            package_namespace
        )) {
            Some(json_body) => Some(json_body),
            None => None,
        }
    }

    pub fn clear_cache_for_package(&mut self, package: PackageIdent) {
        self.reset_namespace(&package_ns_key(&package.origin, &package.name));
    }

    pub fn clear_cache_for_channel(&mut self, origin: &str, channel: &str) {
        self.reset_namespace(&channel_ns_key(origin, channel));
    }

    pub fn get_session(&mut self, token: &str) -> Option<Session> {
        trace!("Getting session for user {} from memcached", token);

        match self.get_bytes(&hash_key(token)) {
            Some(session) => Some(protobuf::parse_from_bytes(&session).unwrap()),
            None => None,
        }
    }

    pub fn delete_session_key(&mut self, key: &str) {
        match self.cli.delete(&hash_key(key)) {
            Ok(b) => trace!("Deleted key {}, {:?}", key, b),
            Err(e) => warn!("Failed to delete key {}: {}", key, e),
        };
    }

    pub fn set_session(&mut self, token: &str, session: &Session, ttl: Option<u32>) {
        let computed_ttl = match ttl {
            Some(ttl) => ttl,
            None => self.ttl * 60,
        };

        match self.cli.set(
            &hash_key(token),
            session.write_to_bytes().unwrap().as_slice(),
            computed_ttl,
        ) {
            Ok(_) => trace!("Saved token to memcached!"),
            Err(e) => warn!("Failed to save token to memcached: {}", e),
        };
    }

    fn package_namespace(&mut self, origin: &str, name: &str) -> String {
        self.get_namespace(&package_ns_key(origin, name))
    }

    fn channel_namespace(&mut self, origin: &str, channel: &str) -> String {
        self.get_namespace(&channel_ns_key(origin, channel))
    }

    fn get_namespace(&mut self, namespace_key: &str) -> String {
        match self.get_string(namespace_key) {
            Some(value) => value,
            None => self.reset_namespace(namespace_key),
        }
    }

    fn reset_namespace(&mut self, namespace_key: &str) -> String {
        let epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.cli.set(namespace_key, epoch, self.ttl * 60).unwrap();
        format!("{}", epoch)
    }

    // These are to make the compiler happy
    fn get_bytes(&mut self, key: &str) -> Option<Vec<u8>> {
        match self.cli.get(key) {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Error getting key {}: {:?}", key, e);
                None
            }
        }
    }

    fn get_string(&mut self, key: &str) -> Option<String> {
        match self.cli.get(key) {
            Ok(string) => string,
            Err(e) => {
                warn!("Error getting key {}: {:?}", key, e);
                None
            }
        }
    }
}

fn package_ns_key(origin: &str, name: &str) -> String {
    format!("package:{}/{}", origin, name)
}

fn channel_ns_key(origin: &str, channel: &str) -> String {
    format!("channel:{}/{}", origin, channel)
}

fn hash_key(key: &str) -> String {
    let mut hasher = Sha512::new();
    hasher.input(key);
    format!("{:02x}", hasher.result())
}
