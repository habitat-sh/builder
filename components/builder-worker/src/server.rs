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

#[cfg(windows)]
use std::path::PathBuf;
use std::{collections::HashMap,
          fs,
          iter::FromIterator,
          path::Path,
          sync::Arc,
          thread,
          time::Duration};

use zmq;

#[cfg(windows)]
use crate::hab_core::env;
#[cfg(not(windows))]
use crate::hab_core::util::posix_perm;
use crate::{bldr_core::{self,
                        socket::DEFAULT_CONTEXT},
            hab_core::users,
            protocol::{jobsrv,
                       message}};

use crate::{config::Config,
            error::{Error,
                    Result},
            feat,
            heartbeat::{HeartbeatCli,
                        HeartbeatMgr},
            log_forwarder::LogForwarder,
            network::NetworkNamespace,
            runner::{studio,
                     RunnerCli,
                     RunnerMgr}};

/// Interval for main thread to check cancel status
pub const BUILD_CANCEL_WAIT_SECS: u64 = 15;

enum State {
    Ready,
    Busy,
}

impl Default for State {
    fn default() -> State { State::Ready }
}

pub struct Server {
    config: Arc<Config>,
    /// Dealer Socket connected to JobSrv
    fe_sock: zmq::Socket,
    hb_cli: HeartbeatCli,
    runner_cli: RunnerCli,
    state: State,
    msg: zmq::Message,
    net_ident: Arc<String>,
}

impl Server {
    pub fn new(config: Config) -> Self {
        let net_ident = bldr_core::socket::srv_ident();
        let fe_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        let hb_cli = HeartbeatCli::new(net_ident.clone(), config.target.to_string());
        let runner_cli = RunnerCli::new();
        fe_sock.set_identity(net_ident.as_bytes()).unwrap();
        Server { config: Arc::new(config),
                 fe_sock,
                 hb_cli,
                 runner_cli,
                 state: State::default(),
                 msg: zmq::Message::new().unwrap(),
                 net_ident: Arc::new(net_ident) }
    }

    pub fn run(&mut self) -> Result<()> {
        init_users()?;
        self.setup_networking()?;
        self.enable_features_from_config();

        HeartbeatMgr::start(&self.config, (&*self.net_ident).clone())?;
        RunnerMgr::start(self.config.clone(), self.net_ident.clone())?;
        LogForwarder::start(&self.config)?;
        self.hb_cli.connect()?;
        self.runner_cli.connect()?;
        for (_, queue, _) in self.config.jobsrv_addrs() {
            println!("Connecting to job queue, {}", queue);
            self.fe_sock.connect(&queue)?;
        }

        let mut fe_msg = false;
        let mut runner_msg = false;
        info!("builder-worker is ready to go.");
        loop {
            {
                let mut items = [self.fe_sock.as_poll_item(1),
                                 self.runner_cli.as_poll_item(1)];
                zmq::poll(&mut items, -1)?;
                if items[0].get_revents() & zmq::POLLIN > 0 {
                    fe_msg = true;
                }
                if items[1].get_revents() & zmq::POLLIN > 0 {
                    runner_msg = true;
                }
            }
            if runner_msg {
                {
                    let reply = self.runner_cli.recv_complete()?;
                    self.fe_sock.send(reply, 0)?;
                }
                self.set_ready()?;
                runner_msg = false;
            }
            if fe_msg {
                self.fe_sock.recv(&mut self.msg, 0)?; // Receive empty msg
                self.fe_sock.recv(&mut self.msg, 0)?; // Receive Command msg

                let wc = message::decode::<jobsrv::WorkerCommand>(&self.msg)?;
                self.fe_sock.recv(&mut self.msg, 0)?; // Receive Job msg

                match self.state {
                    State::Ready => {
                        match wc.get_op() {
                            jobsrv::WorkerOperation::StartJob => self.start_job()?,
                            jobsrv::WorkerOperation::CancelJob => {
                                warn!("Received unexpected Cancel for Ready worker")
                            }
                        }
                    }
                    State::Busy => {
                        match wc.get_op() {
                            jobsrv::WorkerOperation::StartJob => self.reject_job()?,
                            jobsrv::WorkerOperation::CancelJob => self.cancel_job()?,
                        }
                    }
                }
                fe_msg = false;
            }
        }
    }

    fn start_job(&mut self) -> Result<()> {
        self.runner_cli.start_job(&self.msg)?;
        {
            let reply = self.runner_cli.recv_ack()?;
            self.fe_sock.send(reply, 0)?;
        }
        self.set_busy()?;
        Ok(())
    }

    fn cancel_job(&mut self) -> Result<()> {
        self.runner_cli.cancel_job(&self.msg)?;
        thread::sleep(Duration::new(BUILD_CANCEL_WAIT_SECS, 0));
        {
            let reply = self.runner_cli.recv_ack()?;
            self.fe_sock.send(reply, 0)?;
        }
        self.set_ready()?;
        Ok(())
    }

    fn reject_job(&mut self) -> Result<()> {
        let mut reply = message::decode::<jobsrv::Job>(&self.msg)?;
        reply.set_state(jobsrv::JobState::Rejected);
        self.fe_sock.send(&message::encode(&reply)?, 0)?;
        Ok(())
    }

    fn set_busy(&mut self) -> Result<()> {
        self.hb_cli.set_busy()?;
        self.state = State::Busy;
        Ok(())
    }

