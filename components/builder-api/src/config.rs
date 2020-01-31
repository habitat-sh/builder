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

//! Configuration for a Habitat Builder-API service

use std::{env,
          error,
          fmt,
          io,
          net::{IpAddr,
                Ipv4Addr,
                SocketAddr,
                ToSocketAddrs},
          option::IntoIter,
          path::PathBuf};

use num_cpus;

use artifactory_client::config::ArtifactoryCfg;
use github_api_client::config::GitHubCfg;
use oauth_client::config::OAuth2Cfg;

use crate::{db::config::DataStoreCfg,
            hab_core::{self,
                       config::ConfigFile,
                       package::target::{self,
                                         PackageTarget}}};

pub trait GatewayCfg {
    /// Default number of worker threads to simultaneously handle HTTP requests.
    fn default_handler_count() -> usize { num_cpus::get() * 8 }

    /// Number of worker threads to simultaneously handle HTTP requests.
    fn handler_count(&self) -> usize { Self::default_handler_count() }

    fn listen_addr(&self) -> &IpAddr;

    fn listen_port(&self) -> u16;
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub api:         ApiCfg,
    pub artifactory: ArtifactoryCfg,
    pub github:      GitHubCfg,
    pub http:        HttpCfg,
    pub oauth:       OAuth2Cfg,
    pub s3:          S3Cfg,
    pub ui:          UiCfg,
    pub memcache:    MemcacheCfg,
    pub jobsrv:      JobsrvCfg,
    pub datastore:   DataStoreCfg,
}

impl Default for Config {
    fn default() -> Self {
        Config { api:         ApiCfg::default(),
                 artifactory: ArtifactoryCfg::default(),
                 github:      GitHubCfg::default(),
                 http:        HttpCfg::default(),
                 oauth:       OAuth2Cfg::default(),
                 s3:          S3Cfg::default(),
                 ui:          UiCfg::default(),
                 memcache:    MemcacheCfg::default(),
                 jobsrv:      JobsrvCfg::default(),
                 datastore:   DataStoreCfg::default(), }
    }
}

#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", *self) }
}

impl error::Error for ConfigError {
    fn description(&self) -> &str { "Error reading config file" }
}

impl ConfigFile for Config {
    type Error = ConfigError;
}

