mod docker;
mod job_streamer;
mod postprocessor;
mod publisher;
pub mod studio;
mod toml_builder;
mod util;
mod workspace;

use self::{docker::DockerExporter,
           job_streamer::{JobStreamer,
                          Section},
           postprocessor::post_process,
           studio::Studio,
           workspace::Workspace};
pub use crate::protocol::jobsrv::JobState;
use crate::{bldr_core::{self,
                        api_client::ApiClient,
                        job::Job,
                        logger::Logger,
                        socket::DEFAULT_CONTEXT},
            config::Config,
            error::{Error,
                    Result},
            hab_core::{env,
                       package::{archive::PackageArchive,
                                 target::{self,
                                          PackageTarget}}},
            protocol::{jobsrv,
                       message,
                       net::{self,
                             ErrCode},
                       originsrv::OriginPackageIdent},
            vcs::VCS};
use chrono::Utc;
use futures::{channel::mpsc as async_mpsc,
              sink::SinkExt};
use retry::delay;
use std::{fs,
          process::Command,
          str::FromStr,
          sync::{atomic::{AtomicBool,
                          Ordering},
                 mpsc,
                 Arc},
          thread::{self,
                   JoinHandle},
          time::Duration};
use zmq;

// TODO fn: copied from `components/common/src/ui.rs`. As this component doesn't currently depend
// on habitat_common it didnt' seem worth it to add a dependency for only this constant. Probably
// means that the constant should be relocated to habitat_core.
/// Environment variable to disable progress bars in Habitat programs
const NONINTERACTIVE_ENVVAR: &str = "HAB_NONINTERACTIVE";

/// Environment variable to enable or disable debug output in runner's studio
const RUNNER_DEBUG_ENVVAR: &str = "BUILDER_RUNNER_DEBUG";

/// Environment variable to disable workspace teardown
const RUNNER_NO_TEARDOWN: &str = "BUILDER_RUNNER_NO_TEARDOWN";

/// Environment variable to enable or disable dev mode.
const DEV_MODE: &str = "DEV_MODE";
/// In-memory zmq address of Job RunnerMgr
const INPROC_ADDR: &str = "inproc://runner";
/// Protocol message to indicate the Job Runner has received a work request
const WORK_ACK: &str = "A";
/// Protocol message to indicate the Job Runner has completed a work request
const WORK_COMPLETE: &str = "C";
/// Protocol message to indicate the Runner Cli is sending a work request
const WORK_START: &str = "S";
/// Protocol message to indicate the Runner Cli is sending a cancel request
const WORK_CANCEL: &str = "X";

pub const RETRIES: usize = 10;
pub const RETRY_WAIT: Duration = Duration::from_secs(60);

/// Interval for main thread to check child status
pub const STUDIO_CHILD_WAIT_SECS: u64 = 10;

pub struct Runner {
    config:     Arc<Config>,
    depot_cli:  ApiClient,
    workspace:  Workspace,
    logger:     Logger,
    bldr_token: String,
    cancel:     Arc<AtomicBool>,
}

impl Runner {
    pub fn new(job: Job,
               config: Arc<Config>,
               net_ident: &str,
               cancel: Arc<AtomicBool>)
               -> Result<Self> {
        debug!("Creating new Runner with config: {:?}", config);
        let depot_cli = ApiClient::new(&config.bldr_url)?;

        let log_path = config.log_path.clone();
        let mut logger = Logger::init(log_path, "builder-worker.log");
        logger.log_ident(net_ident);
        let bldr_token = bldr_core::access_token::generate_bldr_token(&config.key_dir).unwrap();

        Ok(Runner { workspace: Workspace::new(&config.data_path, job),
                    config,
                    depot_cli,
                    logger,
                    bldr_token,
                    cancel })
    }

    pub fn job(&self) -> &Job { &self.workspace.job }

    pub fn job_mut(&mut self) -> &mut Job { &mut self.workspace.job }

    fn is_canceled(&self) -> bool { self.cancel.load(Ordering::SeqCst) }

    async fn check_cancel(&mut self, tx: &mut async_mpsc::UnboundedSender<Job>) -> Result<()> {
        if self.is_canceled() {
            debug!("Runner canceling job id: {}", self.job().get_id());
            self.cancel();
            self.cleanup();
            tx.send(self.job().clone())
              .await
              .map_err(Error::MpscAsync)?;
            return Err(Error::JobCanceled);
        }

        Ok(())
    }

