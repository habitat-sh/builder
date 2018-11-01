// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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

mod docker;
mod job_streamer;
mod postprocessor;
mod publisher;
pub mod studio;
mod toml_builder;
mod util;
mod workspace;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};

use bldr_core;
use bldr_core::api_client::ApiClient;
use bldr_core::job::Job;
use bldr_core::logger::Logger;
use chrono::Utc;
use hab_core::os::users;
use hab_core::package::archive::PackageArchive;
use hab_core::util::posix_perm;
use hab_net::socket::DEFAULT_CONTEXT;
pub use protocol::jobsrv::JobState;
use protocol::net::{self, ErrCode};
use protocol::originsrv::OriginPackageIdent;
use protocol::{jobsrv, message};
use zmq;

use self::docker::DockerExporter;
use self::job_streamer::{JobStreamer, Section};
use self::postprocessor::post_process;
use self::studio::{key_path, Studio, STUDIO_GROUP, STUDIO_USER};
use self::workspace::Workspace;
use config::Config;
use error::{Error, Result};
use network::NetworkNamespace;
use retry::retry;
use vcs::VCS;

// TODO fn: copied from `components/common/src/ui.rs`. As this component doesn't currently depend
// on habitat_common it didnt' seem worth it to add a dependency for only this constant. Probably
// means that the constant should be relocated to habitat_core.
/// Environment variable to disable progress bars in Habitat programs
const NONINTERACTIVE_ENVVAR: &'static str = "HAB_NONINTERACTIVE";

/// Environment variable to enable or disable debug output in runner's studio
const RUNNER_DEBUG_ENVVAR: &'static str = "BUILDER_RUNNER_DEBUG";
/// Environment variable to enable or disable dev mode.
const DEV_MODE: &'static str = "DEV_MODE";
/// In-memory zmq address of Job RunnerMgr
const INPROC_ADDR: &'static str = "inproc://runner";
/// Protocol message to indicate the Job Runner has received a work request
const WORK_ACK: &'static str = "A";
/// Protocol message to indicate the Job Runner has completed a work request
const WORK_COMPLETE: &'static str = "C";
/// Protocol message to indicate the Runner Cli is sending a work request
const WORK_START: &'static str = "S";
/// Protocol message to indicate the Runner Cli is sending a cancel request
const WORK_CANCEL: &'static str = "X";

pub const RETRIES: u64 = 10;
pub const RETRY_WAIT: u64 = 60000;

pub struct Runner {
    config: Arc<Config>,
    depot_cli: ApiClient,
    workspace: Workspace,
    logger: Logger,
    bldr_token: String,
    cancel: Arc<AtomicBool>,
}

impl Runner {
    pub fn new(job: Job, config: Arc<Config>, net_ident: &str, cancel: Arc<AtomicBool>) -> Self {
        let depot_cli = ApiClient::new(&config.bldr_url);

        let log_path = config.log_path.clone();
        let mut logger = Logger::init(PathBuf::from(log_path), "builder-worker.log");
        logger.log_ident(net_ident);
        let bldr_token = bldr_core::access_token::generate_bldr_token(&config.key_dir).unwrap();

        Runner {
            workspace: Workspace::new(&config.data_path, job),
            config: config,
            depot_cli: depot_cli,
            logger: logger,
            bldr_token: bldr_token,
            cancel: cancel,
        }
    }

    pub fn job(&self) -> &Job {
        &self.workspace.job
    }

    pub fn job_mut(&mut self) -> &mut Job {
        &mut self.workspace.job
    }