impl From<hab_core::Error> for ConfigError {
    fn from(err: hab_core::Error) -> ConfigError { ConfigError(format!("{:?}", err)) }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum S3Backend {
    Aws,
    Minio,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct S3Cfg {
    // These are for using S3 as the artifact storage
    pub key_id:      String,
    pub secret_key:  String,
    pub bucket_name: String,
    pub backend:     S3Backend,
    pub endpoint:    String,
}

impl Default for S3Cfg {
    fn default() -> Self {
        S3Cfg { key_id:      String::from("depot"),
                secret_key:  String::from("password"),
                bucket_name: String::from("habitat-builder-artifact-store.default"),
                backend:     S3Backend::Minio,
                endpoint:    String::from("http://localhost:9000"), }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ApiCfg {
    pub data_path:        PathBuf,
    pub log_path:         PathBuf,
    pub key_path:         PathBuf,
    pub targets:          Vec<PackageTarget>,
    pub build_targets:    Vec<PackageTarget>,
    pub features_enabled: String,
    pub build_on_upload:  bool,
    pub private_max_age:  usize,
}

impl Default for ApiCfg {
    fn default() -> Self {
        ApiCfg { data_path:        PathBuf::from("/hab/svc/builder-api/data"),
                 log_path:         env::temp_dir(),
                 key_path:         PathBuf::from("/hab/svc/builder-api/files"),
                 targets:          vec![target::X86_64_LINUX,
                                        target::X86_64_LINUX_KERNEL2,
                                        target::X86_64_WINDOWS,],
                 build_targets:    vec![target::X86_64_LINUX, target::X86_64_WINDOWS],
                 features_enabled: String::from("jobsrv"),
                 build_on_upload:  true,
                 private_max_age:  300, }
    }
}

impl GatewayCfg for Config {
    fn handler_count(&self) -> usize { self.http.handler_count }

    fn listen_addr(&self) -> &IpAddr { &self.http.listen }

    fn listen_port(&self) -> u16 { self.http.port }
}

/// Public listening net address for HTTP requests
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct HttpCfg {
    pub listen:        IpAddr,
    pub port:          u16,
    pub tls:           Option<TLSServerCfg>,
    pub handler_count: usize,
    pub keep_alive:    usize,
}

/// Optional TLS configuration
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct TLSServerCfg {
    pub cert_path:    PathBuf,
    pub key_path:     PathBuf,
    pub ca_cert_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct TLSClientCfg {
    pub cert_path:    Option<PathBuf>,
    pub key_path:     Option<PathBuf>,
    pub ca_cert_path: Option<PathBuf>,
    pub verify:       bool,
}

impl Default for TLSServerCfg {
    fn default() -> Self {
        TLSServerCfg { cert_path:    PathBuf::from("/hab/svc/builder-api/files/service.crt"),
                       key_path:     PathBuf::from("/hab/svc/builder-api/files/service.key"),
                       ca_cert_path: None, }
    }
}

impl Default for TLSClientCfg {
    fn default() -> Self {
        TLSClientCfg { cert_path:    None,
                       key_path:     None,
                       ca_cert_path: None,
                       verify:       true, }
    }
}

impl Default for HttpCfg {
    fn default() -> Self {
        HttpCfg { listen:        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                  port:          9636,
                  tls:           None,
                  handler_count: Config::default_handler_count(),
                  keep_alive:    60, }
    }
}

impl ToSocketAddrs for HttpCfg {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<IntoIter<SocketAddr>> {
        match self.listen {
            IpAddr::V4(ref a) => (*a, self.port).to_socket_addrs(),
            IpAddr::V6(ref a) => (*a, self.port).to_socket_addrs(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct UiCfg {
    /// Path to UI files to host over HTTP. If not set the UI will be disabled.
    pub root: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MemcacheCfgHosts {
    pub host: String,
    pub port: u16,
    pub tls:  Option<TLSClientCfg>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MemcacheCfg {
    pub ttl:   u32,
    pub hosts: Vec<MemcacheCfgHosts>,
}

impl Default for MemcacheCfgHosts {
    fn default() -> Self {
        MemcacheCfgHosts { host: String::from("localhost"),
                           port: 11211,
                           tls:  None, }
    }
}

impl Default for MemcacheCfg {
    fn default() -> Self {
        MemcacheCfg { hosts: vec![MemcacheCfgHosts::default()],
                      ttl:   15, }
    }
}

impl MemcacheCfg {
    pub fn memcache_hosts(&self) -> Vec<String> {
        self.hosts
            .iter()
            .map(|h| h.to_string_with_params())
            .collect()
    }
}

impl MemcacheCfgHosts {
    pub fn to_string_with_params(&self) -> String {
        let mut url = format!("{}?tcp_nodelay=true", self); // tcp_nodelay is a significant perf gain
        if let Some(tls_config) = &self.tls {
            if tls_config.ca_cert_path.is_some() {
                url.push_str(&format!("&ca_path={}",
                                      tls_config.ca_cert_path.as_ref().unwrap().to_string_lossy()))
            }

            if tls_config.key_path.is_some() {
                url.push_str(&format!("&key_path={}",
                                      tls_config.key_path.as_ref().unwrap().to_string_lossy()))
            }

            if tls_config.cert_path.is_some() {
                url.push_str(&format!("&cert_path={}",
                                      tls_config.cert_path.as_ref().unwrap().to_string_lossy()))
            }

            if tls_config.verify {
                url.push_str("&verify_mode=peer");
            } else {
                url.push_str("&verify_mode=none");
            }
        }
        url
    }
}

impl fmt::Display for MemcacheCfgHosts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.tls {
            Some(_) => write!(f, "memcache+tls://{}:{}", self.host, self.port),
            None => write!(f, "memcache://{}:{}", self.host, self.port),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct JobsrvCfg {
    pub host: String,
    pub port: u16,
}

impl Default for JobsrvCfg {
    fn default() -> Self {
        JobsrvCfg { host: String::from("localhost"),
                    port: 5580, }
    }
}

impl fmt::Display for JobsrvCfg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "http://{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn config_from_file() {
        let content = r#"
        [api]
        data_path = "/hab/svc/hab-depot/data"
        log_path = "/hab/svc/hab-depot/var/log"
        key_path = "/hab/svc/hab-depot/files"
        targets = ["x86_64-linux", "x86_64-linux-kernel2", "x86_64-windows"]
        build_targets = ["x86_64-linux"]
        features_enabled = "foo, bar"
        build_on_upload = false
        private_max_age = 400

        [http]
        listen = "0:0:0:0:0:0:0:1"
        port = 9636
        handler_count = 128
        keep_alive = 30

        [memcache]
        ttl = 11
        [[memcache.hosts]]
        host = "192.168.0.1"
        port = 12345

        [ui]
        root = "/some/path"

        [oauth]
        client_id = "0c2f738a7d0bd300de10"
        client_secret = "438223113eeb6e7edf2d2f91a232b72de72b9bdf"

        [s3]
        backend = "minio"
        key_id = "AWSKEYIDORSOMETHING"
        secret_key = "aW5S3c437Key7hIn817s7o7a11yN457y70Wr173L1k37h15"
        endpoint = "http://localhost:9000"
        bucket_name = "hibbity-bibbity-poopity-scoopity"

        [artifactory]
        api_url = "http://abcde"
        api_key = "secret"
        repo = "abracadabra"

        [github]
        api_url = "https://api.github.com"

        [jobsrv]
        host = "1.2.3.4"
        port = 1234

        [datastore]
        host = "1.1.1.1"
        port = 9000
        user = "test"
        database = "test"
        connection_retry_ms = 500
        connection_timeout_sec = 4800
        connection_test = true
        pool_size = 1
        ssl_mode = "verify_ca"
        ssl_root_cert = "/root_ca.crt"
        ssl_key = "/ssl.key"
        ssl_cert = "/ssl.crt"
        "#;

        let config = Config::from_raw(&content).unwrap();
        assert_eq!(config.api.data_path,
                   PathBuf::from("/hab/svc/hab-depot/data"));
        assert_eq!(config.api.log_path,
                   PathBuf::from("/hab/svc/hab-depot/var/log"));
        assert_eq!(config.api.key_path,
                   PathBuf::from("/hab/svc/hab-depot/files"));

        assert_eq!(config.api.targets.len(), 3);
        assert_eq!(config.api.targets[0], target::X86_64_LINUX);
        assert_eq!(config.api.targets[1], target::X86_64_LINUX_KERNEL2);
        assert_eq!(config.api.targets[2], target::X86_64_WINDOWS);

        assert_eq!(config.api.build_targets.len(), 1);
        assert_eq!(config.api.build_targets[0], target::X86_64_LINUX);

        assert_eq!(&config.api.features_enabled, "foo, bar");
        assert_eq!(config.api.build_on_upload, false);
        assert_eq!(config.api.private_max_age, 400);

        assert_eq!(&format!("{}", config.http.listen), "::1");

        assert_eq!(config.memcache.ttl, 11);
        assert_eq!(&format!("{}", config.memcache.hosts[0]),
                   "memcache://192.168.0.1:12345");

        assert_eq!(&format!("{}", config.jobsrv), "http://1.2.3.4:1234");

        assert_eq!(config.http.port, 9636);
        assert_eq!(config.http.handler_count, 128);
        assert_eq!(config.http.keep_alive, 30);

        assert_eq!(config.oauth.client_id, "0c2f738a7d0bd300de10");
        assert_eq!(config.oauth.client_secret,
                   "438223113eeb6e7edf2d2f91a232b72de72b9bdf");

        assert_eq!(config.github.api_url, "https://api.github.com");

        assert_eq!(config.ui.root, Some("/some/path".to_string()));

        assert_eq!(config.s3.backend, S3Backend::Minio);
        assert_eq!(config.s3.key_id, "AWSKEYIDORSOMETHING");
        assert_eq!(config.s3.secret_key,
                   "aW5S3c437Key7hIn817s7o7a11yN457y70Wr173L1k37h15");
        assert_eq!(config.s3.endpoint, "http://localhost:9000");
        assert_eq!(config.s3.bucket_name, "hibbity-bibbity-poopity-scoopity");

        assert_eq!(config.artifactory.api_url, "http://abcde");
        assert_eq!(config.artifactory.api_key, "secret");
        assert_eq!(config.artifactory.repo, "abracadabra");

        assert_eq!(config.datastore.port, 9000);
        assert_eq!(config.datastore.user, "test");
        assert_eq!(config.datastore.database, "test");
        assert_eq!(config.datastore.connection_retry_ms, 500);
        assert_eq!(config.datastore.connection_timeout_sec, 4800);
        assert_eq!(config.datastore.connection_test, true);
        assert_eq!(config.datastore.pool_size, 1);
        assert_eq!(config.datastore.ssl_mode, Some("verify_ca".to_string()));
        assert_eq!(config.datastore.ssl_root_cert,
                   Some("/root_ca.crt".to_string()));
        assert_eq!(config.datastore.ssl_key, Some("/ssl.key".to_string()));
        assert_eq!(config.datastore.ssl_cert, Some("/ssl.crt".to_string()));
    }

    #[test]
    fn config_from_file_defaults() {
        let content = r#"
        [http]
        port = 9000
        "#;

        let config = Config::from_raw(&content).unwrap();
        assert_eq!(config.http.port, 9000);
    }
}
