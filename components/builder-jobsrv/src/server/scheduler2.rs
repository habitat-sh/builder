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

use crate::scheduler_datastore::{GroupId,
                                 JobGraphId,
                                 SchedulerDataStore,
                                 WorkerId};

use crate::{db::models::{jobs::{JobExecState,
                                JobGraphEntry,
                                JobStateCounts},
                         package::BuilderPackageTarget},
            protocol::jobsrv};

#[derive(Debug)]
pub struct StateBlob {
    message_count:      usize,
    last_message_debug: String, /* It would be cool to be able to do something like this:
                                 * last_message:  Option<SchedulerMessage>,
                                 * But the responders can't be copied so it's hard to keep the
                                 * message around. */
}

type Responder<T> = oneshot::Sender<T>;

#[derive(Debug)]
#[allow(dead_code)] // TODO REMOVE
#[non_exhaustive]
pub enum SchedulerMessage {
    JobGroupAdded {
        group:  GroupId,
        target: BuilderPackageTarget,
    },
    JobGroupCanceled {
        group: GroupId,
    },
    WorkerNeedsWork {
        worker: WorkerId,
        target: BuilderPackageTarget,
        reply:  Responder<Option<JobGraphEntry>>,
    },
    WorkerFinished {
        worker: WorkerId,
        job:    JobGraphEntry,
    },
    WorkerGone {
        worker: WorkerId,
        job:    JobGraphEntry,
    },
    State {
        reply: Responder<StateBlob>,
    },
    Halt,
    /* TODO maybe Watchdog, ProcessMetrics (or combine those two); what's a good periodic
     * message pump pattern? Could live alongside in separate thread */
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum WorkerManagerMessage {
    NewWorkForTarget { target: BuilderPackageTarget },
    CancelJob { jobs: Vec<JobGraphId> },
}

#[derive(Clone, Debug)]
pub struct Scheduler {
    tx: mpsc::Sender<SchedulerMessage>,
}

#[allow(dead_code)] // because we don't use this yet, TODO remove once we have code using it
impl Scheduler {
    pub fn new(tx: mpsc::Sender<SchedulerMessage>) -> Scheduler { Scheduler { tx } }

    pub fn start(data_store: Box<dyn SchedulerDataStore>,
                 queue_depth: usize)
                 -> (Scheduler, JoinHandle<()>) {
        // enforce once semantics
        let (s_tx, s_rx) = mpsc::channel(queue_depth);
        let mut scheduler = SchedulerInternal::new(data_store, s_rx);
        (Scheduler::new(s_tx), tokio::task::spawn(async move { scheduler.run().await }))
    }

    pub async fn job_group_added(&mut self, group: GroupId, target: BuilderPackageTarget) -> () {
        let msg = SchedulerMessage::JobGroupAdded { group, target };
        let _ = self.tx.send(msg).await;
    }

    pub async fn worker_needs_work(&mut self,
                                   worker: WorkerId,
                                   target: BuilderPackageTarget)
                                   -> Option<JobGraphEntry> {
        let (o_tx, o_rx) = oneshot::channel::<Option<JobGraphEntry>>();

        let msg = SchedulerMessage::WorkerNeedsWork { worker,
                                                      target,
                                                      reply: o_tx };
        let _ = self.tx.send(msg).await;

        let reply: Option<JobGraphEntry> = o_rx.await.unwrap();
        reply
    }

    pub async fn worker_finished(&mut self, worker: WorkerId, job: JobGraphEntry) {
        let msg = SchedulerMessage::WorkerFinished { worker, job };
        let _ = self.tx
                    .send(msg)
                    .await
                    .expect("Unable to send worker finished message");
    }

    pub async fn state(&mut self) -> StateBlob {
        let (o_tx, o_rx) = oneshot::channel::<StateBlob>();

        let msg = SchedulerMessage::State { reply: o_tx };
        let _ = self.tx.send(msg).await;

        let reply1: StateBlob = o_rx.await.unwrap();
        reply1
    }
}

#[derive(Debug)]
struct SchedulerInternal {
    rx:         mpsc::Receiver<SchedulerMessage>,
    data_store: Box<dyn SchedulerDataStore>,
}

impl SchedulerInternal {
    #[allow(dead_code)]
    pub fn new(data_store: Box<dyn SchedulerDataStore>,
               rx: mpsc::Receiver<SchedulerMessage>)
               -> SchedulerInternal {
        SchedulerInternal { data_store, rx }
    }

    #[tracing::instrument]
    pub async fn run(&mut self) {
        println!("Loop started");
        let mut message_count: usize = 0;
        let mut last_message_debug = "".to_owned();

        while let Some(msg) = self.rx.recv().await {
            println!("Msg {:?}", msg);
            message_count += 1;

            let message_debug = format!("{:?}", msg);
            println!("Handling {}: {}", message_count, message_debug);

            match msg {
                SchedulerMessage::JobGroupAdded { group, target } => {
                    self.job_group_added(group, target)
                }
                SchedulerMessage::JobGroupCanceled { .. } => unimplemented!("No JobGroupCanceled"),
                SchedulerMessage::WorkerNeedsWork { worker,
                                                    target,
                                                    reply, } => {
                    self.worker_needs_work(&worker, target, reply)
                }
                SchedulerMessage::WorkerFinished { worker, job } => {
                    self.worker_finished(&worker, job)
                }
                SchedulerMessage::WorkerGone { .. } => unimplemented!("No WorkerGone"),
                SchedulerMessage::State { reply } => {
                    let blob = StateBlob { message_count,
                                           last_message_debug };
                    // We ignore failure here, because this message could come from anywhere
                    let _ = reply.send(blob);
                }
                SchedulerMessage::Halt => break,
            }

            last_message_debug = message_debug;
        }
    }

