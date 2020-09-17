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

use diesel::{r2d2::{ConnectionManager,
                    PooledConnection},
             PgConnection};

use crate::{db::{config::DataStoreCfg,
                 models::{jobs::{Group,
                                 JobGraphEntry,
                                 JobStateCounts},
                          package::BuilderPackageTarget}},
            error::{Error,
                    Result},
            protocol::jobsrv};

use crate::data_store::DataStore;

#[cfg(feature = "postgres_tests")]
#[allow(unused_imports)]
use habitat_builder_db::datastore_test;

mod test;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct WorkerId(pub String);
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct JobGraphId(pub i64);
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct GroupId(pub i64);

// This wraps the datastore API; this should probably be thread safe so it can be shared.
pub trait SchedulerDataStore: Send + Sync {
    fn take_next_job_for_target(&mut self,
                                target: BuilderPackageTarget)
                                -> Result<Option<JobGraphEntry>>;
    fn mark_job_complete_and_update_dependencies(&mut self, job: JobGraphId) -> Result<i32>;
    fn mark_job_failed(&mut self, job: JobGraphId) -> Result<i32>;
    fn count_all_states(&mut self, group: GroupId) -> Result<JobStateCounts>;
    fn set_job_group_state(&mut self,
                           group: GroupId,
                           group_state: jobsrv::JobGroupState)
                           -> Result<()>;
    fn count_ready_for_target(&mut self, target: BuilderPackageTarget) -> Result<usize>;
    fn group_dispatched_update_jobs(&mut self, group_id: GroupId) -> Result<usize>;
    fn take_next_group_for_target(&mut self, target: BuilderPackageTarget)
                                  -> Result<Option<Group>>;
}

//
pub struct SchedulerDataStoreDb {
    data_store: DataStore, // When we get rid of non-diesel stuff maybe just use directly
}

impl SchedulerDataStoreDb {
    /// Create a new DataStore.
    ///
    /// * Can fail if the pool cannot be created
    /// * Blocks creation of the datastore on the existince of the pool; might wait indefinetly.
    pub fn new(cfg: &DataStoreCfg) -> Self {
        let data_store = DataStore::new(cfg);
        SchedulerDataStoreDb { data_store }
    }

    // This works because there's a Deref to  PgConnection implemented
    // https://docs.diesel.rs/1.4.x/src/r2d2/lib.rs.html#620-629
    pub fn get_connection(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.data_store.get_pool().get_conn().unwrap()
    }

    // Test helpers for setup/teardown of internal DB
    #[cfg(test)]
    #[cfg(feature = "postgres_scheduler_tests")]
    pub fn new_test() -> Self {
        let data_store = datastore_test!(DataStore);
        SchedulerDataStoreDb { data_store }
    }

    #[cfg(test)]
    #[cfg(feature = "postgres_scheduler_tests")]
    pub fn get_connection_for_test(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.get_connection()
    }
}

impl SchedulerDataStore for SchedulerDataStoreDb {
    fn take_next_job_for_target(&mut self,
                                target: BuilderPackageTarget)
                                -> Result<Option<JobGraphEntry>> {
        JobGraphEntry::take_next_job_for_target(target,
                                                &self.get_connection()).map_err(|e| Error::SchedulerDbError(e))
    }

    fn mark_job_complete_and_update_dependencies(&mut self, job: JobGraphId) -> Result<i32> {
        JobGraphEntry::mark_job_complete(job.0, &self.get_connection()).map_err(|e| {
            Error::SchedulerDbError(e)
        })
    }

    fn mark_job_failed(&mut self, job: JobGraphId) -> Result<i32> {
        JobGraphEntry::mark_job_failed(job.0, &self.get_connection()).map_err(|e| {
                                                                         Error::SchedulerDbError(e)
                                                                     })
    }

    fn count_all_states(&mut self, group: GroupId) -> Result<JobStateCounts> {
        JobGraphEntry::count_all_states(group.0,  &self.get_connection()).map_err(|e| {
            Error::SchedulerDbError(e)
        })
    }

