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

use std::path::PathBuf;
use std::str::from_utf8;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use bldr_core;
use bldr_core::job::Job;
use bldr_core::metrics::GaugeMetric;
use db::DbPool;
use hab_core::crypto::keys::{parse_key_str, parse_name_with_rev};
use hab_core::crypto::BoxKeyPair;
use hab_net::socket::DEFAULT_CONTEXT;
use linked_hash_map::LinkedHashMap;
use protobuf::{parse_from_bytes, Message, RepeatedField};

use db::models::integration::*;
use db::models::keys::*;
use db::models::origin::*;
use db::models::project_integration::*;
use db::models::secrets::*;

use protocol::jobsrv;
use protocol::originsrv;

use zmq;

use config::Config;
use data_store::DataStore;
use error::{Error, Result};

use super::metrics::Gauge;
use super::scheduler::ScheduleClient;

const WORKER_MGR_ADDR: &'static str = "inproc://work-manager";
const WORKER_TIMEOUT_MS: u64 = 33_000; // 33 sec
const DEFAULT_POLL_TIMEOUT_MS: u64 = 60_000; // 60 secs
const JOB_TIMEOUT_CONVERT_MS: u64 = 60_000; // Conversion from mins to milli-seconds

pub struct WorkerMgrClient {
    socket: zmq::Socket,
}

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
        WorkerMgrClient { socket: socket }
    }
}

#[derive(Debug)]
pub struct Worker {
    pub ident: String,
    pub state: jobsrv::WorkerState,
    pub expiry: Instant,
    pub job_id: Option<u64>,
    pub job_expiry: Option<Instant>,
    pub canceling: bool,
}

