use super::{metrics::Gauge,
            scheduler::ScheduleClient,
            scheduler2::Scheduler};
use crate::{bldr_core::{self,
                        job::Job,
                        metrics::GaugeMetric,
                        socket::DEFAULT_CONTEXT},
            config::Config,
            data_store::DataStore,
            db::{models::{integration::*,
                          jobs::*,
                          keys::*,
                          package::BuilderPackageTarget,
                          project_integration::*,
                          secrets::*},
                 DbPool},
            error::{Error,
                    Result},
            protocol::{jobsrv,
                       originsrv},
            scheduler_datastore::WorkerId};
use futures03::executor::block_on;
use habitat_core::{crypto::keys::{AnonymousBox,
                                  KeyCache,
                                  OriginSecretEncryptionKey},
                   package::{target,
                             PackageTarget}};
use linked_hash_map::LinkedHashMap;
use protobuf::{parse_from_bytes,
               Message,
               RepeatedField};
use std::{collections::HashSet,
          str::FromStr,
          sync::mpsc,
          thread::{self,
                   JoinHandle},
          time::{Duration,
                 Instant}};

const WORKER_MGR_ADDR: &str = "inproc://work-manager";
const WORKER_TIMEOUT_MS: u64 = 33_000; // 33 sec
const DEFAULT_POLL_TIMEOUT_MS: u64 = 60_000; // 60 secs
const JOB_TIMEOUT_CONVERT_MS: u64 = 60_000; // Conversion from mins to milli-seconds

pub struct WorkerMgrClient {
    socket: zmq::Socket,
}

// This is used as a one way channel to 'kick' the WorkerManager when new things arrive
// You'll see the basic pattern 'WorkerMgrClient::default().notify_work()?;'

impl WorkerMgrClient {
    pub fn connect(&mut self) -> Result<()> {
        self.socket.connect(WORKER_MGR_ADDR)?;
        Ok(())
    }

    pub fn notify_work(&mut self) -> Result<()> {
        self.socket.send(&[1], 0)?;
        Ok(())
    }
}

impl Default for WorkerMgrClient {
    fn default() -> WorkerMgrClient {
        let socket = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        socket.connect(WORKER_MGR_ADDR).unwrap();
        WorkerMgrClient { socket }
    }
}

#[derive(Debug)]
pub struct Worker {
    pub target:     PackageTarget,
    pub ident:      String,
    pub state:      jobsrv::WorkerState,
    pub expiry:     Instant,
    pub job_id:     Option<u64>,
    pub job_expiry: Option<Instant>,
    pub canceling:  bool,
}

impl Worker {
    pub fn new(ident: &str, target: PackageTarget) -> Self {
        Worker { target,
                 ident: ident.to_string(),
                 state: jobsrv::WorkerState::Ready,
                 expiry: Instant::now() + Duration::from_millis(WORKER_TIMEOUT_MS),
                 job_id: None,
                 job_expiry: None,
                 canceling: false }
    }

    pub fn ready(&mut self) {
        self.state = jobsrv::WorkerState::Ready;
        self.expiry = Instant::now() + Duration::from_millis(WORKER_TIMEOUT_MS);
        self.job_id = None;
        self.job_expiry = None;
        self.canceling = false;
    }

    pub fn busy(&mut self, job_id: u64, job_timeout: u64) {
        self.state = jobsrv::WorkerState::Busy;
        self.expiry = Instant::now() + Duration::from_millis(WORKER_TIMEOUT_MS);
        self.canceling = false;

        if self.job_id.is_none() {
            self.job_id = Some(job_id);
            self.job_expiry =
                Some(Instant::now() + Duration::from_millis(job_timeout * JOB_TIMEOUT_CONVERT_MS));
        } else {
            assert!(self.job_id.unwrap() == job_id);
        }
    }

    pub fn cancel(&mut self) { self.canceling = true; }

    pub fn is_canceling(&self) -> bool { self.canceling }

