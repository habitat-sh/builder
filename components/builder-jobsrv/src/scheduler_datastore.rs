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

use crate::{config::Config,
            data_store::DataStore,
            db::DbPool,
            error::{Error,
                    Result},
            protocol::jobsrv};

use crate::hab_core::package::{target,
                               PackageIdent,
                               PackageTarget};

#[allow(dead_code)] // TODO REMOVE
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

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct WorkerId(pub String);
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct JobId(pub i64);
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct GroupId(pub i64);

// This wraps the datastore API; this should probably be thread safe so it can be shared.
pub trait SchedulerDataStore: Send + Sync {
    fn take_next_job_for_target(&mut self, target: PackageTarget) -> Result<Option<JobId>>;
    fn mark_job_complete_and_update_dependencies(&mut self, job: JobId) -> Result<()>;
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum DummySchedulerDataStoreCall {
    TakeNextJobForTarget { target: PackageTarget },
    MarkJobCompleteAndUpdateDependencies { job: JobId },
}

#[derive(Debug)]
pub enum DummySchedulerDataStoreResult {
    JobOption(Result<Option<JobId>>),
    UnitResult(Result<()>),
}

#[derive(Default)]
pub struct DummySchedulerDataStore {
    pub actions:  Vec<(DummySchedulerDataStoreCall, DummySchedulerDataStoreResult)>,
}

impl DummySchedulerDataStore {
    pub new(actions:  Vec<(DummySchedulerDataStoreCall, DummySchedulerDataStoreResult)>) -> Self {
        DummySchedulerDataStore{ actions }
    }
}

impl SchedulerDataStore for DummySchedulerDataStore {
    fn take_next_job_for_target(&mut self, target: PackageTarget) -> Result<Option<JobId>> {
        assert!(self.actions.len() > 0);
        assert_eq!(self.actions[0].0,
                   DummySchedulerDataStoreCall::TakeNextJobForTarget { target });
        if let (_, DummySchedulerDataStoreResult::JobOption(r)) = self.actions.pop().unwrap() {
            r
        } else {
            unreachable!("some sort of strange data problem")
        }
    }

    fn mark_job_complete_and_update_dependencies(&mut self, _job: JobId) -> Result<()> { Ok(()) }
}