    async fn do_validate(&mut self,
                         tx: &mut async_mpsc::UnboundedSender<Job>,
                         streamer: &mut JobStreamer)
                         -> Result<()> {
        self.check_cancel(tx).await?;

        let mut section = streamer.start_section(Section::ValidateIntegrations)?;

        if let Some(err) = util::validate_integrations(&self.workspace).err() {
            let msg = format!("Failed to validate integrations for {}, err={:?}",
                              self.workspace.job.get_project().get_name(),
                              err);
            debug!("{}", msg);
            self.logger.log(&msg);

            streamer.println_stderr(msg)?;
            self.fail(net::err(ErrCode::INVALID_INTEGRATIONS, "wk:run:validate"));
            tx.send(self.job().clone())
              .await
              .map_err(Error::MpscAsync)?;
            return Err(err);
        };

        section.end()?;
        Ok(())
    }

    async fn do_setup(&mut self, tx: &mut async_mpsc::UnboundedSender<Job>) -> Result<JobStreamer> {
        self.check_cancel(tx).await?;

        let streamer = match self.setup() {
            Ok(streamer) => streamer,
            Err(err) => {
                let msg = format!("Failed to setup workspace for {}, err={:?}",
                                  self.workspace.job.get_project().get_name(),
                                  err);
                warn!("{}", msg);
                self.logger.log(&msg);

                self.fail(net::err(ErrCode::WORKSPACE_SETUP, "wk:run:workspace"));
                tx.send(self.job().clone())
                  .await
                  .map_err(Error::MpscAsync)?;
                return Err(err);
            }
        };

        Ok(streamer)
    }

    async fn do_install_key(&mut self,
                            tx: &mut async_mpsc::UnboundedSender<Job>,
                            streamer: &mut JobStreamer)
                            -> Result<()> {
        self.check_cancel(tx).await?;

        let mut section = streamer.start_section(Section::FetchOriginKey)?;

        if let Some(err) = self.install_origin_secret_key().await.err() {
            let msg = format!("Failed to install origin secret key {}, err={:?}",
                              self.workspace.job.get_project().get_origin_name(),
                              err);
            debug!("{}", msg);
            self.logger.log(&msg);

            streamer.println_stderr(msg)?;
            self.fail(net::err(ErrCode::SECRET_KEY_FETCH, "wk:run:key"));
            tx.send(self.job().clone())
              .await
              .map_err(Error::MpscAsync)?;
            return Err(err);
        }

        section.end()?;
        Ok(())
    }

    async fn do_clone(&mut self,
                      tx: &mut async_mpsc::UnboundedSender<Job>,
                      streamer: &mut JobStreamer)
                      -> Result<()> {
        self.check_cancel(tx).await?;
        let mut section = streamer.start_section(Section::CloneRepository)?;

        let vcs = VCS::from_job(&self.job(), self.config.github.clone())?;
        if let Some(err) = vcs.clone(&self.workspace.src()).await.err() {
            let msg = format!("Failed to clone remote source repository for {}, err={:?}",
                              self.workspace.job.get_project().get_name(),
                              err);
            warn!("{}", msg);
            self.logger.log(&msg);

            streamer.println_stderr(msg)?;
            self.fail(net::err(ErrCode::VCS_CLONE, "wk:run:clone:1"));
            tx.send(self.job().clone())
              .await
              .map_err(Error::MpscAsync)?;
            return Err(err);
        }

        section.end()?;
        Ok(())
    }

    async fn do_build(&mut self,
                      tx: &mut async_mpsc::UnboundedSender<Job>,
                      streamer: &mut JobStreamer)
                      -> Result<PackageArchive> {
        self.check_cancel(tx).await?;

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
        let mut archive = match self.build(self.config.target, streamer, tx).await {
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
                let msg = format!("Failed studio build for {}, err={:?}",
                                  self.workspace.job.get_project().get_name(),
                                  err);
                debug!("{}", msg);
                self.logger.log(&msg);
                streamer.println_stderr(msg)?;

                self.fail(net::err(ErrCode::BUILD, "wk:run:build"));
                tx.send(self.job().clone())
                  .await
                  .map_err(Error::MpscAsync)?;
                return Err(err);
            }
        };