    fn is_canceled(&mut self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    fn check_cancel(&mut self, tx: &mpsc::Sender<Job>) -> Result<()> {
        if self.is_canceled() {
            debug!("Runner canceling job id: {}", self.job().get_id());
            self.cancel();
            self.cleanup();
            tx.send(self.job().clone()).map_err(Error::Mpsc)?;
            return Err(Error::JobCanceled);
        }

        Ok(())
    }

    fn do_validate(&mut self, tx: &mpsc::Sender<Job>, streamer: &mut JobStreamer) -> Result<()> {
        self.check_cancel(tx)?;

        let mut section = streamer.start_section(Section::ValidateIntegrations)?;

        if let Some(err) = util::validate_integrations(&self.workspace).err() {
            let msg = format!(
                "Failed to validate integrations for {}, err={:?}",
                self.workspace.job.get_project().get_name(),
                err
            );
            debug!("{}", msg);
            self.logger.log(&msg);

            streamer.println_stderr(msg)?;
            self.fail(net::err(ErrCode::INVALID_INTEGRATIONS, "wk:run:validate"));
            tx.send(self.job().clone()).map_err(Error::Mpsc)?;
            return Err(err);
        };

        section.end()?;
        Ok(())
    }

    fn do_setup(&mut self, tx: &mpsc::Sender<Job>) -> Result<JobStreamer> {
        self.check_cancel(tx)?;

        let streamer = match self.setup() {
            Ok(streamer) => streamer,
            Err(err) => {
                let msg = format!(
                    "Failed to setup workspace for {}, err={:?}",
                    self.workspace.job.get_project().get_name(),
                    err
                );
                warn!("{}", msg);
                self.logger.log(&msg);

                self.fail(net::err(ErrCode::WORKSPACE_SETUP, "wk:run:workspace"));
                tx.send(self.job().clone()).map_err(Error::Mpsc)?;
                return Err(err);
            }
        };

        Ok(streamer)
    }

    fn do_install_key(&mut self, tx: &mpsc::Sender<Job>, streamer: &mut JobStreamer) -> Result<()> {
        self.check_cancel(tx)?;

        let mut section = streamer.start_section(Section::FetchOriginKey)?;

        if let Some(err) = self.install_origin_secret_key().err() {
            let msg = format!(
                "Failed to install origin secret key {}, err={:?}",
                self.workspace.job.get_project().get_origin_name(),
                err
            );
            debug!("{}", msg);
            self.logger.log(&msg);

            streamer.println_stderr(msg)?;
            self.fail(net::err(ErrCode::SECRET_KEY_FETCH, "wk:run:key"));
            tx.send(self.job().clone()).map_err(Error::Mpsc)?;
            return Err(err);
        }

        section.end()?;
        Ok(())
    }

    fn do_clone(&mut self, tx: &mpsc::Sender<Job>, streamer: &mut JobStreamer) -> Result<()> {
        self.check_cancel(tx)?;
        let mut section = streamer.start_section(Section::CloneRepository)?;

        let vcs = VCS::from_job(&self.job(), self.config.github.clone());
        if let Some(err) = vcs.clone(&self.workspace.src()).err() {
            let msg = format!(
                "Failed to clone remote source repository for {}, err={:?}",
                self.workspace.job.get_project().get_name(),
                err
            );
            warn!("{}", msg);
            self.logger.log(&msg);

            streamer.println_stderr(msg)?;
            self.fail(net::err(ErrCode::VCS_CLONE, "wk:run:clone:1"));
            tx.send(self.job().clone()).map_err(Error::Mpsc)?;
            return Err(err);
        }
        if let Some(err) = util::chown_recursive(
            self.workspace.src(),
            studio::studio_uid(),
            studio::studio_gid(),
        ).err()
        {
            let msg = format!(
                "Failed to change ownership of source repository for {}, err={:?}",
                self.workspace.job.get_project().get_name(),
                err
            );
            debug!("{}", msg);
            self.logger.log(&msg);

            streamer.println_stderr(msg)?;
            self.fail(net::err(ErrCode::VCS_CLONE, "wk:run:clone:2"));
            tx.send(self.job().clone()).map_err(Error::Mpsc)?;
            return Err(err);
        }

        section.end()?;
        Ok(())
    }

    fn do_build(
        &mut self,
        tx: &mpsc::Sender<Job>,
        streamer: &mut JobStreamer,
    ) -> Result<PackageArchive> {
        self.check_cancel(tx)?;

        self.workspace
            .job
            .set_build_started_at(Utc::now().to_rfc3339());

        let mut section = streamer.start_section(Section::BuildPackage)?;

        // TODO: We don't actually update the state of the job to
        // "Processing" (that should happen here), so an outside
        // observer will see a job up going from "Dispatched" directly
        // to "Complete" (or "Failed", etc.). As a result, we won't
        // get the `build_started_at` time set until the job is actually
        // finished.
        let mut archive = match self.build(streamer) {
            Ok(archive) => {
                self.workspace
                    .job
                    .set_build_finished_at(Utc::now().to_rfc3339());
                archive
            }
            Err(err) => {
                self.workspace
                    .job
                    .set_build_finished_at(Utc::now().to_rfc3339());
                let msg = format!(
                    "Failed studio build for {}, err={:?}",
                    self.workspace.job.get_project().get_name(),
                    err
                );
                debug!("{}", msg);
                self.logger.log(&msg);
                streamer.println_stderr(msg)?;

                self.fail(net::err(ErrCode::BUILD, "wk:run:build"));
                tx.send(self.job().clone()).map_err(Error::Mpsc)?;
                return Err(err);
            }
        };

        // Converting from a core::PackageIdent to an OriginPackageIdent
        let ident = OriginPackageIdent::from(archive.ident().unwrap());
        self.workspace.job.set_package_ident(ident);

        section.end()?;
        Ok(archive)
    }

    fn do_export(&mut self, tx: &mpsc::Sender<Job>, mut streamer: &mut JobStreamer) -> Result<()> {
        self.check_cancel(tx)?;

        match self.export(&mut streamer) {
            Ok(_) => (),
            Err(err) => {
                self.fail(net::err(ErrCode::EXPORT, "wk:run:export"));
                tx.send(self.job().clone()).map_err(Error::Mpsc)?;
                return Err(err);
            }
        }

        Ok(())
    }

    fn do_postprocess(
        &mut self,
        tx: &mpsc::Sender<Job>,
        mut archive: PackageArchive,
        streamer: &mut JobStreamer,
    ) -> Result<()> {
        self.check_cancel(tx)?;
        let mut section = streamer.start_section(Section::PublishPackage)?;

        match post_process(
            &mut archive,
            &self.workspace,
            &self.config,
            &self.bldr_token,
            &mut self.logger,
        ) {
            Ok(_) => (),
            Err(err) => {
                let msg = format!(
                    "Failed post processing for {}, err={:?}",
                    self.workspace.job.get_project().get_name(),
                    err
                );
                streamer.println_stderr(msg)?;
                self.fail(net::err(ErrCode::POST_PROCESSOR, "wk:run:postprocess"));
                tx.send(self.job().clone()).map_err(Error::Mpsc)?;
                return Err(err);
            }
        }

        section.end()?;
        Ok(())
    }

    fn cleanup(&mut self) {
        if let Some(err) = fs::remove_dir_all(self.workspace.out()).err() {
            warn!(
                "Failed to delete directory during cleanup, dir={}, err={:?}",
                self.workspace.out().display(),
                err
            )
        }
        self.teardown();
    }

    pub fn run(mut self, tx: mpsc::Sender<Job>) -> Result<()> {
        // TBD (SA) - Spin up a LogStreamer first thing indpendendly of setup.
        // Currently we need to do it as part of setup because the log file to be
        // streamed lives inside of the workspace, which is created by setup.
        let mut streamer = self.do_setup(&tx)?;

        self.do_validate(&tx, &mut streamer)?;
        self.do_install_key(&tx, &mut streamer)?;
        self.do_clone(&tx, &mut streamer)?;

        let archive = self.do_build(&tx, &mut streamer)?;
        self.do_export(&tx, &mut streamer)?;
        self.do_postprocess(&tx, archive, &mut streamer)?;

        self.cleanup();
        self.complete();
        tx.send(self.workspace.job).map_err(Error::Mpsc)?;

        streamer.finish()?;

        Ok(())
    }

    fn install_origin_secret_key(&mut self) -> Result<()> {
        match retry(
            RETRIES,
            RETRY_WAIT,
            || {
                self.depot_cli.fetch_origin_secret_key(
                    self.job().origin(),
                    &self.bldr_token,
                    key_path(),
                )
            },
            |res| {
                if res.is_err() {
                    debug!("Failed to fetch origin secret key, err={:?}", res);
                };
                res.is_ok()
            },
        ) {
            Ok(res) => {
                let dst = res.unwrap();
                debug!("Imported origin secret key, dst={:?}.", dst);
                if self.config.airlock_enabled {
                    if cfg!(not(windows)) {
                        posix_perm::set_owner(dst, STUDIO_USER, STUDIO_GROUP)?;
                    } else {
                        unreachable!();
                    }
                }
                Ok(())
            }
            Err(err) => {
                let msg = format!(
                    "Failed to import secret key {} after {} retries",
                    self.job().origin(),
                    RETRIES
                );
                debug!("{}", msg);
                self.logger.log(&msg);
                Err(Error::Retry(err))
            }
        }
    }

    fn build(&mut self, streamer: &mut JobStreamer) -> Result<PackageArchive> {
        let network_namespace = match (
            self.config.network_interface.as_ref(),
            self.config.network_gateway.as_ref(),
        ) {
            (Some(_), Some(_)) => Some(NetworkNamespace::new(self.config.ns_dir_path())),
            (None, None) => None,
            (None, Some(_)) => return Err(Error::NoNetworkInterfaceError),
            (Some(_), None) => return Err(Error::NoNetworkGatewayError),
        };
        let status = Studio::new(
            &self.workspace,
            &self.config.bldr_url,
            &self.bldr_token,
            self.config.airlock_enabled,
            network_namespace,
        ).build(streamer)?;

        if fs::rename(self.workspace.src().join("results"), self.workspace.out()).is_err() {
            return Err(Error::BuildFailure(status.code().unwrap_or(-2)));
        }

        if !status.success() {
            let ident = self.workspace.attempted_build()?;
            let op_ident = OriginPackageIdent::from(ident);
            self.workspace.job.set_package_ident(op_ident);
            return Err(Error::BuildFailure(status.code().unwrap_or(-1)));
        }

        self.workspace.last_built()
    }

    fn export(&mut self, streamer: &mut JobStreamer) -> Result<()> {
        if self.has_docker_integration() {
            // TODO fn: This check should be updated in PackageArchive is check for run hooks.
            if self.workspace.last_built()?.is_a_service() {
                debug!("Found runnable package, running docker export");
                let mut section = streamer.start_section(Section::ExportDocker)?;

                let status = DockerExporter::new(
                    util::docker_exporter_spec(&self.workspace),
                    &self.workspace,
                    &self.config.bldr_url,
                    &self.bldr_token,
                ).export(streamer)?;

                if !status.success() {
                    return Err(Error::ExportFailure(status.code().unwrap_or(-1)));
                }

                section.end()?;
            } else {
                debug!("Package not runnable, skipping docker export");
            }
        }

        Ok(())
    }

    fn cancel(&mut self) {
        self.workspace.job.set_state(JobState::CancelComplete);
        self.logger.log_worker_job(&self.workspace.job);
    }

    fn complete(&mut self) {
        self.workspace.job.set_state(JobState::Complete);
        self.logger.log_worker_job(&self.workspace.job);
    }

    fn fail(&mut self, err: net::NetError) {
        self.teardown();
        self.workspace.job.set_state(JobState::Failed);
        self.workspace.job.set_error(err);
        self.logger.log_worker_job(&self.workspace.job);
    }

    fn setup(&mut self) -> Result<JobStreamer> {
        self.logger.log_worker_job(&self.workspace.job);

        // Ensure that data path group ownership is set to the build user and directory perms are
        // `0750`.
        if self.config.airlock_enabled {
            posix_perm::set_owner(
                &self.config.data_path,
                users::get_current_username()
                    .unwrap_or(String::from("root"))
                    .as_str(),
                STUDIO_GROUP,
            )?;
            if cfg!(not(windows)) {
                posix_perm::set_permissions(&self.config.data_path, 0o750)?;
            } else {
                unreachable!();
            }
        }

        if self.workspace.src().exists() {
            if let Some(err) = fs::remove_dir_all(self.workspace.src()).err() {
                warn!(
                    "Failed to delete directory during setup, dir={}, err={:?}",
                    self.workspace.src().display(),
                    err
                )
            }
        }
        if let Some(err) = fs::create_dir_all(self.workspace.src()).err() {
            return Err(Error::WorkspaceSetup(
                format!("{}", self.workspace.src().display()),
                err,
            ));
        }

        if self.config.airlock_enabled {
            if cfg!(not(windows)) {
                posix_perm::set_owner(self.workspace.root(), STUDIO_USER, STUDIO_GROUP)?;
                posix_perm::set_owner(self.workspace.src(), STUDIO_USER, STUDIO_GROUP)?;
            } else {
                unreachable!();
            }
        }

        if let Some(err) = fs::create_dir_all(key_path()).err() {
            return Err(Error::WorkspaceSetup(
                format!("{}", key_path().display()),
                err,
            ));
        }
        util::chown_recursive(
            (&*studio::STUDIO_HOME).lock().unwrap().join(".hab"),
            studio::studio_uid(),
            studio::studio_gid(),
        )?;

        JobStreamer::new(&self.workspace)
    }

    fn teardown(&mut self) {
        if let Some(err) = fs::remove_dir_all(self.workspace.studio()).err() {
            warn!(
                "Failed to remove studio dir {}, err: {:?}",
                self.workspace.studio().display(),
                err
            );
        }
        if let Some(err) = fs::remove_dir_all(self.workspace.src()).err() {
            warn!(
                "Failed to remove studio dir {}, err: {:?}",
                self.workspace.src().display(),
                err
            );
        }
        // TODO fn: purge the secret origin key from worker
    }

    /// Determines whether or not there is a Docker integration for the job.
    ///
    /// TODO fn: remember that for the time being we are only expecting a Docker export integration
    /// and we are assuming that any calls to this method will happen after the integration data
    /// has been validated.
    fn has_docker_integration(&self) -> bool {
        !self.workspace.job.get_project_integrations().is_empty()
    }
}

/// Client for sending and receiving messages to and from the Job Runner
pub struct RunnerCli {
    sock: zmq::Socket,
    msg: zmq::Message,
}

impl RunnerCli {
    /// Create a new Job Runner client
    pub fn new() -> Self {
        let sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        RunnerCli {
            sock: sock,
            msg: zmq::Message::new().unwrap(),
        }
    }