impl Worker {
    pub fn new(ident: &str) -> Self {
        Worker {
            ident: ident.to_string(),
            state: jobsrv::WorkerState::Ready,
            expiry: Instant::now() + Duration::from_millis(WORKER_TIMEOUT_MS),
            job_id: None,
            job_expiry: None,
            canceling: false,
        }
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

    pub fn cancel(&mut self) {
        self.canceling = true;
    }

    pub fn is_canceling(&mut self) -> bool {
        self.canceling
    }

    pub fn refresh(&mut self) {
        self.expiry = Instant::now() + Duration::from_millis(WORKER_TIMEOUT_MS);
    }

    pub fn is_expired(&self) -> bool {
        self.expiry < Instant::now()
    }

    pub fn is_job_expired(&self) -> bool {
        if self.job_expiry.is_some() {
            self.job_expiry.unwrap() < Instant::now()
        } else {
            false
        }
    }
}

pub struct WorkerMgr {
    datastore: DataStore,
    db: DbPool,
    key_dir: PathBuf,
    hb_sock: zmq::Socket,
    rq_sock: zmq::Socket,
    work_mgr_sock: zmq::Socket,
    msg: zmq::Message,
    workers: LinkedHashMap<String, Worker>,
    worker_command: String,
    worker_heartbeat: String,
    schedule_cli: ScheduleClient,
    job_timeout: u64,
}

impl WorkerMgr {
    pub fn new(cfg: &Config, datastore: &DataStore, db: DbPool) -> Result<Self> {
        let hb_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::SUB)?;
        let rq_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::ROUTER)?;
        let work_mgr_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER)?;
        rq_sock.set_router_mandatory(true)?;
        hb_sock.set_subscribe(&[])?;

        let mut schedule_cli = ScheduleClient::default();
        schedule_cli.connect()?;

        Ok(WorkerMgr {
            datastore: datastore.clone(),
            db: db,
            key_dir: cfg.key_dir.clone(),
            hb_sock: hb_sock,
            rq_sock: rq_sock,
            work_mgr_sock: work_mgr_sock,
            msg: zmq::Message::new()?,
            workers: LinkedHashMap::new(),
            worker_command: cfg.net.worker_command_addr(),
            worker_heartbeat: cfg.net.worker_heartbeat_addr(),
            schedule_cli: schedule_cli,
            job_timeout: cfg.job_timeout,
        })
    }

    pub fn start(cfg: &Config, datastore: &DataStore, db: DbPool) -> Result<JoinHandle<()>> {
        let mut manager = Self::new(cfg, datastore, db)?;
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = thread::Builder::new()
            .name("worker-manager".to_string())
            .spawn(move || {
                manager.run(tx).unwrap();
            }).unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("worker-manager thread startup error, err={}", e),
        }
    }

    fn run(&mut self, rz: mpsc::SyncSender<()>) -> Result<()> {
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
                let mut items = [
                    self.hb_sock.as_poll_item(1),
                    self.rq_sock.as_poll_item(1),
                    self.work_mgr_sock.as_poll_item(1),
                ];

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
                    warn!(
                        "Worker-manager unable to complete socket receive: err {:?}",
                        err
                    );
                }
            }

            // Handle potential work in pending_jobs queue
            let now = Instant::now();
            if process_work
                || (&now > &(last_processed + Duration::from_millis(DEFAULT_POLL_TIMEOUT_MS)))
            {
                if let Err(err) = self.process_cancelations() {
                    warn!("Worker-manager unable to process cancels: err {:?}", err);
                }
                if let Err(err) = self.process_work() {
                    warn!("Worker-manager unable to process work: err {:?}", err);
                }
                last_processed = now;
            }

            if let Err(err) = self.process_metrics() {
                warn!("Worker-manager unable to process metrics: err {:?}", err);
            }
        }
    }

    fn load_workers(&mut self) -> Result<()> {
        let workers = self.datastore.get_busy_workers()?;

        for worker in workers {
            let mut bw = Worker::new(worker.get_ident());
            bw.busy(worker.get_job_id(), self.job_timeout);
            self.workers.insert(worker.get_ident().to_owned(), bw);
        }

        Ok(())
    }

    fn save_worker(&mut self, worker: &Worker) -> Result<()> {
        let mut bw = jobsrv::BusyWorker::new();
        bw.set_ident(worker.ident.clone());
        bw.set_job_id(worker.job_id.unwrap()); // unwrap Ok

        self.datastore.upsert_busy_worker(&bw)
    }

    fn delete_worker(&mut self, worker: &Worker) -> Result<()> {
        let mut bw = jobsrv::BusyWorker::new();
        bw.set_ident(worker.ident.clone());
        bw.set_job_id(worker.job_id.unwrap()); // unwrap Ok

        self.datastore.delete_busy_worker(&bw)
    }

    fn requeue_jobs(&mut self) -> Result<()> {
        let jobs = self.datastore.get_dispatched_jobs()?;

        for mut job in jobs {
            if self
                .workers
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

    fn process_metrics(&mut self) -> Result<()> {
        Gauge::Workers.set(self.workers.len() as f64);

        let ready_workers = self
            .workers
            .iter()
            .filter(|t| t.1.state == jobsrv::WorkerState::Ready)
            .count();

        let busy_workers = self
            .workers
            .iter()
            .filter(|t| t.1.state == jobsrv::WorkerState::Busy)
            .count();

        Gauge::ReadyWorkers.set(ready_workers as f64);
        Gauge::BusyWorkers.set(busy_workers as f64);

        Ok(())
    }

    fn process_cancelations(&mut self) -> Result<()> {
        // Get the cancel-pending jobs list
        let jobs = self.datastore.get_cancel_pending_jobs()?;

        if jobs.len() > 0 {
            debug!("process_cancelations: Found {} cancels", jobs.len());
        }

        for job in jobs {
            let mut job = Job::new(job);

            // Find the worker processing this job
            // TODO (SA): Would be nice not doing an iterative search here
            let worker_ident = match self
                .workers
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
                    warn!(
                        "Failed to cancel job on worker {}, err={:?}",
                        worker_ident, err
                    );
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

    fn process_work(&mut self) -> Result<()> {
        loop {
            // Exit if we don't have any Ready workers
            let worker_ident = match self
                .workers
                .iter()
                .find(|t| t.1.state == jobsrv::WorkerState::Ready)
            {
                Some(t) => t.0.clone(),
                None => return Ok(()),
            };

            // Take one job from the pending list
            let job_opt = self.datastore.next_pending_job(&worker_ident)?;
            if job_opt.is_none() {
                break;
            }

            let mut job = Job::new(job_opt.unwrap()); // unwrap Ok

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
                    warn!(
                        "Failed to dispatch job to worker {}, err={:?}",
                        worker_ident, err
                    );
                    job.set_state(jobsrv::JobState::Pending);
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
                    let plaintext = match bldr_core::integrations::decrypt(&self.key_dir, &i.body) {
                        Ok(b) => match String::from_utf8(b) {
                            Ok(s) => s,
                            Err(e) => {
                                debug!("Error converting to string. e = {:?}", e);
                                continue;
                            }
                        },
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

        // TODO (SA) - we shouldn't need to pull the origin here to just get the id
        let origin_id = match Origin::get(&origin, &*conn) {
            Ok(origin) => origin.id,
            Err(err) => return Err(Error::DieselError(err)),
        };

        let mut secrets = RepeatedField::new();

        match OriginSecret::list(origin_id as i64, &*conn).map_err(Error::DieselError) {
            Ok(secrets_list) => {
                if secrets_list.len() > 0 {
                    // fetch the private origin encryption key from the database
                    let priv_key = match OriginPrivateEncryptionKey::get(&origin, &*conn)
                        .map_err(Error::DieselError)
                    {
                        Ok(key) => {
                            let key_str = from_utf8(&key.body).unwrap();
                            BoxKeyPair::secret_key_from_str(key_str)?
                        }
                        Err(err) => return Err(err),
                    };

                    // fetch the public origin encryption key from the database
                    let (name, rev, pub_key) =
                        match OriginPublicEncryptionKey::latest(&origin, &*conn)
                            .map_err(Error::DieselError)
                        {
                            Ok(key) => {
                                let key_str = from_utf8(&key.body).unwrap();
                                let (name, rev) = match parse_key_str(key_str) {
                                    Ok((_, name_with_rev, _)) => {
                                        parse_name_with_rev(name_with_rev)?
                                    }
                                    Err(e) => return Err(Error::HabitatCore(e)),
                                };
                                (name, rev, BoxKeyPair::public_key_from_str(key_str)?)
                            }
                            Err(err) => return Err(err),
                        };

                    let box_key_pair =
                        BoxKeyPair::new(name, rev.clone(), Some(pub_key), Some(priv_key));
                    for secret in secrets_list {
                        debug!("Adding secret to job: {:?}", secret);
                        let mut secret_decrypted = originsrv::OriginSecret::new();
                        let mut secret_decrypted_wrapper = originsrv::OriginSecretDecrypted::new();
                        match BoxKeyPair::secret_metadata(secret.value.as_bytes()) {
                            Ok(secret_metadata) => {
                                match box_key_pair.decrypt(&secret_metadata.ciphertext, None, None)
                                {
                                    Ok(decrypted_secret) => {
                                        secret_decrypted.set_id(secret.id as u64);
                                        secret_decrypted.set_origin_id(secret.origin_id as u64);
                                        secret_decrypted.set_name(secret.name.to_string());
                                        secret_decrypted.set_value(
                                            String::from_utf8(decrypted_secret).unwrap(),
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
                                warn!("Failed to get metadata from secret: {}", e);
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
            debug!("Expiring worker due to missed heartbeat: {:?}", worker);

            if worker.state == jobsrv::WorkerState::Busy {
                self.requeue_job(worker.job_id.unwrap())?; // unwrap Ok
                self.delete_worker(&worker)?;
            }
        }

        Ok(())
    }

    fn requeue_job(&mut self, job_id: u64) -> Result<()> {
        let mut req = jobsrv::JobGet::new();
        req.set_id(job_id);

        match self.datastore.get_job(&req)? {
            Some(mut job) => match job.get_state() {
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
            },
            None => {
                warn!("Unable to requeue job {:?} (not found)", job_id,);
            }
        }

        Ok(())
    }

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
                        warn!(
                            "Failed to cancel job on worker {}, err={:?}",
                            worker_ident, err
                        );
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

    fn is_job_complete(&mut self, job_id: u64) -> Result<bool> {
        let mut req = jobsrv::JobGet::new();
        req.set_id(job_id);

        let ret = match self.datastore.get_job(&req)? {
            Some(job) => match job.get_state() {
                jobsrv::JobState::Pending
                | jobsrv::JobState::Processing
                | jobsrv::JobState::Dispatched
                | jobsrv::JobState::CancelPending
                | jobsrv::JobState::CancelProcessing => false,

                jobsrv::JobState::Complete
                | jobsrv::JobState::Failed
                | jobsrv::JobState::CancelComplete
                | jobsrv::JobState::Rejected => true,
            },
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
        debug!("Got heartbeat: {:?}", heartbeat);

        let worker_ident = heartbeat.get_endpoint().to_string();

        let mut worker = match self.workers.remove(&worker_ident) {
            Some(worker) => worker,
            None => {
                if heartbeat.get_state() == jobsrv::WorkerState::Ready {
                    Worker::new(&worker_ident)
                } else {
                    warn!(
                        "Unexpacted Busy heartbeat from unknown worker {}",
                        worker_ident
                    );
                    return Ok(()); // Something went wrong, don't process this HB
                }
            }
        };

        match (worker.state, heartbeat.get_state()) {
            (jobsrv::WorkerState::Ready, jobsrv::WorkerState::Busy) => {
                warn!(
                    "Unexpected Busy heartbeat from known worker {}",
                    worker_ident
                );
                return Ok(()); // Something went wrong, don't process this HB
            }
            (jobsrv::WorkerState::Busy, jobsrv::WorkerState::Busy) => {
                let job_id = worker.job_id.unwrap(); // unwrap Ok
                if worker.is_job_expired() && !worker.is_canceling() {
                    debug!("Canceling job due to timeout: {}", job_id);
                    self.cancel_job(job_id, &worker_ident)?;
                    worker.cancel();
                };
                worker.refresh();
            }
            (jobsrv::WorkerState::Busy, jobsrv::WorkerState::Ready) => {
                if !self.is_job_complete(worker.job_id.unwrap())? {
                    // Handle potential race condition where a Ready heartbeat
                    // is received right *after* the job has been dispatched
                    warn!(
                        "Unexpected Ready heartbeat from incomplete job: {}",
                        worker.job_id.unwrap()
                    );
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
