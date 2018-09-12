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

use std::env;
use std::error;
use std::fmt;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::option::IntoIter;
use std::path::PathBuf;

use num_cpus;

use hab_core;
use hab_core::config::ConfigFile;
use hab_core::package::target::{self, PackageTarget};
use hab_net::app::config::RouterAddr;

use github_api_client::config::GitHubCfg;
use oauth_client::config::OAuth2Cfg;
use segment_api_client::SegmentCfg;

pub trait GatewayCfg {
    /// Default number of worker threads to simultaneously handle HTTP requests.
    fn default_handler_count() -> usize {
        num_cpus::get() * 8
    }

    /// Number of worker threads to simultaneously handle HTTP requests.
    fn handler_count(&self) -> usize {
        Self::default_handler_count()
    }

    fn listen_addr(&self) -> &IpAddr;

    fn listen_port(&self) -> u16;

    /// Return a list of router addresses
    fn route_addrs(&self) -> &[RouterAddr];
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub api: ApiCfg,
    pub github: GitHubCfg,
    pub http: HttpCfg,
    pub oauth: OAuth2Cfg,
    pub routers: Vec<RouterAddr>,
    pub s3: S3Cfg,
    pub segment: SegmentCfg,
    pub ui: UiCfg,
    pub upstream: UpstreamCfg,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api: ApiCfg::default(),
            github: GitHubCfg::default(),
            http: HttpCfg::default(),
            oauth: OAuth2Cfg::default(),
            routers: vec![RouterAddr::default()],
            s3: S3Cfg::default(),
            segment: SegmentCfg::default(),
            ui: UiCfg::default(),
            upstream: UpstreamCfg::default(),
        }
    }
}

#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", *self)
    }
}

impl error::Error for ConfigError {
    fn description(&self) -> &str {
        "Error reading config file"
    }
}

impl ConfigFile for Config {
    type Error = ConfigError;
}

impl From<hab_core::Error> for ConfigError {
    fn from(err: hab_core::Error) -> ConfigError {
        ConfigError(format!("{:?}", err))
    }
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
    pub key_id: String,
    pub secret_key: String,
    pub bucket_name: String,
    pub backend: S3Backend,
    pub endpoint: String,
}

impl Default for S3Cfg {
    fn default() -> Self {
        S3Cfg {
            key_id: String::from("depot"),
            secret_key: String::from("password"),
            bucket_name: String::from("habitat-builder-artifact-store.default"),
            backend: S3Backend::Minio,
            endpoint: String::from("http://localhost:9000"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UpstreamCfg {
    pub endpoint: String,
    pub origins: Vec<String>,
}

impl Default for UpstreamCfg {
    fn default() -> Self {
        UpstreamCfg {
            endpoint: String::from("http://localhost"),
            origins: vec!["core".to_string()],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ApiCfg {
    pub data_path: PathBuf,
    pub log_path: PathBuf,
    pub key_path: PathBuf,
    pub targets: Vec<PackageTarget>,
    pub features_enabled: String,
}

impl Default for ApiCfg {
    fn default() -> Self {
        ApiCfg {
            data_path: PathBuf::from("/hab/svc/builder-api/data"),
            log_path: env::temp_dir(),
            key_path: PathBuf::from("/hab/svc/builder-api/files"),
            targets: vec![
                target::X86_64_LINUX,
                target::X86_64_LINUX_KERNEL2,
                target::X86_64_WINDOWS,
            ],
            features_enabled: String::from("jobsrv"),
        }
    }
}

impl GatewayCfg for Config {
    fn handler_count(&self) -> usize {
        self.http.handler_count
    }

    fn listen_addr(&self) -> &IpAddr {
        &self.http.listen
    }

    fn listen_port(&self) -> u16 {
        self.http.port
    }

    fn route_addrs(&self) -> &[RouterAddr] {
        self.routers.as_slice()
    }
}

/// Public listening net address for HTTP requests
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct HttpCfg {
    pub listen: IpAddr,
    pub port: u16,
    pub handler_count: usize,
}

impl Default for HttpCfg {
    fn default() -> Self {
        HttpCfg {
            listen: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            port: 9636,
            handler_count: Config::default_handler_count(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_file() {
        let content = r#"
        [api]
        data_path = "/hab/svc/hab-depot/data"
        log_path = "/hab/svc/hab-depot/var/log"
        key_path = "/hab/svc/hab-depot/files"
        targets = ["x86_64-linux", "x86_64-linux-kernel2", "x86_64-windows"]
        features_enabled = "foo, bar"

        [upstream]
        endpoint = "http://example.com"
        origins = ["foo", "bar"]

        [http]
        listen = "0:0:0:0:0:0:0:1"
        port = 9636
        handler_count = 128

        [ui]
        root = "/some/path"

        [[routers]]
        host = "172.18.0.2"
        port = 9632

        [oauth]
        client_id = "0c2f738a7d0bd300de10"
        client_secret = "438223113eeb6e7edf2d2f91a232b72de72b9bdf"

        [s3]
        backend = "minio"
        key_id = "AWSKEYIDORSOMETHING"
        secret_key = "aW5S3c437Key7hIn817s7o7a11yN457y70Wr173L1k37h15"
        endpoint = "http://localhost:9000"
        bucket_name = "hibbity-bibbity-poopity-scoopity"

        [github]
        api_url = "https://api.github.com"
        "#;

        let config = Config::from_raw(&content).unwrap();
        assert_eq!(
            config.api.data_path,
            PathBuf::from("/hab/svc/hab-depot/data")
        );
        assert_eq!(
            config.api.log_path,
            PathBuf::from("/hab/svc/hab-depot/var/log")
        );
        assert_eq!(
            config.api.key_path,
            PathBuf::from("/hab/svc/hab-depot/files")
        );

        assert_eq!(config.api.targets.len(), 3);
        assert_eq!(config.api.targets[0], target::X86_64_LINUX);
        assert_eq!(config.api.targets[1], target::X86_64_LINUX_KERNEL2);
        assert_eq!(config.api.targets[2], target::X86_64_WINDOWS);

        assert_eq!(&config.api.features_enabled, "foo, bar");

        assert_eq!(&format!("{}", config.http.listen), "::1");

        assert_eq!(config.upstream.endpoint, String::from("http://example.com"));
        assert_eq!(
            config.upstream.origins,
            vec!["foo".to_string(), "bar".to_string()]
        );

        assert_eq!(config.http.port, 9636);
        assert_eq!(config.http.handler_count, 128);
        assert_eq!(&format!("{}", config.routers[0]), "172.18.0.2:9632");

        assert_eq!(config.oauth.client_id, "0c2f738a7d0bd300de10");
        assert_eq!(
            config.oauth.client_secret,
            "438223113eeb6e7edf2d2f91a232b72de72b9bdf"
        );

        assert_eq!(config.github.api_url, "https://api.github.com");

        assert_eq!(config.ui.root, Some("/some/path".to_string()));

        assert_eq!(config.segment.url, "https://api.segment.io");

        assert_eq!(config.s3.backend, S3Backend::Minio);
        assert_eq!(config.s3.key_id, "AWSKEYIDORSOMETHING");
        assert_eq!(
            config.s3.secret_key,
            "aW5S3c437Key7hIn817s7o7a11yN457y70Wr173L1k37h15"
        );
        assert_eq!(config.s3.endpoint, "http://localhost:9000");
        assert_eq!(config.s3.bucket_name, "hibbity-bibbity-poopity-scoopity");
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