    fn set_job_group_state(&mut self,
                           group: GroupId,
                           group_state: jobsrv::JobGroupState)
                           -> Result<()> {
        // TODO REVISIT the u64 cast; we cast it back in forth multiple times
        self.data_store
            .set_job_group_state(group.0 as u64, group_state)
    }

    fn count_ready_for_target(&mut self, target: BuilderPackageTarget) -> Result<usize> {
        JobGraphEntry::count_ready_for_target(target,
                 &self.get_connection())
                 .map_err(|e| {
                    Error::SchedulerDbError(e)
                 })
                 .map(|x| x as usize)
    }

    fn group_dispatched_update_jobs(&mut self, group_id: GroupId) -> Result<usize> {
        JobGraphEntry::group_dispatched_update_jobs(group_id.0,
            &self.get_connection())
            .map_err(|e| {
               Error::SchedulerDbError(e)
            })
    }

    fn take_next_group_for_target(&mut self,
                                  target: BuilderPackageTarget)
                                  -> Result<Option<Group>> {
        Group::take_next_group_for_target(target.0,
            &self.get_connection()).map_err(|e| Error::SchedulerDbError(e))
    }
}

// Test code
//
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum DummySchedulerDataStoreCall {
    TakeNextJobForTarget { target: BuilderPackageTarget },
    MarkJobCompleteAndUpdateDependencies { job_id: JobGraphId },
}

#[derive(Debug)]
pub enum DummySchedulerDataStoreResult {
    JobOption(Result<Option<JobGraphEntry>>),
    UnitResult(),
}

#[derive(Default)]
pub struct DummySchedulerDataStore {
    pub actions: Vec<(DummySchedulerDataStoreCall, DummySchedulerDataStoreResult)>,
}

impl DummySchedulerDataStore {
    pub fn new(actions: Vec<(DummySchedulerDataStoreCall, DummySchedulerDataStoreResult)>) -> Self {
        DummySchedulerDataStore { actions }
    }
}

impl SchedulerDataStore for DummySchedulerDataStore {
    fn take_next_job_for_target(&mut self,
                                target: BuilderPackageTarget)
                                -> Result<Option<JobGraphEntry>> {
        assert!(self.actions.len() > 0);
        assert_eq!(self.actions[0].0,
                   DummySchedulerDataStoreCall::TakeNextJobForTarget { target });
        if let (_, DummySchedulerDataStoreResult::JobOption(r)) = self.actions.remove(0) {
            r
        } else {
            unreachable!("some sort of strange data problem")
        }
    }

    fn mark_job_complete_and_update_dependencies(&mut self, job_id: JobGraphId) -> Result<i32> {
        assert!(self.actions.len() > 0);
        assert_eq!(self.actions[0].0,
                   DummySchedulerDataStoreCall::MarkJobCompleteAndUpdateDependencies { job_id });
        if let (_, DummySchedulerDataStoreResult::UnitResult()) = self.actions.remove(0) {
            Ok(1)
        } else {
            unreachable!("some sort of strange data problem")
        }
    }

    fn mark_job_failed(&mut self, _job: JobGraphId) -> Result<i32> { Ok(0) }

    fn count_all_states(&mut self, _group: GroupId) -> Result<JobStateCounts> {
        Ok(JobStateCounts::default())
    }

    fn set_job_group_state(&mut self,
                           _group: GroupId,
                           _group_state: jobsrv::JobGroupState)
                           -> Result<()> {
        Ok(())
    }

    fn count_ready_for_target(&mut self, _target: BuilderPackageTarget) -> Result<usize> { Ok(0) }

    fn group_dispatched_update_jobs(&mut self, _group: GroupId) -> Result<usize> { Ok(0) }

    fn take_next_group_for_target(&mut self,
                                  target: BuilderPackageTarget)
                                  -> Result<Option<Group>> {
        // Todo make a better error here
        Err(Error::System)
    }
}
