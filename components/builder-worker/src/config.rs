//! Configuration for a Habitat JobSrv Worker

use crate::error::Error;
use github_api_client::config::GitHubCfg;
use habitat_core::{config::ConfigFile,
                   crypto::keys::KeyCache,
                   package::PackageTarget,
                   url,
                   ChannelIdent};
use std::{net::{IpAddr,
                Ipv4Addr},
          path::PathBuf,
          str::FromStr};

pub type JobSrvCfg = Vec<JobSrvAddr>;

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Enable automatic publishing for all builds by default
    pub auto_publish:       bool,
    /// Filepath where persistent application data is stored
    pub data_path:          PathBuf,
    /// Location of Builder encryption keys
    pub key_dir:            KeyCache,
    /// Path to worker event logs
    pub log_path:           PathBuf,
    /// Default channel name for Publish post-processor to use to determine which channel to
    /// publish artifacts to
    pub bldr_channel:       ChannelIdent,
    /// Default URL for Publish post-processor to use to determine which Builder to use
    /// for retrieving signing keys and publishing artifacts
    pub bldr_url:           String,
    /// List of Job Servers to connect to
    pub jobsrv:             JobSrvCfg,
    pub features_enabled:   String,
    /// Github application id to use for private repo access
    pub github:             GitHubCfg,
    pub target:             PackageTarget,
    /// The frequency to poll the zmq socket for messages from jobsrv, in seconds
    pub work_poll_interval: usize,
}

impl Config {
    pub fn jobsrv_addrs(&self) -> Vec<(String, String, String)> {
        let mut addrs = vec![];
        for job_server in &self.jobsrv {
            let hb = format!("tcp://{}:{}", job_server.host, job_server.heartbeat);
            let queue = format!("tcp://{}:{}", job_server.host, job_server.port);
            let log = format!("tcp://{}:{}", job_server.host, job_server.log_port);
            addrs.push((hb, queue, log));
        }
        addrs
    }
}

impl Default for Config {
    fn default() -> Self {
        Config { auto_publish:       true,
                 data_path:          PathBuf::from("/tmp"),
                 log_path:           PathBuf::from("/tmp"),
                 key_dir:            KeyCache::new("/hab/svc/builder-worker/files"),
                 bldr_channel:       ChannelIdent::unstable(),
                 bldr_url:           url::default_bldr_url(),
                 jobsrv:             vec![JobSrvAddr::default()],
                 features_enabled:   "".to_string(),
                 github:             GitHubCfg::default(),
                 target:             PackageTarget::from_str("x86_64-linux").unwrap(),
                 work_poll_interval: 60, }
    }
}

impl ConfigFile for Config {
    type Error = Error;
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct JobSrvAddr {
    pub host:      IpAddr,
    pub port:      u16,
    pub heartbeat: u16,
    pub log_port:  u16,
}

impl Default for JobSrvAddr {
    fn default() -> Self {
        JobSrvAddr { host:      IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                     port:      5566,
                     heartbeat: 5567,
                     log_port:  5568, }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_file() {
        let content = r#"
        data_path = "/path/to/data"
        log_path = "/path/to/logs"
        key_dir = "/path/to/key"
        features_enabled = "FOO,BAR"
        target = "x86_64-linux-kernel2"
        work_poll_interval = 10

        [[jobsrv]]
        host = "1:1:1:1:1:1:1:1"
        port = 9000
        heartbeat = 9001
        log_port = 9021

        [[jobsrv]]
        host = "2.2.2.2"
        port = 9000
        "#;

        let config = Config::from_raw(&content).unwrap();
        assert_eq!(&format!("{}", config.data_path.display()), "/path/to/data");
        assert_eq!(&format!("{}", config.log_path.display()), "/path/to/logs");
        assert_eq!(config.key_dir, KeyCache::new("/path/to/key"));
        assert_eq!(&format!("{}", config.jobsrv[0].host), "1:1:1:1:1:1:1:1");
        assert_eq!(config.jobsrv[0].port, 9000);
        assert_eq!(config.jobsrv[0].heartbeat, 9001);
        assert_eq!(config.jobsrv[0].log_port, 9021);
        assert_eq!(&format!("{}", config.jobsrv[1].host), "2.2.2.2");
        assert_eq!(config.jobsrv[1].port, 9000);
        assert_eq!(config.jobsrv[1].heartbeat, 5567);
        assert_eq!(&config.features_enabled, "FOO,BAR");
        assert_eq!(config.target,
                   PackageTarget::from_str("x86_64-linux-kernel2").unwrap());
        assert_eq!(config.work_poll_interval, 10);
    }
}
