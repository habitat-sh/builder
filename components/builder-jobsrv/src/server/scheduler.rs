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

use std::{collections::{HashMap,
                        HashSet},
          str::FromStr,
          sync::mpsc,
          thread::{self,
                   JoinHandle}};

use chrono::{DateTime,
             Duration,
             Utc};

use crate::{config::Config,
            data_store::DataStore,
            db::DbPool,
            error::{Error,
                    Result},
            protocol::jobsrv};

use crate::db::models::{jobs::*,
                        package::*,
                        projects::*};

use crate::{bldr_core::{logger::Logger,
                        metrics::{CounterMetric,
                                  GaugeMetric,
                                  HistogramMetric},
                        socket::DEFAULT_CONTEXT},
            hab_core::package::{target,
                                PackageIdent,
                                PackageTarget},
            server::feat};

use super::{metrics::{Counter,
                      Gauge,
                      Histogram},
            worker_manager::WorkerMgrClient};

const SCHEDULER_ADDR: &str = "inproc://scheduler";
const SOCKET_TIMEOUT_MS: i64 = 60_000;

pub struct ScheduleClient {
    socket: zmq::Socket,
}

impl ScheduleClient {
    pub fn connect(&mut self) -> Result<()> {
        self.socket.connect(SCHEDULER_ADDR)?;
        Ok(())
    }

    pub fn notify(&mut self) -> Result<()> {
        self.socket.send(&[1], 0)?;
        Ok(())
    }
}

impl Default for ScheduleClient {
    fn default() -> ScheduleClient {
        let socket = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        socket.connect(SCHEDULER_ADDR).unwrap();
        ScheduleClient { socket }
    }
}

pub struct ScheduleMgr {
    datastore:     DataStore,
    db:            DbPool,
    logger:        Logger,
    msg:           zmq::Message,
    schedule_cli:  ScheduleClient,
    socket:        zmq::Socket,
    worker_mgr:    WorkerMgrClient,
    build_targets: HashSet<PackageTarget>,
    job_timeout:   Duration,
}

impl ScheduleMgr {
    pub fn new(cfg: &Config, datastore: &DataStore, db: DbPool) -> Self {
        let socket = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();

        let mut schedule_cli = ScheduleClient::default();
        schedule_cli.connect().unwrap();

        let mut worker_mgr = WorkerMgrClient::default();
        worker_mgr.connect().unwrap();

        ScheduleMgr { datastore: datastore.clone(),
                      db,
                      logger: Logger::init(cfg.log_path.clone(), "builder-scheduler.log"),
                      msg: zmq::Message::new().unwrap(),
                      schedule_cli,
                      socket,
                      worker_mgr,
                      build_targets: cfg.build_targets.clone(),
                      job_timeout: Duration::minutes(cfg.job_timeout as i64) }
    }

