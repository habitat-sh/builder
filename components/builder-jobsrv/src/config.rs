//! Configuration for a Habitat JobSrv service

use crate::{db::config::DataStoreCfg,
            error::Error,
            server::log_archiver::ArchiveBackend};
use habitat_core::{config::ConfigFile,
                   package::target::{self,
                                     PackageTarget}};
use std::{collections::HashSet,
          env,
          io,
          iter::FromIterator,
          net::{IpAddr,
                Ipv4Addr,
                SocketAddr,
                ToSocketAddrs},
          option::IntoIter,
          path::PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub net:              NetCfg,
    pub http:             HttpCfg,
    pub datastore:        DataStoreCfg,
    /// Directory to which log output of running build processes will
    /// be written. Defaults to the system temp directory. Must exist
    /// and be writable by the server process.
    pub log_dir:          PathBuf,
    /// Configuration for the job log archiver
    pub archive:          ArchiveCfg,
    /// Filepath to where the builder encryption keys can be found
    pub key_dir:          PathBuf,
    /// Path to scheduler event logs
    pub log_path:         PathBuf,
    /// Max time (in minutes) allowed for a build job
    pub job_timeout:      u64,
    /// Supported build targets
    pub build_targets:    HashSet<PackageTarget>,
    /// Feature flag toggles
    pub features_enabled: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut datastore = DataStoreCfg::default();
        datastore.database = String::from("builder");
        Config { net: NetCfg::default(),
                 http: HttpCfg::default(),
                 datastore,
                 log_dir: env::temp_dir(),
                 archive: ArchiveCfg::default(),
                 key_dir: PathBuf::from("/hab/svc/hab-depot/files"),
                 log_path: PathBuf::from("/tmp"),
                 job_timeout: 60,
                 build_targets: HashSet::from_iter(vec![target::X86_64_LINUX,
                                                        target::X86_64_WINDOWS]),
                 features_enabled: String::from("builddeps") }
    }
}

impl ConfigFile for Config {
    type Error = Error;
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct NetCfg {
    /// Worker Command socket's listening address
    pub worker_command_listen:   IpAddr,
    /// Worker Command socket's port
    pub worker_command_port:     u16,
    /// Worker Heartbeat socket's listening address
    pub worker_heartbeat_listen: IpAddr,
    /// Worker Heartbeat socket's port
    pub worker_heartbeat_port:   u16,
    /// Worker Log Ingestion socket's listening address
    pub log_ingestion_listen:    IpAddr,
    /// Worker Log Ingestion socket's port
    pub log_ingestion_port:      u16,
}

impl NetCfg {
    pub fn worker_command_addr(&self) -> String {
        format!("tcp://{}:{}",
                self.worker_command_listen, self.worker_command_port)
    }

    pub fn worker_heartbeat_addr(&self) -> String {
        format!("tcp://{}:{}",
                self.worker_heartbeat_listen, self.worker_heartbeat_port)
    }

    pub fn log_ingestion_addr(&self) -> String {
        format!("tcp://{}:{}",
                self.log_ingestion_listen, self.log_ingestion_port)
    }
}

impl Default for NetCfg {
    fn default() -> Self {
        NetCfg { worker_command_listen:   IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                 worker_command_port:     5566,
                 worker_heartbeat_listen: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                 worker_heartbeat_port:   5567,
                 log_ingestion_listen:    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                 log_ingestion_port:      5568, }
    }
}

pub trait GatewayCfg {
    /// Default number of worker threads to simultaneously handle HTTP requests.
    fn default_handler_count() -> usize { num_cpus::get() * 2 }

    /// Number of worker threads to simultaneously handle HTTP requests.
    fn handler_count(&self) -> usize { Self::default_handler_count() }

    fn listen_addr(&self) -> &IpAddr;