    /// Return a poll item used in `zmq::poll` for awaiting messages on multiple sockets
    pub fn as_poll_item<'a>(&'a self, events: i16) -> zmq::PollItem<'a> {
        self.sock.as_poll_item(events)
    }

    /// Connect to the Job Runner
    pub fn connect(&mut self) -> Result<()> {
        self.sock.connect(INPROC_ADDR)?;
        Ok(())
    }

    /// Wait until client receives a work received acknowledgement by the Runner and return
    /// the assigned JobID.
    pub fn recv_ack(&mut self) -> Result<&zmq::Message> {
        self.sock.recv(&mut self.msg, 0)?;
        if Some(WORK_ACK) != self.msg.as_str() {
            unreachable!("wk:run:1, received unexpected response from runner");
        }
        self.sock.recv(&mut self.msg, 0)?;
        Ok(&self.msg)
    }

    /// Wait until client receives a work complete message by the Runner and return an encoded
    /// representation of the job.
    pub fn recv_complete(&mut self) -> Result<&zmq::Message> {
        self.sock.recv(&mut self.msg, 0)?;
        if Some(WORK_COMPLETE) != self.msg.as_str() {
            unreachable!("wk:run:2, received unexpected response from runner");
        }
        self.sock.recv(&mut self.msg, 0)?;
        Ok(&self.msg)
    }

    /// Send a message to the Job Runner to start a Job
    pub fn start_job(&mut self, msg: &zmq::Message) -> Result<()> {
        self.sock.send_str(WORK_START, zmq::SNDMORE)?;
        self.sock.send(&*msg, 0)?;
        Ok(())
    }

    /// Send a message to the Job Runner to cancel a Job
    pub fn cancel_job(&mut self, msg: &zmq::Message) -> Result<()> {
        self.sock.send_str(WORK_CANCEL, zmq::SNDMORE)?;
        self.sock.send(&*msg, 0)?;
        Ok(())
    }
}

