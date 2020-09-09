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

use diesel::{result::Error as Dre,
             Connection};

use crate::{db::{config::DataStoreCfg,
                 models::{jobs::{JobExecState,
                                 JobGraphEntry},
                          package::BuilderPackageTarget},
                 DbPool},
            error::Result};

use crate::hab_core::package::PackageTarget;

use std::str::FromStr;

mod test;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct WorkerId(pub String);
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct JobId(pub i64);
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct GroupId(pub i64);

// This wraps the datastore API; this should probably be thread safe so it can be shared.
pub trait SchedulerDataStore: Send + Sync {
    fn take_next_job_for_target(&mut self, target: PackageTarget) -> Result<Option<JobGraphEntry>>;
    fn mark_job_complete_and_update_dependencies(&mut self, job: JobId) -> Result<i32>;
}

//
pub struct SchedulerDataStoreDb {
    diesel_pool: DbPool,
}

impl SchedulerDataStoreDb {
    /// Create a new DataStore.
    ///
    /// * Can fail if the pool cannot be created
    /// * Blocks creation of the datastore on the existince of the pool; might wait indefinetly.
    pub fn new(cfg: &DataStoreCfg) -> Self {
        let diesel_pool = DbPool::new(&cfg);
        SchedulerDataStoreDb { diesel_pool }
    }
}

use chrono::{TimeZone,
             Utc}; // TODO see if this can be cleaned up/eliminated
impl SchedulerDataStore for SchedulerDataStoreDb {
    fn take_next_job_for_target(&mut self, target: PackageTarget) -> Result<Option<JobGraphEntry>> {
        Ok(Some(JobGraphEntry { id: 0,
             group_id:         0, 
             job_state:        JobExecState::Pending, 
             plan_ident:       "dummy_plan_ident".to_owned(),       
             manifest_ident:   "dummy_manifest_ident".to_owned(),         
             as_built_ident:   None, 
             dependencies:     vec![],
             waiting_on_count: 0,
             target_platform:  BuilderPackageTarget( PackageTarget::from_str("x86_64-linux").unwrap()), 
             created_at:       Utc.timestamp(1431648000, 0),
             updated_at:       Utc.timestamp(1431648001, 0)
    }))
    }

    fn mark_job_complete_and_update_dependencies(&mut self, job: JobId) -> Result<i32> { Ok(0) }
}

// Test code
//
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum DummySchedulerDataStoreCall {
    TakeNextJobForTarget { target: PackageTarget },
    MarkJobCompleteAndUpdateDependencies { job_id: JobId },
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
    fn take_next_job_for_target(&mut self, target: PackageTarget) -> Result<Option<JobGraphEntry>> {
        assert!(self.actions.len() > 0);
        assert_eq!(self.actions[0].0,
                   DummySchedulerDataStoreCall::TakeNextJobForTarget { target });
        if let (_, DummySchedulerDataStoreResult::JobOption(r)) = self.actions.remove(0) {
            r
        } else {
            unreachable!("some sort of strange data problem")
        }
    }

    fn mark_job_complete_and_update_dependencies(&mut self, job_id: JobId) -> Result<i32> {
        assert!(self.actions.len() > 0);
        assert_eq!(self.actions[0].0,
                   DummySchedulerDataStoreCall::MarkJobCompleteAndUpdateDependencies { job_id });
        if let (_, DummySchedulerDataStoreResult::UnitResult()) = self.actions.remove(0) {
            Ok(1)
        } else {
            unreachable!("some sort of strange data problem")
        }
    }
}
