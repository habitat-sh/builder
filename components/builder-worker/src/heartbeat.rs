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

use std::{sync::mpsc,
          thread::{self,
                   JoinHandle},
          time::Duration};

use crate::{bldr_core::socket::DEFAULT_CONTEXT,
            protocol::{jobsrv as proto,
                       message}};

use crate::{config::Config,
            error::Result};

/// Polling timeout for HeartbeatMgr
pub const HEARTBEAT_MS: i64 = 30_000;
/// In-memory zmq address for HeartbeatMgr
const INPROC_ADDR: &str = "inproc://heartbeat";
/// Protocol message to notify the HeartbeatMgr to begin pulsing
const CMD_PULSE: &str = "R";
/// Protocol message to notify the HeartbeatMgr to pause pulsing
const CMD_PAUSE: &str = "P";

#[cfg(target_os = "linux")]
fn worker_os() -> proto::Os { proto::Os::Linux }

#[cfg(target_os = "windows")]
fn worker_os() -> proto::Os { proto::Os::Windows }

#[cfg(target_os = "macos")]
fn worker_os() -> proto::Os { proto::Os::Darwin }

#[derive(PartialEq)]
#[derive(Default)]
enum PulseState {
    Pause,
    #[default]
    Pulse,
}

impl AsRef<str> for PulseState {
    fn as_ref(&self) -> &str {
        match *self {
            PulseState::Pause => CMD_PAUSE,
            PulseState::Pulse => CMD_PULSE,
        }
    }
}

/// Client for sending and receiving messages to and from the HeartbeatMgr
pub struct HeartbeatCli {
    msg:   zmq::Message,
    sock:  zmq::Socket,
    state: proto::Heartbeat,
}

impl HeartbeatCli {
    /// Create a new HeartbeatMgr client
    pub fn new(net_ident: String, target: String) -> Self {
        let sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::REQ).unwrap();
        let mut state = proto::Heartbeat::new();
        state.set_endpoint(net_ident);
        state.set_os(worker_os());
        state.set_target(target);
        HeartbeatCli { msg: zmq::Message::new(),
                       sock,
                       state }
    }

    /// Connect to the `HeartbeatMgr`
    pub fn connect(&mut self) -> Result<()> {
        self.sock.connect(INPROC_ADDR)?;
        Ok(())
    }

    /// Set the `HeartbeatMgr` state to busy
    pub fn set_busy(&mut self) -> Result<()> {
        self.state.set_state(proto::WorkerState::Busy);
        self.sock.send(PulseState::Pulse.as_ref(), zmq::SNDMORE)?;
        self.sock.send(message::encode(&self.state)?, 0)?;
        self.sock.recv(&mut self.msg, 0)?;
        Ok(())
    }

    /// Set the `HeartbeatMgr` state to ready
    pub fn set_ready(&mut self) -> Result<()> {
        self.state.set_state(proto::WorkerState::Ready);
        self.sock.send(PulseState::Pulse.as_ref(), zmq::SNDMORE)?;
        self.sock.send(message::encode(&self.state)?, 0)?;
        self.sock.recv(&mut self.msg, 0)?;
        Ok(())
    }

    /// Pause the heartbeats until next state is set
    pub fn pause(&mut self) -> Result<()> {
        self.sock.send(PulseState::Pause.as_ref(), 0)?;
        self.sock.recv(&mut self.msg, 0)?;
        Ok(())
    }
}

/// Maintains and broadcasts health and state of the Worker server to consumers
pub struct HeartbeatMgr {
    /// Internal socket for sending and receiving message to and from a `HeartbeatCli`
    pub cli_sock: zmq::Socket,
    /// Public socket for publishing worker state to consumers
    pub pub_sock: zmq::Socket,
    heartbeat:    proto::Heartbeat,
    msg:          zmq::Message,
    state:        PulseState,
}

