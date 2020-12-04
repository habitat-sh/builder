use diesel::{r2d2::{ConnectionManager,
                    PooledConnection},
             PgConnection};

use protobuf::RepeatedField;

use crate::{db::models::{jobs::{Group,
                                JobGraphEntry,
                                JobStateCounts},
                         package::{BuilderPackageIdent,
                                   BuilderPackageTarget}},
            error::{Error,
                    Result},
            protocol::jobsrv};

use crate::data_store::DataStore;

#[cfg(test)]
#[cfg(feature = "postgres_tests")]
use habitat_builder_db::datastore_test;

// cargo test --features postgres_tests to enable
// from root
// cargo test -p habitat_builder_jobsrv --features=postgres_tests
// --manifest-path=components/builder-jobsrv/Cargo.toml
#[cfg(test)]
#[cfg(feature = "postgres_tests")]
mod test;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct WorkerId(pub String);
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct JobGraphId(pub i64);
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct GroupId(pub i64);

/// This trait wraps the wraps the datastore API. The primary purpose of using a trait here is to
/// allow us to build various mocks for testing.
pub trait SchedulerDataStore: Send + Sync {
    /// Returns an available job for the target, and marks it in flight
    /// This abstracts any desired priortization algorithm
    fn take_next_job_for_target(&mut self,
                                target: BuilderPackageTarget)
                                -> Result<Option<JobGraphEntry>>;

    /// Marks the job as complete and successful, and associates the resulting
    /// package id with the job. It also updates any dependencies, potentially making them
    /// ready to run.
    ///
    /// Returns the number of dependencies updated to 'ready'
    fn mark_job_complete_and_update_dependencies(&mut self,
                                                 job: JobGraphId,
                                                 as_built: &BuilderPackageIdent)
                                                 -> Result<i32>;

    /// Marks the job as failed. Returns the number of dependencies marked
    /// as transitively failed.
    fn mark_job_failed(&mut self, job: JobGraphId) -> Result<i32>;

    /// Counts the distribution of jobs by their states.
    fn count_all_states(&mut self, group: GroupId) -> Result<JobStateCounts>;

    /// Set the group state to a new value
    fn set_job_group_state(&mut self,
                           group: GroupId,
                           group_state: jobsrv::JobGroupState)
                           -> Result<()>;

    /// Return a count of the number of jobs 'ready' for a given target, often as a prelude to
    /// bringing in another group
    fn count_ready_for_target(&mut self, target: BuilderPackageTarget) -> Result<usize>;

    /// Make a group's jobs ready for execution
    ///
    /// This moves all jobs in a group from Pending to either WaitingOnDependency or Ready,
    /// depending on whether the job has dependencies (WaitingOnDependency), or not (Ready)
    fn group_dispatched_update_jobs(&mut self, group_id: GroupId) -> Result<usize>;

    /// This finds the next available build group and returns it if one is available.
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
    /// * Blocks creation of the datastore on the existence of the pool; might wait indefinetly.
    pub fn new(data_store: DataStore) -> Self { SchedulerDataStoreDb { data_store } }

    // This works because there's a Deref to  PgConnection implemented
    // https://docs.diesel.rs/1.4.x/src/r2d2/lib.rs.html#620-629
    fn get_connection(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.data_store
            .get_pool()
            .get_conn()
            .expect("Could not get database connection")
    }

    pub fn get_job_group(&self,
                         group_id: i64,
                         include_projects: bool)
                         -> Result<Option<jobsrv::JobGroup>> {
        let maybe_group = match Group::get(group_id, &self.get_connection()) {
            Err(diesel::result::Error::NotFound) => {
                warn!("JobGroup id {} not found", group_id);
                Ok(None)
            }
            Err(e) => Err(Error::SchedulerDbError(e)),
            Ok(g) => Ok(Some(g)),
        }?;

        if let Some(group) = maybe_group {
            let mut job_group = jobsrv::JobGroup::new();

            job_group.set_id(group.id as u64);

            let group_state = group.group_state.parse::<jobsrv::JobGroupState>()?;
            job_group.set_state(group_state);
            if let Some(date) = group.created_at {
                job_group.set_created_at(date.to_rfc3339())
            }
            job_group.set_project_name(group.project_name);
            job_group.set_target(group.target);

            if include_projects {
                // Need to remap job_graph_entries in to group_project like entries
                let entries = JobGraphEntry::list_group(group_id, &self.get_connection())
                                           .map_err(Error::SchedulerDbError)?;

                let mut projects = RepeatedField::new();
                for entry in entries {
                    let project: jobsrv::JobGroupProject = entry.into();
                    projects.push(project);
                }

                job_group.set_projects(projects);
            }
            Ok(Some(job_group))
        } else {
            Ok(None)
        }
    }
}

// Test helpers for setup/teardown of internal DB
#[cfg(test)]
#[cfg(feature = "postgres_tests")]
impl SchedulerDataStoreDb {
    pub fn new_test() -> Self {
        let data_store = datastore_test!(DataStore);
        SchedulerDataStoreDb { data_store }
    }

    pub fn get_connection_for_test(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.get_connection()
    }
}

impl SchedulerDataStore for SchedulerDataStoreDb {
    fn take_next_job_for_target(&mut self,
                                target: BuilderPackageTarget)
                                -> Result<Option<JobGraphEntry>> {
        JobGraphEntry::take_next_job_for_target(target,
                                                &self.get_connection()).map_err(Error::SchedulerDbError)
    }

    fn mark_job_complete_and_update_dependencies(&mut self,
                                                 job: JobGraphId,
                                                 as_built: &BuilderPackageIdent)
                                                 -> Result<i32> {
        JobGraphEntry::mark_job_complete(job.0, as_built, &self.get_connection()).map_err(|e| {
            Error::SchedulerDbError(e)
        })
    }

    fn mark_job_failed(&mut self, job: JobGraphId) -> Result<i32> {
        JobGraphEntry::mark_job_failed(job.0, &self.get_connection()).map_err(                                                                         Error::SchedulerDbError)
    }

    fn count_all_states(&mut self, group: GroupId) -> Result<JobStateCounts> {
        JobGraphEntry::count_all_states(group.0,  &self.get_connection()).map_err(
            Error::SchedulerDbError)
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
            &self.get_connection()).map_err( Error::SchedulerDbError)
    }
}
