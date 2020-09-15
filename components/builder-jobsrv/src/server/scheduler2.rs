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
                                  SchedulerDataStore,
                                  WorkerId}};

use crate::{db::models::jobs::{JobExecState,
                               JobStateCounts},
            protocol::jobsrv};

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
        state:  JobExecState, /* do we distingush cancel from fail and sucess? Should this
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
        let maybe_job_id = match self.data_store.take_next_job_for_target(target) {
            Ok(Some(job)) => Ok(Some(JobId(job.id))),
            Ok(None) => {
                // TODO: queue up more work if available
                Ok(None)
            }
            _ => Ok(None), // TODO Process them errors!
        };
        // If the worker manager goes away, we're going to be restarting the server because
        // we have no recovery path. So panic is the right strategy.
        reply.send(maybe_job_id)
             .expect("Reply failed: Worker manager appears to have died")
    }

    #[tracing::instrument]
    fn worker_finished(&mut self, worker: &WorkerId, job_id: JobId, state: JobExecState) {
        // Mark the job complete, depending on the result. These need to be atomic as, to avoid
        // losing work in flight
        // NOTE: Should check job group invariants;
        // for each group (jobs in WaitingOnDependency + Ready + Running) states > 0
        // Others?
        match state {
            JobExecState::Complete => {
                // If it successful, we will mark it done, and update the available jobs to run
                // TODO detect if job group is done (
                let new_avail = self.data_store
                                    .mark_job_complete_and_update_dependencies(job_id)
                                    .expect("Can't yet handle db error");

                // TODO Detect when group is complete
                debug!("Job {} completed, {} now avail to run", job_id.0, new_avail);
            }
            JobExecState::JobFailed => {
                // If it fails, we will mark it failed, and recursively mark the dependent jobs as
                // failed
                let marked_failed = self.data_store
                                        .mark_job_failed(job_id)
                                        .expect("Can't yet handle db error");
                debug!("Job {} failed, {} total not runnable",
                       job_id.0, marked_failed);
                // TODO Detect when group is complete (failed)
            }
            // TODO: Handle cancel complete, and worker going AWOL

            // If it is canceled, (maybe handled here?) we mark it canceled; probably should check
            // if the containing group is canceled for sanitys sake.
            state => panic!("Unexpected state {:?}", state),
        }

        // Perhaps workers get the group id and pass it in here, perhaps we query the db
        let group_id = GroupId(0); // TODO FIGURE OUT HOW THIS HAPPENS
        self.check_group_completion(group_id, job_id);
    }

    // Check for the various ways a group might complete, and handle them
    //
    fn check_group_completion(&mut self, group_id: GroupId, job_id: JobId) {
        let counts = self.data_store
                         .count_all_states(group_id)
                         .expect("Can't yet handle db error");

        match counts {
            JobStateCounts { wd: 0,
                             rd: 0,
                             rn: 0,
                             jf: 0,
                             df: 0,
                             ct: complete,
                             .. } => {
                // No work in flight, no failures; assume success
                // (Cancellation will require extension)
                // INVARIANT: complete should be equal to size of group
                self.group_finished_successfully(group_id, complete)
            }
            JobStateCounts { wd: 0,
                             rd: 0,
                             rn: 0,
                             jf: job_fail,
                             df: dep_failed,
                             ct: complete,
                             .. } => {
                // No work in flight, failures, mark failed
                // (Cancellation will require extension)
                self.group_failed(group_id, counts)
            }
            JobStateCounts { wd: waiting,
                             rd: 0,
                             rn: 0,
                             .. } => {
                // No work in flight, none ready, we have a deadlock situation
                // If this state happens, we have most likely botched a state transition
                // or added an invalid graph entry
                error!("Group {} deadlocked, last job updated {}",
                       group_id.0, job_id.0);
                self.group_failed(group_id, counts)
            }
            JobStateCounts { wd: waiting,
                             rd: ready,
                             rn: running,
                             .. } => {
                // Keep on trucking; log and continue
            }
            _ => panic!("Unexpected job state for group {} {:?}", group_id.0, counts),
        }
    }

    fn group_finished_successfully(&mut self, group_id: GroupId, completed: i64) {
        self.data_store
            .set_job_group_state(group_id, jobsrv::JobGroupState::GroupComplete);
        trace!("Group {} completed {} jobs", group_id.0, completed);

        // What notifications/cleanups/protobuf calls etc need to happen here?
    }

    fn group_failed(&mut self, group_id: GroupId, counts: JobStateCounts) {
        self.data_store
            .set_job_group_state(group_id, jobsrv::JobGroupState::GroupFailed);
        trace!("Group {} failed {:?}", group_id.0, counts);
        // What notifications/cleanups/protobuf calls etc need to happen here?
    }
}

impl fmt::Debug for dyn SchedulerDataStore {
    // TODO: What should go here?
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "SchedulerDataStore{{}}") }
}

// TODO: Take a scheduler or the parameters to create one?
// Possibly the former, since we will need to hand out the tx end of the scheduler mpsc
// which would change our return type to (mpsc::Sender, JoinHandle)
#[allow(dead_code)]
pub fn start_scheduler(mut scheduler: Scheduler) -> JoinHandle<()> {
    let x: JoinHandle<()> = tokio::task::spawn(async move {
        scheduler.run().await;
    });
    x
}

#[cfg(test)]
#[cfg(feature = "postgres_scheduler_tests")]
mod test;