    fn set_ready(&mut self) -> Result<()> {
        self.hb_cli.set_ready()?;
        self.state = State::Ready;
        Ok(())
    }

    fn setup_networking(&self) -> Result<()> {
        // Skip if networking details are not specified
        if self.config.network_interface.is_none() && self.config.network_gateway.is_none() {
            info!("airlock networking is not configured, skipping network creation");
            return Ok(());
        }
        if self.config.network_interface.is_some() && self.config.network_gateway.is_none() {
            error!("ERROR: No 'network_gateway' config value specfied when 'network_interface' \
                    was provided. Both must be present to work correctly.");
            return Err(Error::NoNetworkGatewayError);
        }
        if self.config.network_gateway.is_some() && self.config.network_interface.is_none() {
            error!("ERROR: No 'network_interface' config value specfied when 'network_gateway' \
                    was provided. Both must be present to work correctly.");
            return Err(Error::NoNetworkInterfaceError);
        }

        let net_ns = NetworkNamespace::new(self.config.ns_dir_path());
        if net_ns.exists() {
            if self.config.recreate_ns_dir {
                // If a network namespace appears to be setup and the recreate config is true, then
                // we should destroy this namespace so it can be created again below
                net_ns.destroy()?;
            } else {
                info!("reusing network namespace, dir={}",
                      net_ns.ns_dir().display());
                return Ok(());
            }
        }

        let interface = self.config
                            .network_interface
                            .as_ref()
                            .expect("network_interface is set");
        let gateway = self.config
                          .network_gateway
                          .as_ref()
                          .expect("network_gateway is set");
        self.prepare_dirs()?;
        net_ns.create(interface, gateway, studio::STUDIO_USER)
    }

    fn prepare_dirs(&self) -> Result<()> {
        // Ensure that data path group ownership is set to the build user and directory perms are
        // `0750`. This allows the namespace files to be accessed and read by the build user
        if cfg!(not(windows)) {
            set_owner(&self.config.data_path,
                      users::get_current_username().unwrap_or_else(|| String::from("root"))
                                                   .as_str(),
                      studio::STUDIO_GROUP)?;
            set_permissions(&self.config.data_path, 0o750)?;
        } else {
            unreachable!();
        }

        // Set parent directory of ns_dir to be owned by the build user so that the appropriate
        // directories, files, and bind-mounts can be created for the build user
        let parent_path = self.config.ns_dir_path();
        let parent_path =
            parent_path.parent()
                       .expect("parent directory path segement for ns_dir should exist");
        if !parent_path.is_dir() {
            fs::create_dir_all(parent_path).map_err(|e| {
                                               Error::CreateDirectory(parent_path.to_path_buf(), e)
                                           })?;
        }
        if cfg!(not(windows)) {
            set_owner(&parent_path, studio::STUDIO_USER, studio::STUDIO_GROUP)?;
            set_permissions(&parent_path, 0o750)?;
        } else {
            unreachable!();
        }

        Ok(())
    }

    fn enable_features_from_config(&self) {
        let features: HashMap<_, _> = HashMap::from_iter(vec![("LIST", feat::List)]);
        let features_enabled = self.config
                                   .features_enabled
                                   .split(',')
                                   .map(|f| f.trim().to_uppercase());
        for key in features_enabled {
            if features.contains_key(key.as_str()) {
                info!("Enabling feature: {}", key);
                feat::enable(features[key.as_str()]);
            }
        }

        if feat::is_enabled(feat::List) {
            println!("Listing possible feature flags: {:?}", features.keys());
            println!("Enable features by populating 'features_enabled' in config");
        }
    }
}

pub fn run(config: Config) -> Result<()> { Server::new(config).run() }

#[cfg(not(windows))]
fn init_users() -> Result<()> {
    let uid = users::get_uid_by_name(studio::STUDIO_USER).ok_or(Error::NoStudioUser)?;
    let gid = users::get_gid_by_name(studio::STUDIO_GROUP).ok_or(Error::NoStudioGroup)?;
    let mut home = studio::STUDIO_HOME.lock().unwrap();
    *home = users::get_home_for_user(studio::STUDIO_USER).ok_or(Error::NoStudioGroup)?;
    debug!("Setting STUDIO_HOME to {:?}", *home);
    studio::set_studio_uid(uid);
    studio::set_studio_gid(gid);
    Ok(())
}

#[cfg(windows)]
fn init_users() -> Result<()> {
    let mut home = studio::STUDIO_HOME.lock().unwrap();
    if let Some(val) = env::var_os("HOME") {
        debug!("Setting STUDIO_HOME to {:?}", val);
        *home = PathBuf::from(val);
    } else {
        debug!("HOME env var not found!");
        debug!("Setting STUDIO_HOME to C:\\hab");
        *home = PathBuf::from("C:\\hab");
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_owner<T: AsRef<Path>, X: AsRef<str>>(path: T, owner: X, group: X) -> Result<()> {
    posix_perm::set_owner(path, owner, group).map_err(Error::HabitatCore)
}

#[cfg(windows)]
pub fn set_owner<T: AsRef<Path>, X: AsRef<str>>(_path: T, _owner: X, _group: X) -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
pub fn set_permissions<T: AsRef<Path>>(path: T, mode: u32) -> Result<()> {
    posix_perm::set_permissions(&path, mode).map_err(Error::HabitatCore)
}

#[cfg(windows)]
pub fn set_permissions<T: AsRef<Path>>(_path: T, _mode: u32) -> Result<()> { Ok(()) }