/// Receives work notifications from a `RunnerCli` and performs long-running tasks in a
/// separate thread.
pub struct RunnerMgr {
    config: Arc<Config>,
    net_ident: Arc<String>,
    msg: zmq::Message,
    sock: zmq::Socket,
    cancel: Arc<AtomicBool>,
}

impl RunnerMgr {
    /// Start the Job Runner
    pub fn start(config: Arc<Config>, net_ident: Arc<String>) -> Result<JoinHandle<()>> {
        let (tx, rx) = mpsc::sync_channel(0);
        let mut runner = Self::new(config, net_ident).unwrap();
        let handle = thread::Builder::new()
            .name("runner".to_string())
            .spawn(move || {
                runner.run(tx).unwrap();
            }).unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("runner thread startup error, err={}", e),
        }
    }

    fn new(config: Arc<Config>, net_ident: Arc<String>) -> Result<Self> {
        let sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER)?;
        Ok(RunnerMgr {
            config: config,
            msg: zmq::Message::new().unwrap(),
            net_ident: net_ident,
            sock: sock,
            cancel: Arc::new(AtomicBool::new(false)),
        })
    }

    // Main loop for server
    fn run(&mut self, rz: mpsc::SyncSender<()>) -> Result<()> {
        self.sock.bind(INPROC_ADDR)?;
        rz.send(()).unwrap();

        let mut srv_msg = false;
        let (tx, rx): (_, mpsc::Receiver<Job>) = mpsc::channel();

        loop {
            {
                let mut items = [self.sock.as_poll_item(1)];
                zmq::poll(&mut items, 60000)?;
                if items[0].get_revents() & zmq::POLLIN > 0 {
                    srv_msg = true;
                }
            }

            if srv_msg {
                srv_msg = false;
                self.sock.recv(&mut self.msg, 0)?;
                let op = self.msg.as_str().unwrap().to_owned();
                let mut job = self.recv_job()?;

                match &op[..] {
                    WORK_START => {
                        self.cancel.store(false, Ordering::SeqCst);
                        self.send_ack(&job)?;
                        self.spawn_job(job, tx.clone())?;
                    }
                    WORK_CANCEL => {
                        self.cancel.store(true, Ordering::SeqCst);
                        job.set_state(jobsrv::JobState::CancelProcessing);
                        self.send_ack(&job)?;
                    }
                    _ => error!("Unexpected operation"),
                }
            }

            let res = rx.try_recv();
            if res.is_ok() {
                let job: Job = res.unwrap();
                debug!("Got result from spawned runner: {:?}", job);
                self.send_complete(&job)?;
            }
        }
    }

    fn spawn_job(&mut self, job: Job, tx: mpsc::Sender<Job>) -> Result<()> {
        let runner = Runner::new(
            job,
            self.config.clone(),
            &self.net_ident,
            self.cancel.clone(),
        );

        let _ = thread::Builder::new()
            .name("job_runner".to_string())
            .spawn(move || runner.run(tx))
            .unwrap();

        Ok(())
    }

    fn recv_job(&mut self) -> Result<Job> {
        self.sock.recv(&mut self.msg, 0)?;
        let job = message::decode::<jobsrv::Job>(&self.msg)?;
        Ok(Job::new(job))
    }

    fn send_ack(&mut self, job: &Job) -> Result<()> {
        debug!("Received work, job={:?}", job);
        self.sock.send_str(WORK_ACK, zmq::SNDMORE)?;
        self.sock.send(&message::encode(&**job)?, 0)?;
        Ok(())
    }

    fn send_complete(&mut self, job: &Job) -> Result<()> {
        debug!("Completed work, job={:?}", job);
        self.sock.send_str(WORK_COMPLETE, zmq::SNDMORE)?;
        self.sock.send(&message::encode(&**job)?, 0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{jobsrv, originsrv};

    #[test]
    fn extract_origin_from_job() {
        let mut inner = jobsrv::Job::new();
        let mut project = originsrv::OriginProject::new();
        project.set_name("core/nginx".to_string());
        inner.set_project(project);
        let job = Job::new(inner);
        assert_eq!(job.origin(), "core");
    }
}
