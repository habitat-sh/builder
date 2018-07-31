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

use backend::s3;
use bldr_core::api_client::ApiClient;
use bldr_core::logger::Logger;
use hab_core::package::{Identifiable, PackageIdent, PackageTarget};
use hab_net::socket::DEFAULT_CONTEXT;
use http_gateway::conn::RouteBroker;
use http_gateway::http::helpers::all_visibilities;
use iron::typemap::Key;
use protobuf::{parse_from_bytes, Message};
use protocol::originsrv::*;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use zmq;

use config::Config;
use error::{Error, Result};
use protocol::originsrv::{OriginPackageIdent, UpstreamRequest};

use server::download_package_from_upstream_depot;

const UPSTREAM_MGR_ADDR: &'static str = "inproc://upstream";
const DEFAULT_POLL_TIMEOUT_MS: u64 = 60_000; // 60 secs

pub struct UpstreamClient;

pub struct UpstreamCli;

impl Key for UpstreamCli {
    type Value = UpstreamClient;
}

impl UpstreamClient {
    pub fn refresh(&self, ident: &OriginPackageIdent, target: &PackageTarget) -> Result<()> {
        let mut req = UpstreamRequest::new();
        req.set_ident(ident.clone());
        req.set_target(target.to_string());

        // TODO: Use a per-thread socket when we move to a post-Iron framework
        let socket = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        socket.connect(UPSTREAM_MGR_ADDR).map_err(Error::Zmq)?;
        socket
            .send(&req.write_to_bytes().unwrap(), 0)
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
    depot_client: Option<ApiClient>,
    s3_handler: s3::S3Handler,
    upstream_mgr_sock: zmq::Socket,
    want_origins: HashSet<String>,
    logger: Logger,
    msg: zmq::Message,
}

impl UpstreamMgr {
    pub fn new(cfg: &Config, s3_handler: s3::S3Handler) -> Result<Self> {
        let upstream_mgr_sock = (**DEFAULT_CONTEXT)
            .as_mut()
            .socket(zmq::DEALER)
            .map_err(Error::Zmq)?;

        let depot_client = if let Some(ref upstream_depot) = cfg.upstream_depot {
            Some(ApiClient::new(upstream_depot))
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
            s3_handler: s3_handler,
            upstream_mgr_sock: upstream_mgr_sock,
            want_origins: want_origins,
            logger: logger,
            msg: zmq::Message::new().map_err(Error::Zmq)?,
        })
    }

    pub fn start(cfg: &Config, s3_handler: s3::S3Handler) -> Result<JoinHandle<()>> {
        let mut manager = Self::new(cfg, s3_handler)?;
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
        let mut requests = VecDeque::new();

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

                let mut upstream_request: UpstreamRequest =
                    parse_from_bytes(&self.msg).map_err(Error::Protobuf)?;

                debug!("Upstream received message: {:?}", &upstream_request);

                // we have to assume ownership of these values here to appease the borrow checker
                // - otherwise it complains about immutable vs mutable borrows
                let msg_ident = upstream_request.take_ident();
                let target = upstream_request.take_target();

                // We only care about the base ident
                let ident =
                    PackageIdent::new(msg_ident.get_origin(), msg_ident.get_name(), None, None);
                upstream_request.set_ident(OriginPackageIdent::from(ident.clone()));
                upstream_request.set_target(target.clone());

                if self.config.upstream_depot.is_some()
                    && self.want_origins.contains(ident.origin())
                    && !requests.contains(&upstream_request)
                {
                    debug!("Adding {}-{} to work queue", &ident, &target);
                    requests.push_back(upstream_request.clone());
                }
            }

            // Handle potential work in requests queue
            let now = Instant::now();
            if &now > &(last_processed + Duration::from_millis(DEFAULT_POLL_TIMEOUT_MS)) {
                while let Some(upstream_request) = requests.pop_front() {
                    match self.check_request(&upstream_request) {
                        Ok(None) => (),
                        Ok(Some(ref ident)) => {
                            let msg = format!("UPDATED: {}", ident);
                            self.logger.log(&msg);
                        }
                        Err(err) => {
                            let msg = format!(
                                "FAILURE: {}-{} ({:?})",
                                upstream_request.get_ident(),
                                upstream_request.get_target(),
                                err
                            );
                            self.logger.log(&msg);
                        }
                    }
                }
                last_processed = now;
            }
        }
    }

    fn latest_ident(&mut self, ident: &OriginPackageIdent, target: &str) -> Result<PackageIdent> {
        let mut conn = RouteBroker::connect().unwrap();

        let mut request = OriginChannelPackageLatestGet::new();
        request.set_name("stable".to_owned());
        request.set_target(target.to_owned());
        request.set_visibilities(all_visibilities());
        request.set_ident(ident.clone());

        match conn.route::<OriginChannelPackageLatestGet, OriginPackageIdent>(&request) {
            Ok(id) => Ok(id.into()),
            Err(err) => Err(Error::NetError(err)),
        }
    }

    fn check_request(
        &mut self,
        upstream_request: &UpstreamRequest,
    ) -> Result<Option<PackageIdent>> {
        let ident = upstream_request.get_ident();
        let target = upstream_request.get_target();

        debug!("Checking upstream package: {}-{}", ident, target);
        assert!(!ident.fully_qualified());

        let local_ident = match self.latest_ident(ident, target) {
            Ok(i) => Some(i),
            Err(_) => None,
        };

        debug!("Latest local ident: {:?}", local_ident);

        match self.depot_client {
            // We only sync down stable packages from the upstream for now
            Some(ref depot_cli) => match depot_cli.show_package(ident, "stable", target, None) {
                Ok(mut package) => {
                    let remote_pkg_ident: PackageIdent = package.ident.into();

                    debug!("Got remote ident: {}", remote_pkg_ident);

                    if local_ident.is_none() || remote_pkg_ident > local_ident.unwrap() {
                        let opi: OriginPackageIdent =
                            OriginPackageIdent::from(remote_pkg_ident.clone());

                        debug!("Downloading package {:?} from upstream", opi);

                        if let Err(err) = download_package_from_upstream_depot(
                            &self.config,
                            depot_cli,
                            &self.s3_handler,
                            opi,
                            "stable",
                            &target,
                        ) {
                            warn!("Failed to download package from upstream, err {:?}", err);
                            return Err(err);
                        }
                        return Ok(Some(remote_pkg_ident));
                    }
                    Ok(None)
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
