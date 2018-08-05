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
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::option::IntoIter;
use std::path::PathBuf;

use github_api_client::config::GitHubCfg;
use http_gateway::config::prelude::*;
use oauth_client::config::OAuth2Cfg;
use segment_api_client::SegmentCfg;
use typemap;

use error::Error;
use hab_core::package::target::{self, PackageTarget};

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub http: HttpCfg,
    /// List of net addresses for routing servers to connect to
    pub routers: Vec<RouterAddr>,
    pub oauth: OAuth2Cfg,
    pub github: GitHubCfg,
    pub segment: SegmentCfg,
    pub s3: S3Cfg,
    pub ui: UiCfg,
    /// Whether to log events for funnel metrics
    pub events_enabled: bool,
    /// Whether to enable builds for non-core origins
    pub non_core_builds_enabled: bool,
    /// Where to record log events for funnel metrics
    pub log_dir: String,
    /// Whether jobsrv is present or not
    pub jobsrv_enabled: bool,
    /// Whether to schedule builds on package upload
    pub builds_enabled: bool,
    /// Filepath to location on disk to store entities
    pub path: PathBuf,
    /// Filepath to where the builder encryption keys can be found
    pub key_dir: PathBuf,
    /// A list of package targets which can be uploaded and hosted
    pub targets: Vec<PackageTarget>,
    /// Upstream depot to pull packages from if someone tries to install from this depot and they
    /// aren't present. This is optional because e.g. public Builder doesn't have an upstream.
    pub upstream_depot: Option<String>,
    // Origins for which we pull from upstream (default: core)
    pub upstream_origins: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            http: HttpCfg::default(),
            routers: vec![RouterAddr::default()],
            oauth: OAuth2Cfg::default(),
            github: GitHubCfg::default(),
            s3: S3Cfg::default(),
            segment: SegmentCfg::default(),
            ui: UiCfg::default(),
            events_enabled: false,
            non_core_builds_enabled: true,
            log_dir: env::temp_dir().to_string_lossy().into_owned(),
            jobsrv_enabled: true,
            path: PathBuf::from("/hab/svc/builder-api/data"),
            builds_enabled: true,
            key_dir: PathBuf::from("/hab/svc/builder-api/files"),
            targets: vec![target::X86_64_LINUX, target::X86_64_WINDOWS],
            upstream_depot: None,
            upstream_origins: vec!["core".to_string()],
        }
    }
}

impl ConfigFile for Config {
    type Error = Error;
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

impl typemap::Key for Config {
    type Value = Self;
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
        events_enabled = true
        non_core_builds_enabled = true
        jobsrv_enabled = false

        [http]
        listen = "0:0:0:0:0:0:0:1"
        port = 9636
        handler_count = 128

        [ui]
        root = "/some/path"

        [[targets]]
        platform = "linux"
        architecture = "x86_64"

        [[targets]]
        platform = "windows"
        architecture = "x86_64"

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
        assert_eq!(config.events_enabled, true);
        assert_eq!(config.jobsrv_enabled, false);
        assert_eq!(config.non_core_builds_enabled, true);
        assert_eq!(&format!("{}", config.http.listen), "::1");
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
        assert_eq!(config.events_enabled, false);
        assert_eq!(config.non_core_builds_enabled, true);
        assert_eq!(config.http.port, 9000);
    }
}