impl HeartbeatMgr {
    /// Start the HeartbeatMgr
    pub fn start(config: &Config, net_ident: String) -> Result<JoinHandle<()>> {
        let (tx, rx) = mpsc::sync_channel(0);
        let mut heartbeat = Self::new(net_ident, config.target.to_string());
        let jobsrv_addrs = config.jobsrv_addrs();
        let handle = thread::Builder::new().name("heartbeat".to_string())
                                           .spawn(move || {
                                               heartbeat.run(&tx, jobsrv_addrs).unwrap();
                                           })
                                           .unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("heartbeat thread startup error, err={}", e),
        }
    }

    fn new(net_ident: String, target: String) -> Self {
        let pub_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::PUB).unwrap();
        let cli_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::REP).unwrap();
        pub_sock.set_immediate(true).unwrap();
        pub_sock.set_sndhwm(1).unwrap();
        pub_sock.set_linger(0).unwrap();
        let mut heartbeat = proto::Heartbeat::new();
        heartbeat.set_endpoint(net_ident);
        heartbeat.set_os(worker_os());
        heartbeat.set_state(proto::WorkerState::Ready);
        heartbeat.set_target(target);
        HeartbeatMgr { state: PulseState::default(),
                       pub_sock,
                       cli_sock,
                       heartbeat,
                       msg: zmq::Message::new() }
    }

    // Main loop for server
    fn run(&mut self,
           rz: &mpsc::SyncSender<()>,
           jobsrv_addrs: Vec<(String, String, String)>)
           -> Result<()> {
        for (hb, ..) in jobsrv_addrs {
            println!("Connecting to heartbeat, {}", hb);
            self.pub_sock.connect(&hb)?;
        }
        self.cli_sock.bind(INPROC_ADDR)?;
        rz.send(()).unwrap();
        // This hacky sleep is recommended and required by zmq for connections to establish
        thread::sleep(Duration::from_millis(100));
        let mut cli_sock_msg = false;
        loop {
            if self.state == PulseState::Pulse {
                self.pulse()?;
            }
            {
                let mut items = [self.cli_sock.as_poll_item(zmq::POLLIN)];
                // Poll until timeout or message is received. Checking for the zmq::POLLIN flag on
                // a poll item's revents will let you know if you have received a message or not
                // on that socket.
                zmq::poll(&mut items, HEARTBEAT_MS)?;
                if items[0].is_readable() {
                    cli_sock_msg = true;
                }
            }
            if cli_sock_msg {
                self.recv_cmd()?;
                cli_sock_msg = false;
            }
        }
    }

    // Set internal state to `PulseState::Pause` and notify client OK
    fn pause(&mut self) {
        debug!("heartbeat paused");
        self.state = PulseState::Pause;
        self.cli_sock.send("", 0).unwrap();
    }

    // Broadcast to subscribers the HeartbeatMgr health and state
    fn pulse(&mut self) -> Result<()> {
        trace!("heartbeat pulsed: {:?}", self.heartbeat);
        self.pub_sock.send(message::encode(&self.heartbeat)?, 0)?;
        Ok(())
    }

    // Wait receive for a command from a client
    fn recv_cmd(&mut self) -> Result<()> {
        self.cli_sock.recv(&mut self.msg, 0)?;
        match self.msg.as_str() {
            Some(CMD_PAUSE) => {
                self.pause();
                return Ok(());
            }
            Some(CMD_PULSE) => (),
            _ => unreachable!("wk:hb:1, received unexpected message from client"),
        }
        self.cli_sock.recv(&mut self.msg, 0)?;
        self.heartbeat = message::decode(&self.msg)?;
        debug!("heartbeat received by mgr: {:?}", self.heartbeat);
        self.resume();
        Ok(())
    }

    // Set internal state to `PulseState::Pulse` and notify client OK
    fn resume(&mut self) {
        debug!("heartbeat resumed");
        self.state = PulseState::Pulse;
        self.cli_sock.send("", 0).unwrap();
    }
}
