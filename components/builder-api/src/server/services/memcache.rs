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

use memcache;
use protobuf;
use protobuf::Message;
use rand::{self, Rng};
use sha2::{Digest, Sha512};
use time::PreciseTime;

use super::metrics::Histogram;

use crate::bldr_core::metrics::HistogramMetric;
use crate::config::MemcacheCfg;
use crate::hab_core::package::PackageIdent;
use crate::protocol::originsrv::Session;

pub struct MemcacheClient {
    cli: memcache::Client,
    ttl: u32,
}

impl MemcacheClient {
    pub fn new(config: &MemcacheCfg) -> Self {
        trace!("Creating memcache client, hosts: {:?}", config.hosts);
        let memcache_host_strings: Vec<String> = config
            .hosts
            .iter()
            .map(|h| format!("{}?tcp_nodelay=true", h)) // tcp_nodelay is a significant perf gain
            .collect();
        let memcache_hosts: Vec<&str> = memcache_host_strings.iter().map(AsRef::as_ref).collect();
        MemcacheClient {
            cli: memcache::Client::new(memcache_hosts).unwrap(),
            ttl: config.ttl,
        }
    }

    pub fn set_package(
        &mut self,
        ident: &PackageIdent,
        pkg_json: Option<&str>,
        channel: &str,
        target: &str,
        opt_account_id: Option<u64>,
    ) {
        let package_namespace = self.package_namespace(&ident.origin, &ident.name);
        let channel_namespace = self.channel_namespace(&ident.origin, channel);

        let account_str = match opt_account_id {
            Some(id) => format!(":{}", id),
            None => "".to_string(),
        };

        let body = match pkg_json {
            Some(json) => json,
            None => "404",
        };

        match self.cli.set(
            &format!(
                "{}/{}/{}:{}:{}{}",
                target,
                channel,
                ident.to_string(),
                channel_namespace,
                package_namespace,
                account_str
            ),
            body,
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
        ident: &PackageIdent,
        channel: &str,
        target: &str,
        opt_account_id: Option<u64>,
    ) -> (bool, Option<String>) {
        let package_namespace = self.package_namespace(&ident.origin, &ident.name);
        let channel_namespace = self.channel_namespace(&ident.origin, channel);

        trace!(
            "Getting {}/{}/{} from memcached for {:?}",
            target,
            channel,
            ident.to_string(),
            opt_account_id
        );

        let account_str = match opt_account_id {
            Some(id) => format!(":{}", id),
            None => "".to_string(),
        };

        let start_time = PreciseTime::now();
        match self.get_string(&format!(
            "{}/{}/{}:{}:{}{}",
            target,
            channel,
            ident.to_string(),
            channel_namespace,
            package_namespace,
            account_str
        )) {
            Some(json_body) => {
                let end_time = PreciseTime::now();
                trace!(
                    "Memcache get_package time: {} ms",
                    start_time.to(end_time).num_milliseconds()
                );
                Histogram::MemcacheCallTime.set(start_time.to(end_time).num_milliseconds() as f64);

                if json_body == "404" {
                    (true, None)
                } else {
                    (true, Some(json_body))
                }
            }
            None => (false, None),
        }
    }

    pub fn clear_cache_for_package(&mut self, ident: &PackageIdent) {
        self.reset_namespace(&package_ns_key(&ident.origin, &ident.name));
    }

    pub fn clear_cache_for_channel(&mut self, origin: &str, channel: &str) {
        self.reset_namespace(&channel_ns_key(origin, channel));
    }

    pub fn get_session(&mut self, token: &str) -> Option<Session> {
        trace!("Getting session for token {} from memcached", token);

        let start_time = PreciseTime::now();
        match self.get_bytes(&hash_key(token)) {
            Some(session) => {
                let end_time = PreciseTime::now();
                trace!(
                    "Memcache get_session time: {} ms",
                    start_time.to(end_time).num_milliseconds()
                );
                Histogram::MemcacheCallTime.set(start_time.to(end_time).num_milliseconds() as f64);
                Some(protobuf::parse_from_bytes(&session).unwrap())
            }
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
            Ok(_) => trace!("Saved session to memcached!"),
            Err(e) => warn!("Failed to save session to memcached: {}", e),
        };
    }

    pub fn set_origin_member(&mut self, origin: &str, account_id: u64, val: bool) {
        let key = format!("member:{}/{}", origin, account_id);

        match self.cli.set(&key, val, self.ttl * 60) {
            Ok(_) => trace!(
                "Saved origin membership {}/{} to memcached!",
                origin,
                account_id
            ),
            Err(e) => warn!("Failed to save origin membership to memcached: {}", e),
        }
    }

    pub fn get_origin_member(&mut self, origin: &str, account_id: u64) -> Option<bool> {
        trace!(
            "Getting origin membership for {} {} from memcached",
            origin,
            account_id
        );

        let key = format!("member:{}/{}", origin, account_id);

        let start_time = PreciseTime::now();
        let ret = self.get_bool(&key);
        let end_time = PreciseTime::now();
        trace!(
            "Memcache get_origin_member time: {} ms",
            start_time.to(end_time).num_milliseconds()
        );
        Histogram::MemcacheCallTime.set(start_time.to(end_time).num_milliseconds() as f64);

        ret
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
        let mut rng = rand::thread_rng();
        let val: u64 = rng.gen();
        trace!("Reset namespace {} to {}", namespace_key, val);
        self.cli.set(namespace_key, val, self.ttl * 60).unwrap();
        format!("{}", val)
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

    fn get_bool(&mut self, key: &str) -> Option<bool> {
        match self.cli.get(key) {
            Ok(val) => val,
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