    pub fn start(cfg: &Config, datastore: &DataStore, db: DbPool) -> Result<JoinHandle<()>> {
        let (tx, rx) = mpsc::sync_channel(1);
        let mut schedule_mgr = Self::new(cfg, datastore, db);
        let handle = thread::Builder::new().name("scheduler".to_string())
                                           .spawn(move || {
                                               schedule_mgr.run(&tx).unwrap();
                                           })
                                           .unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("scheduler thread startup error, err={}", e),
        }
    }

    fn run(&mut self, rz: &mpsc::SyncSender<()>) -> Result<()> {
        self.socket.bind(SCHEDULER_ADDR)?;

        let mut socket = false;
        rz.send(()).unwrap();
        loop {
            {
                let mut items = [self.socket.as_poll_item(1)];
                if let Err(err) = zmq::poll(&mut items, SOCKET_TIMEOUT_MS) {
                    warn!("Scheduler unable to complete ZMQ poll: err {:?}", err);
                };

                if (items[0].get_revents() & zmq::POLLIN) > 0 {
                    socket = true;
                }
            }

            for target in PackageTarget::targets() {
                if self.build_targets.contains(target) {
                    if let Err(err) = self.process_metrics(*target) {
                        warn!("Scheduler unable to process metrics: err {:?}", err);
                    }

                    if let Err(err) = self.process_status(*target) {
                        warn!("Scheduler unable to process status: err {:?}", err);
                    }

                    if let Err(err) = self.process_queue(*target) {
                        warn!("Scheduler unable to process queue: err {:?}", err);
                    }

                    if let Err(err) = self.process_work(*target) {
                        warn!("Scheduler unable to process work: err {:?}", err);
                    }

                    if let Err(err) = self.watchdog(*target) {
                        warn!("Scheduler unable to execute watchdog task: err: {:?}", err);
                    }
                }
            }

            if socket {
                if let Err(err) = self.socket.recv(&mut self.msg, 0) {
                    warn!("Scheduler unable to complete socket receive: err {:?}", err);
                }
                socket = false;
            }
        }
    }

    fn log_error(&mut self, msg: &str) {
        warn!("{}", msg);
        self.logger.log(msg);
    }

    fn process_metrics(&mut self, target: PackageTarget) -> Result<()> {
        let conn = self.db.get_conn().map_err(Error::Db)?;
        let waiting_jobs = Job::count(jobsrv::JobState::Pending, target, &*conn)?;
        let working_jobs = Job::count(jobsrv::JobState::Dispatched, target, &*conn)?;

        Gauge::WaitingJobs(target).set(waiting_jobs as f64);
        Gauge::WorkingJobs(target).set(working_jobs as f64);

        Ok(())
    }

    fn process_queue(&mut self, target: PackageTarget) -> Result<()> {
        let conn = self.db.get_conn().map_err(Error::Db)?;
        let groups = Group::get_all_queued(target, &*conn)?;

        for group in groups.iter() {
            assert!(group.group_state == jobsrv::JobGroupState::GroupQueued.to_string());

            match Group::get_active(&group.project_name, target, &*conn) {
                Ok(group) => {
                    trace!("Found active project {} for target {}, skipping queued job",
                           group.project_name,
                           target);
                }
                Err(diesel::result::Error::NotFound) => {
                    debug!("Setting group {} from queued to pending",
                           group.project_name);
                    self.datastore.set_job_group_state(group.id as u64,
                                                        jobsrv::JobGroupState::GroupPending)?;
                }
                Err(err) => {
                    debug!("Failed to get active group, err = {}", err);
                }
            }
        }

        Ok(())
    }

    fn process_work(&mut self, target: PackageTarget) -> Result<()> {
        let conn = self.db.get_conn().map_err(Error::Db)?;

        loop {
            // Take oldest group from the pending list
            let group = match Group::get_pending(target, &*conn) {
                Ok(group) => self.get_group(group.id as u64)?,
                Err(diesel::result::Error::NotFound) => break,
                Err(err) => {
                    debug!("Failed to get pending group, err = {:?}", err);
                    return Err(Error::DieselError(err));
                }
            };

            debug!("Found pending group {:?} for target {}", group, target);

            assert!(group.get_state() == jobsrv::JobGroupState::GroupPending);
            self.dispatch_group(&group)?;
            self.update_group_state(group.get_id())?;
        }

        Ok(())
    }

    fn watchdog(&mut self, target: PackageTarget) -> Result<()> {
        let conn = self.db.get_conn().map_err(Error::Db)?;

        let groups = match Group::get_all_dispatching(target, &*conn) {
            Ok(groups) => groups,
            Err(diesel::result::Error::NotFound) => return Ok(()),
            Err(err) => {
                debug!("Failed to get dispatching groups, err = {:?}", err);
                return Err(Error::DieselError(err));
            }
        };

        if !groups.is_empty() {
            debug!("Watchdog found {} dispatching groups for target {}: {:?}",
                   groups.len(),
                   target,
                   groups);
        }

        for group in groups {
            self.check_group(&group)?;
        }

        Ok(())
    }

    fn check_group(&mut self, group: &Group) -> Result<()> {
        let group = self.get_group(group.id as u64)?;

        let is_buildable = group.get_projects().iter().any(|x| buildable(x));
        if !is_buildable {
            let msg = format!("Watchdog: canceling group {} with no buildable projects",
                              group.get_id());
            error!("{}", &msg);
            self.log_error(&msg);
            self.datastore.cancel_job_group(group.get_id())?;
        } else {
            for project in
                group.get_projects()
                     .iter()
                     .filter(|x| x.get_state() == jobsrv::JobGroupProjectState::InProgress)
            {
                self.check_project(project)?;
            }
        }
        Ok(())
    }

    fn check_project(&mut self, project: &jobsrv::JobGroupProject) -> Result<()> {
        assert!(project.get_state() == jobsrv::JobGroupProjectState::InProgress);
        let conn = self.db.get_conn().map_err(Error::Db)?;
        let job = match Job::get(project.get_job_id() as i64, &*conn) {
            Ok(job) => job,
            Err(err) => {
                error!("Unable to retrieve job: {:?}", err);
                return Err(Error::DieselError(err));
            }
        };

        let utc: DateTime<Utc> = Utc::now();
        let duration_since =
            utc.signed_duration_since(job.created_at.expect("job has a created_at field"));

        if duration_since > self.job_timeout {
            debug!("Job {} has been running for: {:?}", job.id, duration_since);
            let msg = format!("Watchdog: canceling job {} (exceeded timeout: {} sec)",
                              job.id,
                              duration_since.num_seconds());
            error!("{}", &msg);
            self.log_error(&msg);
            let mut job: jobsrv::Job = job.into();
            job.set_state(jobsrv::JobState::CancelPending);
            self.datastore.update_job(&job)?;
        }
        Ok(())
    }

    fn dispatch_group(&mut self, group: &jobsrv::JobGroup) -> Result<()> {
        debug!("Dispatching group {}", group.get_id());
        self.logger.log_group(group);
        self.datastore
            .set_job_group_state(group.get_id(), jobsrv::JobGroupState::GroupDispatching)?;

        let mut skipped = HashMap::new();
        let dispatchable = self.dispatchable_projects(group)?;

        for project in dispatchable {
            if skipped.contains_key(project.get_name()) {
                continue;
            }

            debug!("Dispatching project: {:?}", project.get_name());
            self.logger.log_group_project(group, &project);

            assert!(project.get_state() == jobsrv::JobGroupProjectState::NotStarted);

            match self.schedule_job(group.get_id(), project.get_name(), group.get_target()) {
                Ok(job_opt) => {
                    match job_opt {
                        Some(job) => self.datastore.set_job_group_job_state(&job)?,
                        None => {
                            debug!("Skipping project: {:?}", project.get_name());
                            self.datastore.set_job_group_project_state(
                            group.get_id(),
                            project.get_name(),
                            jobsrv::JobGroupProjectState::Skipped,
                        )?;

                            let skip_list = match self.skip_projects(group, project.get_name()) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.log_error(&format!("Error skipping projects for {:?} \
                                                             (group: {}): {:?}",
                                                            project.get_name(),
                                                            group.get_id(),
                                                            e));
                                    return Err(e);
                                }
                            };
                            for name in skip_list {
                                skipped.insert(name, true);
                            }
                        }
                    }
                }
                Err(err) => {
                    self.log_error(&format!("Failed to schedule job for {} (group: {}), err: \
                                             {:?}",
                                            project.get_name(),
                                            group.get_id(),
                                            err));

                    // TODO: Is this the right thing to do?
                    self.datastore
                        .set_job_group_state(group.get_id(), jobsrv::JobGroupState::GroupFailed)?;
                    self.datastore
                        .set_job_group_project_state(group.get_id(),
                                                     project.get_name(),
                                                     jobsrv::JobGroupProjectState::Failure)?;

                    // TODO: Make this cleaner later
                    let mut updated_group = group.clone();
                    updated_group.set_state(jobsrv::JobGroupState::GroupFailed);
                    self.logger.log_group(&updated_group);

                    break;
                }
            }
        }
        Ok(())
    }

    fn dispatchable_projects(&mut self,
                             group: &jobsrv::JobGroup)
                             -> Result<Vec<jobsrv::JobGroupProject>> {
        let conn = self.db.get_conn().map_err(Error::Db)?;
        let mut projects = Vec::new();
        for project in group.get_projects()
                            .iter()
                            .filter(|x| x.get_state() == jobsrv::JobGroupProjectState::NotStarted)
        {
            // Check the deps for the project. If we don't find any dep that
            // is in our project list and needs to be built, we can dispatch the project.
            // NOTE: get_ident().is_empty() is only true if the project has never been built
            // Otherwise ident is going to be the FQPI of the latest ident (which channel is
            // unclear)
            let dispatchable = if project.get_ident().is_empty() {
                true
            } else {
                let mut check_status = true;

                let package = match Package::get(
                    GetPackage {
                        ident: BuilderPackageIdent(
                            PackageIdent::from_str(project.get_ident())?,
                        ),
                        visibility: vec![
                            PackageVisibility::Public,
                            PackageVisibility::Private,
                            PackageVisibility::Hidden,
                        ],
                        target: BuilderPackageTarget(
                            PackageTarget::from_str(group.get_target())?,
                        ),
                    },
                    &*conn,
                ) {
                    Ok(pkg) => pkg,
                    Err(err) => {
                        warn!(
                            "Failed to retrieve package (possibly deleted?): {} ({}). Err={:?}",
                            &project.get_ident(),
                            &group.get_target(),
                            err
                        );
                        continue;
                    }
                };
                for dep in package.deps {
                    let name = format!("{}/{}", dep.origin, dep.name);

                    if !self.check_dispatchable(group, &name) {
                        check_status = false;
                        break;
                    };
                }
                check_status
            };

            if dispatchable {
                projects.push(project.clone());
            }
        }
        debug!("Found {} dispatchable projects for group {}",
               projects.len(),
               group.get_id());
        Ok(projects)
    }

    fn check_dispatchable(&mut self, group: &jobsrv::JobGroup, name: &str) -> bool {
        for project in group.get_projects() {
            if (project.get_name() == name)
               && (project.get_state() != jobsrv::JobGroupProjectState::Success)
            {
                return false;
            }
        }
        true
    }

    fn skip_projects(&mut self,
                     group: &jobsrv::JobGroup,
                     project_name: &str)
                     -> Result<Vec<String>> {
        let conn = self.db.get_conn().map_err(Error::Db)?;
        let mut skipped = HashMap::new();
        skipped.insert(project_name.to_string(), true);

        for project in group.get_projects()
                            .iter()
                            .filter(|x| x.get_state() == jobsrv::JobGroupProjectState::NotStarted)
        {
            // Check the deps for the project. If we find any dep that is in the
            // skipped list, we set the project status to Skipped and add it to the list
            let package = match Package::get(
                GetPackage {
                    ident: BuilderPackageIdent(
                        PackageIdent::from_str(project.get_ident())?,
                    ),
                    visibility: vec![
                        PackageVisibility::Public,
                        PackageVisibility::Private,
                        PackageVisibility::Hidden,
                    ],
                    target: BuilderPackageTarget(
                        PackageTarget::from_str(group.get_target())?,
                    ),
                },
                &*conn,
            ) {
                Ok(package) => package,
                Err(err) => {
                    warn!(
                        "Unable to retrieve job graph package {} ({}), err: {:?}",
                        project.get_ident(),
                        group.get_target(),
                        err
                    );
                    continue;
                }
            };

            for dep in package.deps {
                let name = format!("{}/{}", dep.origin, dep.name);

                if skipped.contains_key(&name) {
                    debug!("Skipping project {:?}", project.get_name());
                    self.datastore
                        .set_job_group_project_state(group.get_id(),
                                                     project.get_name(),
                                                     jobsrv::JobGroupProjectState::Skipped)?;
                    skipped.insert(project.get_name().to_string(), true);
                    break;
                }
            }
        }

        Ok(skipped.keys().map(|s| s.to_string()).collect())
    }

    fn schedule_job(&mut self,
                    group_id: u64,
                    project_name: &str,
                    target: &str)
                    -> Result<Option<jobsrv::Job>> {
        let conn = self.db.get_conn().map_err(Error::Db)?;

        let get_target = if feat::is_enabled(feat::LegacyProject) {
            "x86_64-linux"
        } else {
            target
        };

        let project = match Project::get(project_name, get_target, &*conn) {
            Ok(project) => project,
            Err(diesel::result::Error::NotFound) => {
                // It's valid to not have a project connected
                debug!("Unable to retrieve project: {:?} (not found)", project_name);
                return Ok(None);
            }
            Err(err) => {
                self.log_error(&format!("Unable to retrieve project: {:?} (group: {}), error: \
                                         {:?}",
                                        project_name, group_id, err));
                return Ok(None);
            }
        };

        let created = self.datastore
                          .create_job_for_project(group_id, project, target);
        if created.is_ok() {
            self.worker_mgr.notify_work()?;
        }
        created
    }

    fn get_group(&mut self, group_id: u64) -> Result<jobsrv::JobGroup> {
        let mut msg: jobsrv::JobGroupGet = jobsrv::JobGroupGet::new();
        msg.set_group_id(group_id);
        msg.set_include_projects(true);

        match self.datastore.get_job_group(&msg) {
            Ok(group_opt) => {
                match group_opt {
                    Some(group) => Ok(group),
                    None => Err(Error::UnknownJobGroup),
                }
            }
            Err(err) => {
                self.log_error(&format!("Failed to get group {} from datastore: {:?}",
                                        group_id, err));
                Err(err)
            }
        }
    }

    fn process_status(&mut self, _target: PackageTarget) -> Result<()> {
        // Get a list of jobs with un-sync'd status
        let jobs = self.datastore.sync_jobs()?;
        if !jobs.is_empty() {
            debug!("Process status: found {} updated jobs", jobs.len());
        }

        for job in jobs {
            debug!("Syncing job status: job={:?}", job);

            let group: jobsrv::JobGroup = match self.get_group(job.get_owner_id()) {
                Ok(group) => group,
                Err(Error::UnknownJobGroup) => {
                    // UnknownGroup is ok, just unset the sync and move on
                    debug!("Skipping unknown group {:?}", job.get_owner_id());
                    self.datastore.set_job_sync(job.get_owner_id())?;
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            };

            self.logger.log_group_job(&group, &job);
            let target = match PackageTarget::from_str(job.get_target()) {
                Ok(t) => t,
                Err(_) => target::X86_64_LINUX,
            };

            match job.get_state() {
                jobsrv::JobState::Complete => {
                    Counter::CompletedJobs(target).increment();
                    assert!(job.has_build_started_at());
                    assert!(job.has_build_finished_at());

                    let build_started_at = job.get_build_started_at().parse::<DateTime<Utc>>()?;
                    let build_finished_at = job.get_build_finished_at().parse::<DateTime<Utc>>()?;

                    let build_duration = build_finished_at - build_started_at;
                    Histogram::JobCompletionTime(target).set(build_duration.num_seconds() as f64);
                }
                jobsrv::JobState::Failed => Counter::FailedJobs(target).increment(),
                _ => (),
            }

            match self.datastore.set_job_group_job_state(&job) {
                Ok(_) => {
                    if job.get_state() == jobsrv::JobState::Failed {
                        match self.skip_projects(&group, job.get_project().get_name()) {
                            Ok(_) => (),
                            Err(e) => {
                                self.log_error(&format!("Error skipping projects for {:?} \
                                                         (group: {}): {:?}",
                                                        job.get_project().get_name(),
                                                        job.get_owner_id(),
                                                        e));
                            }
                        };
                    }

                    match job.get_state() {
                        jobsrv::JobState::Complete
                        | jobsrv::JobState::Failed
                        | jobsrv::JobState::CancelComplete => {
                            self.update_group_state(job.get_owner_id())?
                        }

                        jobsrv::JobState::Pending
                        | jobsrv::JobState::Processing
                        | jobsrv::JobState::Dispatched
                        | jobsrv::JobState::CancelPending
                        | jobsrv::JobState::CancelProcessing
                        | jobsrv::JobState::Rejected => (),
                    }

                    // Unset the sync state
                    self.datastore.set_job_sync(job.get_id())?;
                }
                Err(err) => {
                    self.log_error(&format!("Failed to update job state for {} (group: {}): {:?}",
                                            job.get_project().get_name(),
                                            job.get_owner_id(),
                                            err))
                }
            }
        }

        Ok(())
    }

    fn update_group_state(&mut self, group_id: u64) -> Result<()> {
        let group = self.get_group(group_id)?;

        // Group state transition rules:
        // |   Start Group State     |  Projects State  |   New Group State   |
        // |-------------------------|------------------|---------------------|
        // |     Queued              |     N/A          |        N/A          |
        // |     Pending             |     N/A          |        N/A          |
        // |     Dispatching         |   no remaining   |      Complete       |
        // |     Dispatching         |   dispatchable?  |      Pending        |
        // |     Dispatching         |   otherwise      |      Dispatching    |
        // |     Complete            |     N/A          |        N/A          |
        // |     Failed              |     N/A          |        N/A          |

        if group.get_state() == jobsrv::JobGroupState::GroupDispatching {
            let mut failed = 0;
            let mut succeeded = 0;
            let mut skipped = 0;
            let mut canceled = 0;

            for project in group.get_projects() {
                match project.get_state() {
                    jobsrv::JobGroupProjectState::Failure => failed += 1,
                    jobsrv::JobGroupProjectState::Success => succeeded += 1,
                    jobsrv::JobGroupProjectState::Skipped => skipped += 1,
                    jobsrv::JobGroupProjectState::Canceled => canceled += 1,

                    jobsrv::JobGroupProjectState::NotStarted
                    | jobsrv::JobGroupProjectState::InProgress => (),
                }
            }

            let dispatchable = self.dispatchable_projects(&group)?;

            let new_state = if (succeeded + skipped + failed) == group.get_projects().len() {
                jobsrv::JobGroupState::GroupComplete
            } else if canceled > 0 {
                jobsrv::JobGroupState::GroupCanceled
            } else if !dispatchable.is_empty() {
                jobsrv::JobGroupState::GroupPending
            } else {
                jobsrv::JobGroupState::GroupDispatching
            };

            self.datastore.set_job_group_state(group_id, new_state)?;

            if new_state == jobsrv::JobGroupState::GroupPending {
                self.schedule_cli.notify()?;
            } else {
                // TODO: Make this cleaner later
                let mut updated_group = group;
                updated_group.set_state(new_state);
                self.logger.log_group(&updated_group);
            }
        } else {
            debug!("Skipping group update because state is {:?} for group id: {}",
                   group.get_state(),
                   group_id);
        }

        Ok(())
    }
}

fn buildable(project: &jobsrv::JobGroupProject) -> bool {
    matches!(project.get_state(),
             jobsrv::JobGroupProjectState::NotStarted | jobsrv::JobGroupProjectState::InProgress)
}