    #[tracing::instrument]
    fn job_group_added(&mut self, group: GroupId, target: BuilderPackageTarget) {
        // if there are no ready jobs for this target dispatch it

        let ready = self.data_store
                        .count_ready_for_target(target)
                        .expect("Can't yet handle db error");
        if ready == 0 {
            // We assume there are no other queued jobs, because they'd be pulled in by now
            // as part of worker_needs_work

            self.dispatch_group_for_target(group, target);
        }
    }

    #[tracing::instrument]
    fn take_next_group_for_target(&mut self, target: BuilderPackageTarget) {
        if let Some(group) = self.data_store
                                 .take_next_group_for_target(target)
                                 .expect("Can't yet handle db error")
        {
            self.dispatch_group_for_target(GroupId(group.id), target)
        }
    }

    #[tracing::instrument]
    fn dispatch_group_for_target(&mut self, group_id: GroupId, _target: BuilderPackageTarget) {
        // Move the group to dispatching,
        self.data_store
            .set_job_group_state(group_id, jobsrv::JobGroupState::GroupDispatching)
            .expect("Can't yet handle db error");
        // update job graph entries to WaitingOnDependency or Ready
        // TODO: Need to CREATE jobs here, not
        let _ready = self.data_store
                         .group_dispatched_update_jobs(group_id)
                         .expect("Can't yet handle db error");

        // Eventually 'kick' the worker manger with an alert saying we have work instead of polling
        // Does something with ready/target
    }

    #[tracing::instrument]
    fn worker_needs_work(&mut self,
                         worker: &WorkerId,
                         target: BuilderPackageTarget,
                         reply: Responder<Option<JobGraphEntry>>) {
        // If there's no work, try and get a new group
        let ready = self.data_store
                        .count_ready_for_target(target)
                        .expect("Can't yet handle db error");
        if ready == 0 {
            self.take_next_group_for_target(target);
        }

        let maybe_job = match self.data_store.take_next_job_for_target(target) {
            Ok(Some(job)) => Some(job),
            Ok(None) => None,
            _ => None, // TODO Process them errors!
        };
        // If the worker manager goes away, we're going to be restarting the server because
        // we have no recovery path. So panic is the right strategy.
        reply.send(maybe_job)
             .expect("Reply failed: Worker manager appears to have died")
    }

    #[tracing::instrument]
    fn worker_finished(&mut self, worker: &WorkerId, job_entry: JobGraphEntry) {
        // Mark the job complete, depending on the result. These need to be atomic as, to avoid
        // losing work in flight
        // NOTE: Should check job group invariants;
        // for each group (jobs in WaitingOnDependency + Ready + Running) states > 0
        // Others?
        let job_id = JobGraphId(job_entry.id);
        match job_entry.job_state {
            JobExecState::Complete => {
                // If it successful, we will mark it done, and update the available jobs to run
                let new_avail = self.data_store
                                    .mark_job_complete_and_update_dependencies(job_id)
                                    .expect("Can't yet handle db error");

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
            }
            // TODO: Handle cancel complete, and worker going AWOL

            // If it is canceled, (maybe handled here?) we mark it canceled; probably should check
            // if the containing group is canceled for sanitys sake.
            state => panic!("Unexpected state {:?}", state),
        }

        // Perhaps workers get the group id and pass it in here, perhaps we query the db

        self.check_group_completion(job_entry);
    }

    // This probably belongs in a job_group_lifecycle module, but not today
    // Check for the various ways a group might complete, and handle them
    //
    fn check_group_completion(&mut self, job_entry: JobGraphEntry) {
        let group_id = GroupId(job_entry.group_id);

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
                             .. } if job_fail + dep_failed > 0 => {
                // No work in flight, failures, mark failed
                // (Cancellation will require extension)
                self.group_failed(group_id, counts)
            }
            JobStateCounts { wd: waiting,
                             rd: 0,
                             rn: 0,
                             .. } if waiting > 0 => {
                // No work in flight, none ready, we have a deadlock situation
                // If this state happens, we have most likely botched a state transition
                // or added an invalid graph entry
                error!("Group {} deadlocked, last job updated {} {}",
                       group_id.0, job_entry.manifest_ident, job_entry.id);
                self.group_failed(group_id, counts)
            }
            JobStateCounts { wd: waiting,
                             rd: ready,
                             rn: running,
                             .. } if waiting + ready + running > 0 => {
                // Keep on trucking; log and continue
            }
            _ => panic!("Unexpected job state for group {} {:?}", group_id.0, counts),
        }
    }

    fn group_finished_successfully(&mut self, group_id: GroupId, completed: i64) {
        self.data_store
            .set_job_group_state(group_id, jobsrv::JobGroupState::GroupComplete)
            .expect("Can't yet handle db error");
        trace!("Group {} completed {} jobs", group_id.0, completed);

        // What notifications/cleanups/protobuf calls etc need to happen here?
    }

    fn group_failed(&mut self, group_id: GroupId, counts: JobStateCounts) {
        self.data_store
            .set_job_group_state(group_id, jobsrv::JobGroupState::GroupFailed)
            .expect("Can't yet handle db error");
        trace!("Group {} failed {:?}", group_id.0, counts);
        // What notifications/cleanups/protobuf calls etc need to happen here?
    }
}

impl fmt::Debug for dyn SchedulerDataStore {
    // TODO: What should go here?
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "SchedulerDataStore{{}}") }
}

#[cfg(test)]
#[cfg(feature = "postgres_scheduler_tests")]
mod test;
