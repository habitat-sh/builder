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

use std::{collections::HashMap,
          iter::FromIterator,
          panic,
          sync::Arc,
          thread,
          time::Duration};

use zmq;

use crate::{bldr_core::{self,
                        socket::DEFAULT_CONTEXT},
            protocol::{jobsrv,
                       message}};

use crate::{config::Config,
            error::Result,
            feat,
            heartbeat::{HeartbeatCli,
                        HeartbeatMgr},
            log_forwarder::LogForwarder,
            runner::{RunnerCli,
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
        // Set custom panic hook - a panic on the runner thread will
        // cause the builder-worker process to exit (and be re-started
        // by the supervisor when running under hab)
        panic::set_hook(Box::new(|panic_info| {
                            let backtrace = backtrace::Backtrace::new();
                            println!("panic info: {:?}", panic_info);
                            println!("{:?}", backtrace);
                            println!("Exiting builder-worker process");
                            std::process::exit(1)
                        }));

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