        // Converting from a core::PackageIdent to an OriginPackageIdent
        let ident = OriginPackageIdent::from(archive.ident().unwrap());
        self.workspace.job.set_package_ident(ident);

        section.end()?;
        Ok(archive)
    }

    async fn do_export(&mut self,
                       tx: &mut async_mpsc::UnboundedSender<Job>,
                       mut streamer: &mut JobStreamer)
                       -> Result<()> {
        self.check_cancel(tx).await?;

        match self.export(&mut streamer) {
            Ok(_) => (),
            Err(err) => {
                self.fail(net::err(ErrCode::EXPORT, "wk:run:export"));
                tx.send(self.job().clone())
                  .await
                  .map_err(Error::MpscAsync)?;
                return Err(err);
            }
        }

        Ok(())
    }

    async fn do_postprocess(&mut self,
                            tx: &mut async_mpsc::UnboundedSender<Job>,
                            mut archive: PackageArchive,
                            streamer: &mut JobStreamer)
                            -> Result<()> {
        self.check_cancel(tx).await?;
        let mut section = streamer.start_section(Section::PublishPackage)?;

        match post_process(&mut archive,
                           &self.workspace,
                           &self.config,
                           &self.bldr_token,
                           &mut self.logger).await
        {
            Ok(_) => (),
            Err(err) => {
                let msg = format!("Failed post processing for {}, err={:?}",
                                  self.workspace.job.get_project().get_name(),
                                  err);
                streamer.println_stderr(msg)?;
                self.fail(net::err(ErrCode::POST_PROCESSOR, "wk:run:postprocess"));
                tx.send(self.job().clone())
                  .await
                  .map_err(Error::MpscAsync)?;
                return Err(err);
            }
        }

        section.end()?;
        Ok(())
    }

    fn cleanup(&mut self) {
        if let Some(err) = fs::remove_dir_all(self.workspace.out()).err() {
            warn!("Failed to delete directory during cleanup, dir={}, err={:?}",
                  self.workspace.out().display(),
                  err)
        }
        self.teardown();
    }

    pub async fn run(mut self, mut tx: async_mpsc::UnboundedSender<Job>) -> Result<()> {
        // TBD (SA) - Spin up a LogStreamer first thing indpendendly of setup.
        // Currently we need to do it as part of setup because the log file to be
        // streamed lives inside of the workspace, which is created by setup.
        let mut streamer = self.do_setup(&mut tx).await?;

        self.do_validate(&mut tx, &mut streamer).await?;
        self.do_install_key(&mut tx, &mut streamer).await?;
        self.do_clone(&mut tx, &mut streamer).await?;

        let archive = self.do_build(&mut tx, &mut streamer).await?;
        self.do_export(&mut tx, &mut streamer).await?;
        self.do_postprocess(&mut tx, archive, &mut streamer).await?;

        self.cleanup();
        self.complete();
        tx.send(self.workspace.job)
          .await
          .map_err(Error::MpscAsync)?;

        streamer.finish()?;

        Ok(())
    }

    async fn install_origin_secret_key(&mut self) -> Result<()> {
        debug!("Installing origin secret key for {} to {:?}",
               self.job().origin(),
               self.workspace.key_path());
        match retry::retry_future!(delay::Fixed::from(RETRY_WAIT).take(RETRIES),
                                   self.fetch_origin_secret_key()).await
        {
            Ok(dst) => {
                debug!("Imported origin secret key, dst={:?}.", dst);
                Ok(())
            }
            Err(err) => {
                let msg = format!("Failed to import secret key {} after {} retries",
                                  self.job().origin(),
                                  RETRIES);
                debug!("{}", msg);
                self.logger.log(&msg);
                Err(Error::Retry(err))
            }
        }
    }

    async fn fetch_origin_secret_key(
        &self)
        -> std::result::Result<std::path::PathBuf, builder_core::Error> {
        let res = self.depot_cli
                      .fetch_origin_secret_key(self.job().origin(),
                                               &self.bldr_token,
                                               self.workspace.key_path())
                      .await;
        if res.is_err() {
            debug!("Failed to fetch origin secret key, err={:?}", res);
        };

        res
    }

    async fn build(&mut self,
                   target: PackageTarget,
                   streamer: &mut JobStreamer,
                   tx: &mut async_mpsc::UnboundedSender<Job>)
                   -> Result<PackageArchive> {
        let studio = Studio::new(&self.workspace,
                                 &self.config.bldr_url,
                                 &self.bldr_token,
                                 target);
        clean_container();

        let mut child = studio.build(streamer)?;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    debug!("Completed studio build, status={:?}", status);

                    let result_path = self.workspace.src().join("results");
                    match fs::rename(&result_path, self.workspace.out()) {
                        Ok(_) => (),
                        Err(err) => {
                            debug!("Failed to rename studio results dir: {:?} to {:?}. Err = {:?}",
                                   result_path,
                                   self.workspace.out(),
                                   err);
                            return Err(Error::BuildFailure(status.code().unwrap_or(-2)));
                        }
                    }

                    if !status.success() {
                        debug!("Status is not success");
                        let ident = self.workspace.attempted_build()?;
                        let op_ident = OriginPackageIdent::from(ident);
                        self.workspace.job.set_package_ident(op_ident);
                        return Err(Error::BuildFailure(status.code().unwrap_or(-1)));
                    }

                    return self.workspace.last_built();
                }
                Ok(None) => {
                    if self.is_canceled() {
                        debug!("Canceling job: {}", self.job().get_id());
                        clean_container();
                        if let Err(err) = child.kill() {
                            debug!("Failed to kill child, err: {:?}", err);
                        }
                        self.cancel();
                        self.cleanup();
                        tx.send(self.job().clone())
                          .await
                          .map_err(Error::MpscAsync)?;
                        return Err(Error::JobCanceled);
                    }
                    thread::sleep(Duration::new(STUDIO_CHILD_WAIT_SECS, 0));
                    continue;
                }
                Err(err) => {
                    debug!("Error attempting to wait: {}", err);
                    return Err(Error::StudioBuild(self.workspace.studio().to_path_buf(), err));
                }
            }
        }
    }

    fn export(&mut self, streamer: &mut JobStreamer) -> Result<()> {
        if self.has_docker_integration() {
            let pkg_target = target::PackageTarget::from_str(self.workspace.job.get_target())?;
            match pkg_target {
                target::X86_64_LINUX | target::X86_64_WINDOWS => {
                    // TODO fn: This check should be updated in PackageArchive is check for run
                    // hooks.
                    if self.workspace.last_built()?.is_a_service() {
                        debug!("Found runnable package, running docker export");
                        let mut section = streamer.start_section(Section::ExportDocker)?;

                        let status =
                            DockerExporter::new(util::docker_exporter_spec(&self.workspace),
                                                &self.workspace,
                                                &self.config.bldr_url,
                                                &self.bldr_token).export(streamer)?;

                        if !status.success() {
                            return Err(Error::ExportFailure(status.code().unwrap_or(-1)));
                        }

                        section.end()?;
                    } else {
                        debug!("Package not runnable, skipping docker export");
                    }
                }
                _ => debug!("Exports for {} are not supported", pkg_target.as_ref()),
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

        if self.workspace.src().exists() {
            debug!("Workspace src exists, removing: {:?}",
                   self.workspace.src().display());

            if let Some(err) = fs::remove_dir_all(self.workspace.src()).err() {
                warn!("Failed to delete directory during setup, dir={}, err={:?}",
                      self.workspace.src().display(),
                      err)
            }
        }

        debug!("Creating workspace src directory: {}",
               self.workspace.src().display());
        if let Some(err) = fs::create_dir_all(self.workspace.src()).err() {
            return Err(Error::WorkspaceSetup(format!("{}",
                                                     self.workspace
                                                         .src()
                                                         .display()),
                                             err));
        }

        debug!("Creating workspace keys directory: {}",
               self.workspace.key_path().display());
        if let Some(err) = fs::create_dir_all(self.workspace.key_path()).err() {
            return Err(Error::WorkspaceSetup(format!("{}",
                                                     self.workspace
                                                         .key_path()
                                                         .display()),
                                             err));
        }

        Ok(JobStreamer::new(&self.workspace))
    }

    fn teardown(&mut self) {
        if let Some(_val) = env::var_os(RUNNER_NO_TEARDOWN) {
            debug!("RUNNER_DEBUG_ENVVAR ({}) is set - skipping teardown",
                   RUNNER_NO_TEARDOWN);
        } else {
            debug!("Tearing down workspace: {}",
                   self.workspace.root().display());

            if let Some(err) = fs::remove_dir_all(self.workspace.studio()).err() {
                warn!("Failed to remove studio dir {}, err: {:?}",
                      self.workspace.studio().display(),
                      err);
            }
            if let Some(err) = fs::remove_dir_all(self.workspace.src()).err() {
                warn!("Failed to remove studio dir {}, err: {:?}",
                      self.workspace.src().display(),
                      err);
            }
            if let Some(err) = fs::remove_dir_all(self.workspace.key_path()).err() {
                warn!("Failed to remove studio dir {}, err: {:?}",
                      self.workspace.src().display(),
                      err);
            }
        }
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

fn clean_container() {
    let mut cmd = Command::new(&"docker");
    cmd.arg("rm");
    cmd.arg("builder");
    cmd.arg("--force");
    match cmd.output() {
        Ok(output) => debug!("docker rm status: {}", output.status),
        Err(err) => error!("Failed to remove docker container, err={:?}", err),
    }
}

/// Client for sending and receiving messages to and from the Job Runner
pub struct RunnerCli {
    sock: zmq::Socket,
    msg:  zmq::Message,
}

impl Default for RunnerCli {
    fn default() -> Self { Self::new() }
}

impl RunnerCli {
    /// Create a new Job Runner client
    pub fn new() -> Self {
        let sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        RunnerCli { sock,
                    msg: zmq::Message::new().unwrap() }
    }

    /// Return a poll item used in `zmq::poll` for awaiting messages on multiple sockets
    pub fn as_poll_item(&self, events: i16) -> zmq::PollItem { self.sock.as_poll_item(events) }

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
    config:    Arc<Config>,
    net_ident: Arc<String>,
    msg:       zmq::Message,
    sock:      zmq::Socket,
    cancel:    Arc<AtomicBool>,
}

impl RunnerMgr {
    /// Start the Job Runner
    pub fn start(config: Arc<Config>, net_ident: Arc<String>) -> Result<JoinHandle<()>> {
        let (tx, rx) = mpsc::sync_channel(0);
        let mut runner = Self::new(config, net_ident);
        let handle = thread::Builder::new().name("runner".to_string())
                                           .spawn(move || {
                                               runner.run(&tx).unwrap();
                                           })
                                           .unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("runner thread startup error, err={}", e),
        }
    }

    fn new(config: Arc<Config>, net_ident: Arc<String>) -> Self {
        let sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        RunnerMgr { config,
                    msg: zmq::Message::new().unwrap(),
                    net_ident,
                    sock,
                    cancel: Arc::new(AtomicBool::new(false)) }
    }

    // Main loop for server
    fn run(&mut self, rz: &mpsc::SyncSender<()>) -> Result<()> {
        self.sock.bind(INPROC_ADDR)?;
        rz.send(()).unwrap();

        let mut srv_msg = false;
        let (tx, mut rx): (_, async_mpsc::UnboundedReceiver<Job>) = async_mpsc::unbounded();

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

            let res = rx.try_next();

            if let Ok(Some(job)) = res {
                debug!("Got result from spawned runner: {:?}", job);
                self.send_complete(&job)?;
            }
        }
    }

    fn spawn_job(&mut self, job: Job, tx: async_mpsc::UnboundedSender<Job>) -> Result<()> {
        let runner = Runner::new(job,
                                 self.config.clone(),
                                 &self.net_ident,
                                 self.cancel.clone())?;
        // TODO: SM This will spawn a new tokio runtime for each job. At this point, the workers
        // only operate on a single task at once, and the setup of the runtime is minimal compared
        // to the average run duration of this thread (minutes).
        let _ = thread::Builder::new().name("job_runner".to_string())
                                      .spawn(move || {
                                          tokio::runtime::Runtime::new().expect("Unable to create \
                                                                                 tokio runtime")
                                                                        .block_on(runner.run(tx))
                                      })
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
    use crate::protocol::{jobsrv,
                          originsrv};

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