    fn listen_port(&self) -> u16;
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
    pub handler_count: usize,
    pub keep_alive:    usize,
}

impl Default for HttpCfg {
    fn default() -> Self {
        HttpCfg { listen:        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                  port:          5580,
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

////////////////////////////////////////////////////////////////////////
// Archive Configuration

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ArchiveCfg {
    pub backend: ArchiveBackend,

    // These are for S3 archiving
    pub key:      Option<String>,
    pub secret:   Option<String>,
    pub endpoint: Option<String>,
    pub bucket:   Option<String>,
    pub region:   String,

    // These are for local log archiving
    pub local_dir: Option<PathBuf>,
}

impl Default for ArchiveCfg {
    fn default() -> Self {
        ArchiveCfg { backend: ArchiveBackend::Local,

                     key:      None,
                     secret:   None,
                     endpoint: None,
                     bucket:   None,
                     region:   String::from("us-east-1"),

                     local_dir: None, }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn config_from_file() {
        let content = r#"
        build_targets = ["x86_64-linux"]
        features_enabled = "foo, bar"
        key_dir = "/path/to/keys"
        log_path = "/path/to/logs"
        job_timeout = 12345678

        [http]
        listen = "1.2.3.4"
        port   = 1234

        [net]
        worker_command_listen = "1:1:1:1:1:1:1:1"
        worker_command_port = 9000
        worker_heartbeat_listen = "1.1.1.1"
        worker_heartbeat_port = 9000
        log_ingestion_listen = "2.2.2.2"
        log_ingestion_port = 9999

        [archive]
        backend = "s3"
        key = "THIS_IS_THE_KEY"
        secret = "THIS_IS_THE_SECRET"
        bucket = "bukkit"
        endpoint = "http://minio.mycompany.com:9000"

        [datastore]
        host = "1.1.1.1"
        port = 9000
        user = "test"
        database = "test_jobsrv"
        connection_retry_ms = 500
        connection_timeout_sec = 4800
        connection_test = true
        pool_size = 1
        "#;

        let config = Config::from_raw(&content).unwrap();
        assert_eq!(&format!("{}", config.http.listen), "1.2.3.4");
        assert_eq!(config.http.port, 1234);
        assert_eq!(config.key_dir, PathBuf::from("/path/to/keys"));
        assert_eq!(config.log_path, PathBuf::from("/path/to/logs"));
        assert_eq!(config.job_timeout, 12345678);

        assert_eq!(&format!("{}", config.net.worker_command_listen),
                   "1:1:1:1:1:1:1:1");
        assert_eq!(&format!("{}", config.net.worker_heartbeat_listen),
                   "1.1.1.1");
        assert_eq!(&format!("{}", config.net.log_ingestion_listen), "2.2.2.2");

        assert_eq!(config.build_targets.len(), 1);
        assert!(config.build_targets.contains(&target::X86_64_LINUX));
        assert_eq!(config.features_enabled, "foo, bar");

        assert_eq!(config.net.worker_command_port, 9000);
        assert_eq!(config.net.worker_heartbeat_port, 9000);
        assert_eq!(config.net.log_ingestion_port, 9999);
        assert_eq!(config.datastore.port, 9000);
        assert_eq!(config.datastore.user, "test");
        assert_eq!(config.datastore.database, "test_jobsrv");
        assert_eq!(config.datastore.connection_retry_ms, 500);
        assert_eq!(config.datastore.connection_timeout_sec, 4800);
        assert_eq!(config.datastore.connection_test, true);
        assert_eq!(config.datastore.pool_size, 1);

        assert_eq!(config.archive.backend, ArchiveBackend::S3);
        assert_eq!(config.archive.key, Some("THIS_IS_THE_KEY".to_string()));
        assert_eq!(config.archive.secret,
                   Some("THIS_IS_THE_SECRET".to_string()));
        assert_eq!(config.archive.bucket, Some("bukkit".to_string()));
        assert_eq!(config.archive.endpoint,
                   Some("http://minio.mycompany.com:9000".to_string()));
        assert_eq!(config.archive.region, "us-east-1");
        assert_eq!(config.archive.local_dir, None);
    }
}
