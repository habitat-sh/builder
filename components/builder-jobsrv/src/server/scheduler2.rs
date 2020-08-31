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

use std::fmt;

use tokio::{sync::{mpsc,
                   oneshot},
            task::JoinHandle};

use crate::{error::Result,
            scheduler_datastore::{GroupId,
                                  JobId,
                                  JobState,
                                  SchedulerDataStore,
                                  WorkerId}};

use crate::hab_core::package::PackageTarget;

#[derive(Debug)]
pub struct StateBlob(String);

type Responder<T> = oneshot::Sender<Result<T>>;

#[derive(Debug)]
#[allow(dead_code)] // TODO REMOVE
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
        reply:  Responder<Option<JobId>>,
    },
    WorkerFinished {
        worker: WorkerId,
        job:    JobId,
        state:  JobState, /* do we distingush cancel from fail and sucess? Should this
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
        println!("Loop started");
        while let Some(msg) = self.rx.recv().await {
            println!("Msg {:?}", msg);
            match msg {
                SchedulerMessage::WorkerNeedsWork { worker,
                                                    target,
                                                    reply, } => {
                    self.worker_needs_work(&worker, target, reply)
                }
                SchedulerMessage::WorkerFinished { worker,
                                                   job: job_id,
                                                   state, } => {
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
                         reply: Responder<Option<JobId>>) {
        let job_id = self.data_store.take_next_job_for_target(target);
        // Probably should do some sort of parse/check here to examine the error
        // returned.SchedulerDataStore

        // If the worker manager goes away, we're going to be restarting the server because we have
        // no recovery path. So panic is the right strategy.
        reply.send(job_id)
             .expect("Reply failed: Worker manager appears to have died");
    }

    #[tracing::instrument]
    fn worker_finished(&self, worker: &WorkerId, job_id: JobId, state: JobState) {
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

    use crate::scheduler_datastore::{DummySchedulerDataStore,
                                     DummySchedulerDataStoreCall,
                                     DummySchedulerDataStoreResult,
                                     GroupId,
                                     JobId,
                                     JobState,
                                     SchedulerDataStore,
                                     WorkerId};

    use lazy_static;
    use std::str::FromStr;

    lazy_static! {
        static ref TARGET_LINUX: PackageTarget = PackageTarget::from_str("x86_64-linux").unwrap();
        static ref TARGET_WINDOWS: PackageTarget =
            PackageTarget::from_str("x86_64-windows").unwrap();
    }

    #[tokio::test]
    async fn simple_job_fetch() {
        let actions =
            vec![(DummySchedulerDataStoreCall::TakeNextJobForTarget { target: *TARGET_LINUX, },
                  DummySchedulerDataStoreResult::JobOption(Ok(Some(JobId(1))))),
                 (DummySchedulerDataStoreCall::TakeNextJobForTarget { target: *TARGET_WINDOWS, },
                  DummySchedulerDataStoreResult::JobOption(Ok(None)))];

        let dummy_store = DummySchedulerDataStore::new(actions);

        let (mut s_tx, s_rx) = tokio::sync::mpsc::channel(1);
        let (wrk_tx, _wrk_rx) = tokio::sync::mpsc::channel(1);

        let mut scheduler = Scheduler::new(Box::new(dummy_store), s_rx, wrk_tx);
        let join = tokio::task::spawn(async move { scheduler.run().await });

        // expect a job for this target
        let (o_tx, o_rx) = oneshot::channel::<Result<Option<JobId>>>();
        let _ = s_tx.send(SchedulerMessage::WorkerNeedsWork { worker:
                                                                  WorkerId("worker1".to_string()),
                                                              target: *TARGET_LINUX,
                                                              reply:  o_tx, })
                    .await;

        let reply: Result<Option<JobId>> = o_rx.await.unwrap();
        println!("Reply 1 {:?}", reply);
        assert_eq!(1, reply.unwrap().unwrap().0);

        // No job for this target
        let (o_tx, o_rx) = oneshot::channel::<Result<Option<JobId>>>();
        let _ = s_tx.send(SchedulerMessage::WorkerNeedsWork { worker:
                                                                  WorkerId("worker1".to_string()),
                                                              target: *TARGET_WINDOWS,
                                                              reply:  o_tx, })
                    .await;

        let reply: Result<Option<JobId>> = o_rx.await.unwrap();
        println!("Reply 2 {:?}", reply);
        assert_eq!(None, reply.unwrap());

        drop(s_tx);
        join.await.unwrap();
    }

    #[tokio::test]
    async fn simple_job_fetch() {
        let worker = WorkerId("worker1".to_string();

        let actions =
            vec![(DummySchedulerDataStoreCall::TakeNextJobForTarget { target: *TARGET_LINUX, },
                  DummySchedulerDataStoreResult::JobOption(Ok(Some(JobId(1))))),
                 (DummySchedulerDataStoreCall::MarkJobCompleteAndUpdateDependencies { job_id:
                                                                                          JobId(1), },
                  DummySchedulerDataStoreResult::UnitResult())];

        let dummy_store = DummySchedulerDataStore::new(actions);

        let (mut s_tx, s_rx) = tokio::sync::mpsc::channel(1);
        let (wrk_tx, _wrk_rx) = tokio::sync::mpsc::channel(1);

        let mut scheduler = Scheduler::new(Box::new(dummy_store), s_rx, wrk_tx);
        let join = tokio::task::spawn(async move { scheduler.run().await });

        // expect a job for this target
        let (o_tx, o_rx) = oneshot::channel::<Result<Option<JobId>>>();
        let _ = s_tx.send(SchedulerMessage::WorkerNeedsWork { worker:
                                                                  worker.clone()),
                                                              target: *TARGET_LINUX,
                                                              reply:  o_tx, })
                    .await;

        let reply: Result<Option<JobId>> = o_rx.await.unwrap();
        println!("Reply 1 {:?}", reply);
        assert_eq!(1, reply.unwrap().unwrap().0);
        let job_id = reply.unwrap().unwrap();

        let _ = s_tx.send(SchedulerMessage::WorkerFinished{ worker: worker.clone(),
                job_id, JobState::Complete}).await;



        drop(s_tx);
        join.await.unwrap();
    }
}
