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

use bldr_core::logger::Logger;
use hab_core::package::{Identifiable, PackageIdent};
use hab_net::socket::DEFAULT_CONTEXT;
use iron::typemap::Key;
use protobuf::{parse_from_bytes, Message};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use zmq;

use config::Config;
use error::{Error, Result};
use protocol::originsrv::OriginPackageIdent;

use depot_client::Client as DepotClient;
use server::download_package_from_upstream_depot;

const UPSTREAM_MGR_ADDR: &'static str = "inproc://upstream";
const DEFAULT_POLL_TIMEOUT_MS: u64 = 60_000; // 60 secs

pub struct UpstreamClient;

pub struct UpstreamCli;

impl Key for UpstreamCli {
    type Value = UpstreamClient;
}

impl UpstreamClient {
    pub fn refresh(&self, ident: &OriginPackageIdent) -> Result<()> {
        // TODO: Use a per-thread socket when we move to a post-Iron framework
        let socket = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        socket.connect(UPSTREAM_MGR_ADDR).map_err(Error::Zmq)?;
        socket
            .send(&ident.write_to_bytes().unwrap(), 0)
            .map_err(Error::Zmq)?;
        Ok(())
    }
}

impl Default for UpstreamClient {
    fn default() -> UpstreamClient {
        UpstreamClient {}
    }
}

pub struct UpstreamMgr {
    config: Config,
    depot_client: Option<DepotClient>,
    upstream_mgr_sock: zmq::Socket,
    want_origins: HashSet<String>,
    logger: Logger,
    msg: zmq::Message,
}

impl UpstreamMgr {
    pub fn new(cfg: &Config) -> Result<Self> {
        let upstream_mgr_sock = (**DEFAULT_CONTEXT)
            .as_mut()
            .socket(zmq::DEALER)
            .map_err(Error::Zmq)?;

        let depot_client = if let Some(ref upstream_depot) = cfg.upstream_depot {
            Some(DepotClient::new(upstream_depot, "builder-upstream", "0.0.0", None).unwrap())
        } else {
            None
        };

        let log_path = cfg.log_dir.clone();
        let mut logger = Logger::init(PathBuf::from(log_path), "builder-upstream.log");

        let want_origins: HashSet<String> =
            cfg.upstream_origins.iter().map(|s| s.to_owned()).collect();

        let msg = format!(
            "UPSTREAM {:?} (origins: {:?})",
            cfg.upstream_depot, cfg.upstream_origins
        );
        logger.log_ident(&msg);

        Ok(UpstreamMgr {
            config: cfg.clone(),
            depot_client: depot_client,
            upstream_mgr_sock: upstream_mgr_sock,
            want_origins: want_origins,
            logger: logger,
            msg: zmq::Message::new().map_err(Error::Zmq)?,
        })
    }

    pub fn start(cfg: &Config) -> Result<JoinHandle<()>> {
        let mut manager = Self::new(cfg)?;
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = thread::Builder::new()
            .name("upstream-manager".to_string())
            .spawn(move || {
                manager.run(tx).unwrap();
            })
            .unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("upstream-manager thread startup error, err={}", e),
        }
    }

    fn run(&mut self, rz: mpsc::SyncSender<()>) -> Result<()> {
        self.upstream_mgr_sock
            .bind(UPSTREAM_MGR_ADDR)
            .map_err(Error::Zmq)?;
        let mut upstream_mgr_sock = false;
        let mut last_processed = Instant::now();
        let mut idents = VecDeque::new();

        rz.send(()).unwrap();

        info!(
            "upstream-manager is ready to go (upstream_depot: {:?}).",
            self.config.upstream_depot
        );

        loop {
            {
                let mut items = [self.upstream_mgr_sock.as_poll_item(1)];

                if let Err(err) = zmq::poll(&mut items, DEFAULT_POLL_TIMEOUT_MS as i64) {
                    warn!(
                        "Upstream-manager unable to complete ZMQ poll: err {:?}",
                        err
                    );
                };
                if (items[0].get_revents() & zmq::POLLIN) > 0 {
                    upstream_mgr_sock = true;
                }
            }

            if upstream_mgr_sock {
                upstream_mgr_sock = false;

                if let Err(err) = self.upstream_mgr_sock.recv(&mut self.msg, 0) {
                    warn!(
                        "Upstream-manager unable to complete socket receive: err {:?}",
                        err
                    );
                    continue;
                }

                let ident: OriginPackageIdent =
                    parse_from_bytes(&self.msg).map_err(Error::Protobuf)?;

                debug!("Upstream received message: {:?}", ident);

                if self.config.upstream_depot.is_some()
                    && self.want_origins.contains(ident.get_origin())
                    && !idents.contains(&ident)
                {
                    debug!("Adding {} to work queue", &ident);
                    idents.push_back(ident);
                }
            }

            // Handle potential work in idents queue
            let now = Instant::now();
            if &now > &(last_processed + Duration::from_millis(DEFAULT_POLL_TIMEOUT_MS)) {
                while let Some(ident) = idents.pop_front() {
                    match self.check_package(&ident) {
                        Ok(None) => (),
                        Ok(Some(ref ident)) => {
                            let msg = format!("UPDATED: {}", ident);
                            self.logger.log(&msg);
                        }
                        Err(err) => {
                            let msg = format!("FAILURE: {} ({:?})", ident, err);
                            self.logger.log(&msg);
                        }
                    }
                }
                last_processed = now;
            }
        }
    }

    fn check_package(&mut self, ident: &OriginPackageIdent) -> Result<Option<PackageIdent>> {
        debug!("Checking upstream package: {}", ident);

        match self.depot_client {
            // We only sync down stable packages from the upstream for now
            Some(ref depot_cli) => match depot_cli.show_package(ident, Some("stable"), None) {
                Ok(mut package) => {
                    let pkg_ident: PackageIdent = ident.clone().into();
                    let remote_pkg_ident: PackageIdent = package.take_ident().into();

                    debug!("Got remote ident: {}", remote_pkg_ident);

                    if !ident.fully_qualified() || remote_pkg_ident > pkg_ident {
                        let opi: OriginPackageIdent =
                            OriginPackageIdent::from(remote_pkg_ident.clone());

                        debug!("Downloading package {:?} from upstream", opi);

                        if let Err(err) = download_package_from_upstream_depot(
                            &self.config,
                            depot_cli,
                            opi,
                            Some("stable".to_string()),
                        ) {
                            warn!("Failed to download package from upstream, err {:?}", err);
                            return Err(err);
                        }
                    }

                    Ok(Some(remote_pkg_ident))
                }
                Err(err) => {
                    warn!(
                        "Failed to get package metadata for {} from {:?}, err {:?}",
                        ident, self.config.upstream_depot, err
                    );
                    Err(Error::DepotClientError(err))
                }
            },
            _ => Ok(None),
        }
    }
}