    pub fn refresh(&mut self) {
        self.expiry = Instant::now() + Duration::from_millis(WORKER_TIMEOUT_MS);
    }

    pub fn is_expired(&self) -> bool { self.expiry < Instant::now() }

    pub fn is_job_expired(&self) -> bool {
        if self.job_expiry.is_some() {
            self.job_expiry.unwrap() < Instant::now()
        } else {
            false
        }
    }
}

pub struct WorkerMgr {
    datastore:        DataStore,
    db:               DbPool,
    /// Location of Builder encryption keys
    key_cache:        KeyCache,
    hb_sock:          zmq::Socket,
    rq_sock:          zmq::Socket,
    work_mgr_sock:    zmq::Socket,
    msg:              zmq::Message,
    workers:          LinkedHashMap<String, Worker>,
    worker_command:   String,
    worker_heartbeat: String,
    schedule_cli:     ScheduleClient,
    job_timeout:      u64,
    build_targets:    HashSet<PackageTarget>,
    scheduler:        Option<Scheduler>,
}

impl WorkerMgr {
    pub fn new(cfg: &Config,
               datastore: &DataStore,
               db: DbPool,
               scheduler: Option<Scheduler>)
               -> Self {
        let hb_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::SUB).unwrap();
        let rq_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::ROUTER).unwrap();
        let work_mgr_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        rq_sock.set_router_mandatory(true).unwrap();
        hb_sock.set_subscribe(&[]).unwrap();

        let mut schedule_cli = ScheduleClient::default();
        schedule_cli.connect().unwrap();

        WorkerMgr { datastore: datastore.clone(),
                    db,
                    // cfg is hydrated from a TOML file, and the
                    // `key_dir` name is currently part of that
                    // interface. `WorkerMgr` is fully private,
                    // though, so we can actually freely name this
                    // `key_cache`.
                    key_cache: cfg.key_dir.clone(),
                    hb_sock,
                    rq_sock,
                    work_mgr_sock,
                    msg: zmq::Message::new().unwrap(),
                    workers: LinkedHashMap::new(),
                    worker_command: cfg.net.worker_command_addr(),
                    worker_heartbeat: cfg.net.worker_heartbeat_addr(),
                    schedule_cli,
                    job_timeout: cfg.job_timeout,
                    build_targets: cfg.build_targets.clone(),
                    scheduler }
    }

    pub fn start(cfg: &Config,
                 datastore: &DataStore,
                 db: DbPool,
                 scheduler: Option<Scheduler>)
                 -> Result<JoinHandle<()>> {
        let mut manager = Self::new(cfg, datastore, db, scheduler);
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = thread::Builder::new().name("worker-manager".to_string())
                                           .spawn(move || {
                                               manager.run(&tx).unwrap();
                                           })
                                           .unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("worker-manager thread startup error, err={}", e),
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn run(&mut self, rz: &mpsc::SyncSender<()>) -> Result<()> {
        self.work_mgr_sock.bind(WORKER_MGR_ADDR)?;
        println!("Listening for commands on {}", self.worker_command);
        self.rq_sock.bind(&self.worker_command)?;
        println!("Listening for heartbeats on {}", self.worker_heartbeat);
        self.hb_sock.bind(&self.worker_heartbeat)?;
        let mut hb_sock = false;
        let mut rq_sock = false;
        let mut work_mgr_sock = false;
        let mut process_work = false;
        let mut last_processed = Instant::now();

        rz.send(()).unwrap();

        // Load busy worker state
        self.load_workers()?;

        // Re-queue any Dispatched jobs that don't have a busy worker
        self.requeue_jobs()?;

        info!("builder-jobsrv is ready to go.");

        loop {
            {
                let mut items = [self.hb_sock.as_poll_item(1),
                                 self.rq_sock.as_poll_item(1),
                                 self.work_mgr_sock.as_poll_item(1)];

                if let Err(err) = zmq::poll(&mut items, DEFAULT_POLL_TIMEOUT_MS as i64) {
                    warn!("Worker-manager unable to complete ZMQ poll: err {:?}", err);
                };
                if (items[0].get_revents() & zmq::POLLIN) > 0 {
                    hb_sock = true;
                }
                if (items[1].get_revents() & zmq::POLLIN) > 0 {
                    rq_sock = true;
                }
                if (items[2].get_revents() & zmq::POLLIN) > 0 {
                    work_mgr_sock = true;
                }
            }

            if hb_sock {
                if let Err(err) = self.process_heartbeat() {
                    warn!("Worker-manager unable to process heartbeat: err {:?}", err);
                };
                hb_sock = false;
            }
            if let Err(err) = self.expire_workers() {
                warn!("Worker-manager unable to expire workers: err {:?}", err);
            }
            if rq_sock {
                if let Err(err) = self.process_job_status() {
                    warn!("Worker-manager unable to process job status: err {:?}", err);
                }
                rq_sock = false;
            }
            if work_mgr_sock {
                process_work = true;
                work_mgr_sock = false;

                if let Err(err) = self.work_mgr_sock.recv(&mut self.msg, 0) {
                    warn!("Worker-manager unable to complete socket receive: err {:?}",
                          err);
                }
            }

            // Handle potential work in pending_jobs queue
            let now = Instant::now();
            if process_work
               || (now > (last_processed + Duration::from_millis(DEFAULT_POLL_TIMEOUT_MS)))
            {
                if let Err(err) = self.process_cancelations() {
                    warn!("Worker-manager unable to process cancels: err {:?}", err);
                }

                for target in PackageTarget::targets() {
                    if self.build_targets.contains(&target) {
                        if let Err(err) = self.process_work(*target) {
                            warn!("Worker-manager unable to process work: err {:?}", err);
                        }
                    }
                }
                last_processed = now;
            }

            for target in PackageTarget::targets() {
                if self.build_targets.contains(target) {
                    if let Err(err) = self.process_metrics(*target) {
                        warn!("Worker-manager unable to process metrics: err {:?}", err);
                    }
                }
            }
        }
    }

    fn load_workers(&mut self) -> Result<()> {
        let conn = self.db.get_conn().map_err(Error::Db)?;
        let workers = BusyWorker::list(&*conn).map_err(Error::DieselError)?;

        for worker in workers {
            debug!("Loading busy worker: {}", worker.ident);
            let target = PackageTarget::from_str(&worker.target)?;
            let mut bw = Worker::new(&worker.ident, target);
            bw.busy(worker.job_id as u64, self.job_timeout);
            self.workers.insert(worker.ident.to_owned(), bw);
        }

        Ok(())
    }

    fn save_worker(&mut self, worker: &Worker) -> Result<()> {
        debug!("Saving busy worker: {}", worker.ident);
        let conn = self.db.get_conn().map_err(Error::Db)?;

        BusyWorker::create(&NewBusyWorker { target:      &worker.target.to_string(),
                                            ident:       &worker.ident,
                                            job_id:      worker.job_id.unwrap() as i64,
                                            quarantined: false, },
                           &*conn).map_err(Error::DieselError)?;

        Ok(())
    }

    fn delete_worker(&mut self, worker: &Worker) -> Result<()> {
        debug!("Deleting busy worker: {}", worker.ident);
        let conn = self.db.get_conn().map_err(Error::Db)?;

        BusyWorker::delete(&worker.ident, worker.job_id.unwrap() as i64, &*conn)
            .map_err(Error::DieselError)?;

        Ok(())
    }

    // TODO This will need to communicate with scheduler to update job on it's side.
    fn requeue_jobs(&mut self) -> Result<()> {
        let jobs = self.datastore.get_dispatched_jobs()?;

        for mut job in jobs {
            if self.workers
                   .iter()
                   .find(|t| t.1.job_id == Some(job.get_id()))
                   .is_none()
            {
                warn!("Requeing job: {}", job.get_id());
                job.set_state(jobsrv::JobState::Pending);
                self.datastore.update_job(&job)?;
            };
        }

        Ok(())
    }

    fn process_metrics(&mut self, target: PackageTarget) -> Result<()> {
        Gauge::Workers(target).set(self.workers
                                       .iter()
                                       .filter(|t| (t.1.target == target))
                                       .count() as f64);

        let ready_workers =
            self.workers
                .iter()
                .filter(|t| (t.1.target == target) && (t.1.state == jobsrv::WorkerState::Ready))
                .count();

        let busy_workers =
            self.workers
                .iter()
                .filter(|t| (t.1.target == target) && (t.1.state == jobsrv::WorkerState::Busy))
                .count();

        Gauge::ReadyWorkers(target).set(ready_workers as f64);
        Gauge::BusyWorkers(target).set(busy_workers as f64);

        Ok(())
    }

    fn process_cancelations(&mut self) -> Result<()> {
        // Get the cancel-pending jobs list
        let jobs = self.datastore.get_cancel_pending_jobs()?;

        if !jobs.is_empty() {
            debug!("process_cancelations: Found {} cancels", jobs.len());
        }

        for job in jobs {
            let mut job = Job::new(job);

            // Find the worker processing this job
            // TODO (SA): Would be nice not doing an iterative search here
            let worker_ident = match self.workers
                                         .iter()
                                         .find(|t| t.1.job_id == Some(job.get_id()))
            {
                Some(t) => t.0.clone(),
                None => {
                    warn!("Did not find any workers with job id: {}", job.get_id());
                    job.set_state(jobsrv::JobState::CancelComplete);
                    self.datastore.update_job(&job)?;
                    continue;
                }
            };

            match self.worker_cancel_job(&job, &worker_ident) {
                Ok(()) => {
                    job.set_state(jobsrv::JobState::CancelProcessing);
                    self.datastore.update_job(&job)?;
                }
                Err(err) => {
                    warn!("Failed to cancel job on worker {}, err={:?}",
                          worker_ident, err);
                    job.set_state(jobsrv::JobState::CancelComplete);
                    self.datastore.update_job(&job)?;
                }
            }
        }

        Ok(())
    }

    fn worker_cancel_job(&mut self, job: &Job, worker_ident: &str) -> Result<()> {
        debug!("Canceling job on worker {:?}: {:?}", worker_ident, job);

        let mut wc = jobsrv::WorkerCommand::new();
        wc.set_op(jobsrv::WorkerOperation::CancelJob);

        self.rq_sock.send_str(&worker_ident, zmq::SNDMORE)?;
        self.rq_sock.send(&[], zmq::SNDMORE)?;
        self.rq_sock
            .send(&wc.write_to_bytes().unwrap(), zmq::SNDMORE)?;
        self.rq_sock.send(&job.write_to_bytes().unwrap(), 0)?;

        Ok(())
    }

    fn process_work(&mut self, target: PackageTarget) -> Result<()> {
        loop {
            // Exit if we don't have any Ready workers
            let worker_ident =
                match self.workers
                          .iter()
                          .find(|t| {
                              (t.1.target == target) && (t.1.state == jobsrv::WorkerState::Ready)
                          }) {
                    Some(t) => t.0.clone(),
                    None => return Ok(()),
                };

            // Take one job from the pending list
            // TODO This will need to communicate with scheduler to update job on it's side.
            let mut job = if let Some(scheduler) = &mut self.scheduler {
                // Runtime::new().unwrap().block_on(|| worker_needs_work.await )
                if let Some(job_entry) =
                    block_on(scheduler.worker_needs_work(WorkerId(worker_ident),
                                                         BuilderPackageTarget(target)))
                {
                    job_entry.into()
                } else {
                    break;
                }
            } else {
                let job_opt = self.datastore
                                  .next_pending_job(&worker_ident, &target.to_string())?;
                if job_opt.is_none() {
                    break;
                }

                Job::new(job_opt.unwrap()) // unwrap Ok
            };

            self.add_integrations_to_job(&mut job);
            self.add_project_integrations_to_job(&mut job);
            self.add_secrets_to_job(&mut job)?;

            match self.worker_start_job(&job, &worker_ident) {
                Ok(()) => {
                    let mut worker = self.workers.remove(&worker_ident).unwrap(); // unwrap Ok
                    worker.busy(job.get_id(), self.job_timeout);
                    self.save_worker(&worker)?;
                    self.workers.insert(worker_ident, worker);
                }
                Err(err) => {
                    warn!("Failed to dispatch job to worker {}, err={:?}",
                          worker_ident, err);
                    job.set_state(jobsrv::JobState::Pending); // TODO sched2 needs to update job_graph_entry/scheduler with state
                    self.datastore.update_job(&job)?;
                    return Ok(()); // Exit instead of re-trying immediately
                }
            }
        }
        Ok(())
    }

    fn worker_start_job(&mut self, job: &Job, worker_ident: &str) -> Result<()> {
        debug!("Dispatching job to worker {:?}: {:?}", worker_ident, job);

        let mut wc = jobsrv::WorkerCommand::new();
        wc.set_op(jobsrv::WorkerOperation::StartJob);

        self.rq_sock.send_str(&worker_ident, zmq::SNDMORE)?;
        self.rq_sock.send(&[], zmq::SNDMORE)?;
        self.rq_sock
            .send(&wc.write_to_bytes().unwrap(), zmq::SNDMORE)?;
        self.rq_sock.send(&job.write_to_bytes().unwrap(), 0)?;

        Ok(())
    }

    fn add_integrations_to_job(&mut self, job: &mut Job) {
        let mut integrations = RepeatedField::new();
        let origin = job.get_project().get_origin_name().to_string();

        let conn = match self.db.get_conn().map_err(Error::Db) {
            Ok(conn) => conn,
            Err(_) => return,
        };

        match OriginIntegration::list_for_origin(&origin, &*conn).map_err(Error::DieselError) {
            Ok(oir) => {
                for i in oir {
                    let mut oi = originsrv::OriginIntegration::new();
                    let plaintext = match bldr_core::crypto::decrypt(&self.key_cache, &i.body) {
                        Ok(b) => {
                            match String::from_utf8(b) {
                                Ok(s) => s,
                                Err(e) => {
                                    debug!("Error converting to string. e = {:?}", e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Error decrypting integration. e = {:?}", e);
                            continue;
                        }
                    };
                    oi.set_origin(i.origin);
                    oi.set_integration(i.integration);
                    oi.set_name(i.name);
                    oi.set_body(plaintext);
                    integrations.push(oi);
                }

                job.set_integrations(integrations);
            }
            Err(e) => {
                debug!("Error fetching integrations. e = {:?}", e);
            }
        }
    }

    fn add_project_integrations_to_job(&mut self, job: &mut Job) {
        let mut integrations = RepeatedField::new();
        let origin = job.get_project().get_origin_name().to_string();
        let name = job.get_project().get_package_name().to_string();

        let conn = match self.db.get_conn().map_err(Error::Db) {
            Ok(conn) => conn,
            Err(_) => return,
        };

        match ProjectIntegration::list(&origin, &name, &*conn).map_err(Error::DieselError) {
            Ok(opir) => {
                for opi in opir {
                    integrations.push(opi.into());
                }
                job.set_project_integrations(integrations);
            }
            Err(e) => {
                debug!("Error fetching project integrations. e = {:?}", e);
            }
        }
    }

    fn add_secrets_to_job(&mut self, job: &mut Job) -> Result<()> {
        let origin = job.get_project().get_origin_name().to_string();
        let conn = self.db.get_conn().map_err(Error::Db)?;

        let mut secrets = RepeatedField::new();

        match OriginSecret::list(&origin, &*conn).map_err(Error::DieselError) {
            Ok(secrets_list) => {
                if !secrets_list.is_empty() {
                    // fetch the private origin encryption key from the database
                    let priv_key = match OriginPrivateEncryptionKey::get(&origin, &*conn)
                        .map_err(Error::DieselError)
                    {
                        Ok(key) => {
                            key.body.parse::<OriginSecretEncryptionKey>()?
                        }
                        Err(err) => return Err(err),
                    };

                    for secret in secrets_list {
                        debug!("Adding secret to job: {:?}", secret);
                        match secret.value.parse::<AnonymousBox>() {
                            Ok(anonymous_box) => {
                                match priv_key.decrypt(&anonymous_box) {
                                    Ok(decrypted_secret) => {
                                        let mut secret_decrypted = originsrv::OriginSecret::new();
                                        let mut secret_decrypted_wrapper =
                                            originsrv::OriginSecretDecrypted::new();

                                        secret_decrypted.set_id(secret.id as u64);
                                        secret_decrypted.set_origin(secret.origin);
                                        secret_decrypted.set_name(secret.name.to_string());
                                        secret_decrypted.set_value(
                                            String::from_utf8(decrypted_secret)?,
                                        );
                                        secret_decrypted_wrapper
                                            .set_decrypted_secret(secret_decrypted);
                                        secrets.push(secret_decrypted_wrapper);
                                    }
                                    Err(e) => {
                                        warn!("Unable to add secret to job: {}", e);
                                        continue;
                                    }
                                };
                            }
                            Err(e) => {
                                warn!("Failed to decrypt secret: {}", e);
                                continue;
                            }
                        };
                    }
                }
                job.set_secrets(secrets);
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }

    fn expire_workers(&mut self) -> Result<()> {
        loop {
            if let Some(worker) = self.workers.front() {
                if !worker.1.is_expired() {
                    break;
                }
            } else {
                break;
            }

            let worker = self.workers.pop_front().unwrap().1;
            warn!("Expiring worker due to missed heartbeat: {:?}", worker);

            // TODO: There may be a possible corner case here that
            // a worker can have work assigned, but not be Busy.
            if worker.state == jobsrv::WorkerState::Busy {
                self.requeue_job(worker.job_id.unwrap())?; // unwrap Ok
                self.delete_worker(&worker)?;
            }
        }

        Ok(())
    }

    // This will need to communicate with scheduler to update job on it's side.
    fn requeue_job(&mut self, job_id: u64) -> Result<()> {
        let mut req = jobsrv::JobGet::new();
        req.set_id(job_id);

        match self.datastore.get_job(&req)? {
            Some(mut job) => {
                match job.get_state() {
                    jobsrv::JobState::Processing | jobsrv::JobState::Dispatched => {
                        debug!("Requeing job {:?}", job_id);
                        job.set_state(jobsrv::JobState::Pending);
                        self.datastore.update_job(&job)?;
                    }
                    jobsrv::JobState::CancelPending | jobsrv::JobState::CancelProcessing => {
                        debug!("Marking orhpaned job as canceled: {:?}", job_id);
                        job.set_state(jobsrv::JobState::CancelComplete);
                        self.datastore.update_job(&job)?;
                    }
                    jobsrv::JobState::Pending
                    | jobsrv::JobState::Complete
                    | jobsrv::JobState::Failed
                    | jobsrv::JobState::CancelComplete
                    | jobsrv::JobState::Rejected => (),
                }
            }
            None => {
                warn!("Unable to requeue job {:?} (not found)", job_id,);
            }
        }

        Ok(())
    }

    // This will need to communicate with scheduler to update job on it's side. TBI
    fn cancel_job(&mut self, job_id: u64, worker_ident: &str) -> Result<()> {
        let mut req = jobsrv::JobGet::new();
        req.set_id(job_id);

        match self.datastore.get_job(&req)? {
            Some(job) => {
                let mut job = Job::new(job);
                match self.worker_cancel_job(&job, &worker_ident) {
                    Ok(()) => {
                        job.set_state(jobsrv::JobState::CancelProcessing);
                        self.datastore.update_job(&job)?;
                    }
                    Err(err) => {
                        warn!("Failed to cancel job on worker {}, err={:?}",
                              worker_ident, err);
                        job.set_state(jobsrv::JobState::CancelComplete);
                        self.datastore.update_job(&job)?;
                    }
                }
            }
            None => {
                warn!("Unable to cancel job {:?} (not found)", job_id,);
            }
        };

        Ok(())
    }

    fn is_job_complete(&self, job_id: u64) -> Result<bool> {
        let mut req = jobsrv::JobGet::new();
        req.set_id(job_id);

        let ret = match self.datastore.get_job(&req)? {
            Some(job) => {
                match job.get_state() {
                    jobsrv::JobState::Pending
                    | jobsrv::JobState::Processing
                    | jobsrv::JobState::Dispatched
                    | jobsrv::JobState::CancelPending
                    | jobsrv::JobState::CancelProcessing => false,

                    jobsrv::JobState::Complete
                    | jobsrv::JobState::Failed
                    | jobsrv::JobState::CancelComplete
                    | jobsrv::JobState::Rejected => true,
                }
            }
            None => {
                warn!("Unable to check job completeness {:?} (not found)", job_id,);
                false
            }
        };

        Ok(ret)
    }

    fn process_heartbeat(&mut self) -> Result<()> {
        self.hb_sock.recv(&mut self.msg, 0)?;
        let heartbeat: jobsrv::Heartbeat = parse_from_bytes(&self.msg)?;
        trace!("Got heartbeat: {:?}", heartbeat);

        let worker_ident = heartbeat.get_endpoint().to_string();

        let mut worker = match self.workers.remove(&worker_ident) {
            Some(worker) => worker,
            None => {
                info!("New worker detected, heartbeat: {:?}", heartbeat);
                let worker_target = match PackageTarget::from_str(heartbeat.get_target()) {
                    Ok(t) => t,
                    Err(_) => target::X86_64_LINUX,
                };

                if heartbeat.get_state() == jobsrv::WorkerState::Ready {
                    Worker::new(&worker_ident, worker_target)
                } else {
                    warn!("Unexpected Busy heartbeat from unknown worker {}",
                          worker_ident);
                    return Ok(()); // Something went wrong, don't process this HB
                }
            }
        };

        match (worker.state, heartbeat.get_state()) {
            (jobsrv::WorkerState::Ready, jobsrv::WorkerState::Busy) => {
                warn!("Unexpected Busy heartbeat from known worker {}",
                      worker_ident);
                return Ok(()); // Something went wrong, don't process this HB
            }
            (jobsrv::WorkerState::Busy, jobsrv::WorkerState::Busy) => {
                let job_id = worker.job_id.unwrap(); // unwrap Ok
                if worker.is_job_expired() && !worker.is_canceling() {
                    warn!("Canceling job due to timeout: {}, {:?}", job_id, worker);
                    self.cancel_job(job_id, &worker_ident)?;
                    worker.cancel();
                };
                worker.refresh();
            }
            (jobsrv::WorkerState::Busy, jobsrv::WorkerState::Ready) => {
                if !self.is_job_complete(worker.job_id.unwrap())? {
                    // Handle potential race condition where a Ready heartbeat
                    // is received right *after* the job has been dispatched
                    warn!("Unexpected Ready heartbeat from incomplete job: {}, {:?}",
                          worker.job_id.unwrap(),
                          worker);
                    worker.refresh();
                } else {
                    self.delete_worker(&worker)?;
                    worker.ready();
                }
            }
            _ => worker.ready(),
        };

        assert!(!worker.is_expired());
        self.workers.insert(worker_ident, worker);
        Ok(())
    }

    fn process_job_status(&mut self) -> Result<()> {
        self.rq_sock.recv(&mut self.msg, 0)?;
        self.rq_sock.recv(&mut self.msg, 0)?;

        let job = Job::new(parse_from_bytes::<jobsrv::Job>(&self.msg)?);
        debug!("Got job status: {:?}", job);
        self.datastore.update_job(&job)?;
        self.schedule_cli.notify()?;

        Ok(())
    }
}
