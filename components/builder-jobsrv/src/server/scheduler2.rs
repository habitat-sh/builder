// Copyright (c) 2020 Chef Software Inc. and/or applicable contributors
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

use std::{fmt,
          str::FromStr};

use tokio::{sync::{mpsc,
                   oneshot},
            task::JoinHandle,
            time::interval};

use crate::{config::Config,
            data_store::DataStore,
            db::DbPool,
            error::{Error,
                    Result},
            protocol::jobsrv};

use crate::hab_core::package::{target,
                               PackageIdent,
                               PackageTarget};

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum JobState {
    Pending          = 0,
    Processing       = 1,
    Complete         = 2,
    Rejected         = 3,
    Failed           = 4,
    Dispatched       = 5,
    CancelPending    = 6,
    CancelProcessing = 7,
    CancelComplete   = 8,
    Schedulable      = 9,
    Eligible         = 10,
}

#[derive(Debug)]
struct WorkerId(String);
#[derive(Debug)]
struct JobId(i64);
#[derive(Debug)]
struct GroupId(i64);
#[derive(Debug)]
struct StateBlob(String);

type Responder<T> = oneshot::Sender<Result<T>>;
type Started = oneshot::Sender<()>;

#[derive(Debug)]
#[non_exhaustive]
pub enum SchedulerMessage {
    JobGroupAdded {
        group: GroupId,
    },
    JobGroupCanceled {
        group: GroupId,
    },
    WorkerNeedsWork {
        worker: WorkerId,
        target: PackageTarget,
        reply:  Responder<JobId>,
    },
    WorkerFinished {
        worker: WorkerId,
        job:    JobId,
        state:  jobsrv::JobState, /* do we distingush cancel from fail and sucess? Should this
                                   * be a status? */
    },
    WorkerGone {
        worker: WorkerId,
        job:    JobId,
    },
    State {
        reply: Responder<StateBlob>,
    },
    /* TODO maybe Watchdog, ProcessMetrics (or combine those two); what's a good periodic
     * message pump pattern? Could live alongside in separate thread */
}

#[derive(Debug)]
pub enum WorkerManagerMessage {
    NewWorkForTarget { target: PackageTarget },
    CancelJob { jobs: Vec<JobId> },
}

// This wraps the datastore API; this should probably be thread safe so it can be shared.
pub trait SchedulerDataStore: Send + Sync {
    fn TakeNextJobForTarget(&self, target: PackageTarget) -> JobId;
    fn MarkJobCompleteAndUpdateDependencies(&self, job: JobId);
}

#[derive(Debug)]
pub struct Scheduler {
    rx:         mpsc::Receiver<SchedulerMessage>,
    data_store: Box<dyn SchedulerDataStore>,
}

impl Scheduler {
    pub fn new(data_store: Box<dyn SchedulerDataStore>,
               rx: mpsc::Receiver<SchedulerMessage>,
               _tx: mpsc::Sender<WorkerManagerMessage>)
               -> Scheduler {
        Scheduler { data_store, rx }
    }

    #[tracing::instrument]
    pub async fn run(&mut self) {
        while let Some(msg) = self.rx.recv().await {
            match msg {
                SchedulerMessage::WorkerNeedsWork { worker: worker,
                                                    target: target,
                                                    reply: reply, } => {
                    self.worker_needs_work(&worker, target, reply)
                }
                SchedulerMessage::WorkerFinished { worker: worker,
                                                   job: job_id,
                                                   state: state, } => {
                    self.worker_finished(&worker, job_id, state)
                }
                _ => (),
            }
        }
    }

    #[tracing::instrument]
    fn worker_needs_work(&mut self,
                         worker: &WorkerId,
                         target: PackageTarget,
                         reply: Responder<JobId>) {
        let job_id = self.data_store.TakeNextJobForTarget(target);
        reply.send(Ok(job_id));
    }

    #[tracing::instrument]
    fn worker_finished(&self, worker: &WorkerId, job_id: JobId, state: jobsrv::JobState) {
        // Mark the job complete, depending on the result. These need to be atomic as, to avoid
        // losing work in flight
        match state {
            // If it successful, we will mark it done, and update the available jobs to run

            // If it fails, we will mark it failed, and recursively mark the dependent jobs as
            // failed

            // If it is canceled, (maybe handled here?) we mark it canceled; probably should check
            // if the containing group is canceled for sanitys sake.
            _ => (), // log an error
        }
    }
}

impl fmt::Debug for dyn SchedulerDataStore {
    // TODO: What should go here?
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "SchedulerDataStore{{}}") }
}

// TODO: Take a scheduler or the parameters to create one?
// Possibly the former, since we will need to hand out the tx end of the scheduler mpsc
// which would change our return type to (mpsc::Sender, JoinHandle)
pub fn start_scheduler(mut scheduler: Scheduler) -> JoinHandle<()> {
    let x: JoinHandle<()> = tokio::task::spawn(async move {
        scheduler.run().await;
    });
    x
}

#[cfg(test)]
mod test {

    use super::*;
    #[derive(Default)]
    struct DummySchedulerDataStore {}

    impl SchedulerDataStore for DummySchedulerDataStore {
        fn TakeNextJobForTarget(&self, target: PackageTarget) -> JobId { JobId(1) }

        fn MarkJobCompleteAndUpdateDependencies(&self, _job: JobId) { () }
    }

    #[tokio::test]
    async fn simple() {
        let (mut s_tx, s_rx) = tokio::sync::mpsc::channel(1);
        let (wrk_tx, _wrk_rx) = tokio::sync::mpsc::channel(1);

        let mut scheduler = Scheduler::new(Box::new(DummySchedulerDataStore {}), s_rx, wrk_tx);
        let join = tokio::task::spawn(async move { scheduler.run().await });
        // Do some tests.
        let (otx, orx) = oneshot::channel::<Result<JobId>>();

        s_tx.send(SchedulerMessage::WorkerNeedsWork { worker: WorkerId("worker1".to_string()),
                                                      target:
                                                          PackageTarget::from_str("x86_64-linux").unwrap(),
                                                      reply:  otx, }).await;
        join.await;
    }
}
