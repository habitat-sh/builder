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

//! The PostgreSQL backend for the Jobsrv.

embed_migrations!("src/migrations");

use std::io;

use chrono::{DateTime,
             Utc};
use diesel::{result::Error as Dre,
             Connection};
use protobuf::{self,
               RepeatedField};

use crate::db::{config::DataStoreCfg,
                migration::setup_ids,
                models::{channel::Channel,
                         jobs::{AuditJob,
                                Group,
                                GroupProject,
                                Job,
                                NewAuditJob,
                                NewJob,
                                UpdateGroupProject,
                                UpdateJob},
                         projects::Project},
                DbPool};

use crate::protocol::jobsrv;

use crate::error::{Error,
                   Result};

mod test;

/// DataStore inherints being Send + Sync by virtue of having only one member, the pool itself.
#[derive(Clone)]
pub struct DataStore {
    diesel_pool: DbPool,
}

impl DataStore {
    /// Create a new DataStore.
    ///
    /// * Can fail if the pool cannot be created
    /// * Blocks creation of the datastore on the existince of the pool; might wait indefinetly.
    pub fn new(cfg: &DataStoreCfg) -> Self {
        let diesel_pool = DbPool::new(&cfg);
        DataStore { diesel_pool }
    }

    pub fn get_pool(&self) -> habitat_builder_db::diesel_pool::DbPool { self.diesel_pool.clone() }

    /// Create a new DataStore from a pre-existing pool; useful for testing the database.
    pub fn from_pool(diesel_pool: DbPool) -> Self { DataStore { diesel_pool } }

    /// Setup the datastore.
    ///
    /// This includes all the schema and data migrations, along with stored procedures for data
    /// access.
    pub fn setup(&self) -> Result<()> {
        let conn = self.diesel_pool.get_conn()?;
        let _ = conn.transaction::<_, Dre, _>(|| {
                        setup_ids(&*conn).unwrap();
                        embedded_migrations::run_with_output(&*conn, &mut io::stdout()).unwrap();
                        Ok(())
                    });
        Ok(())
    }

    /// Create a new job. Sets the state to Pending.
    ///
    /// # Errors
    ///
    /// * If the pool has no connections available
    /// * If the job cannot be created
    /// * If the job has an unknown VCS type
    pub fn create_job(&self, job: &jobsrv::Job) -> Result<jobsrv::Job> {
        debug!("DataStore: create_job");
        let conn = self.diesel_pool.get_conn()?;

        let channel = if job.has_channel() {
            Some(job.get_channel())
        } else {
            None
        };

        if job.get_project().get_vcs_type() == "git" {
            let project = job.get_project();
            let install_id: Option<String> = {
                if project.has_vcs_installation_id() {
                    Some(project.get_vcs_installation_id().to_string())
                } else {
                    None
                }
            };

            let new_job = NewJob { owner_id: job.get_owner_id() as i64,
                                   project_id: project.get_id() as i64,
                                   project_name: &project.get_name(),
                                   job_state: "Pending",
                                   project_owner_id: project.get_owner_id() as i64,
                                   project_plan_path: &project.get_plan_path(),
                                   vcs: &project.get_vcs_type(),
                                   vcs_arguments: &vec![Some(project.get_vcs_data()),
                                                        install_id.as_deref()],
                                   channel,
                                   target: &job.get_target() };

            let result = Job::create(&new_job, &conn).map_err(Error::JobCreate)?;

            let job = result.into();
            Ok(job)
        } else {
            Err(Error::UnknownVCS)
        }
    }

    pub fn create_job_for_project(&self,
                                  group_id: u64,
                                  project: Project,
                                  target: &str)
                                  -> Result<Option<jobsrv::Job>> {
        debug!("DataStore: create_job_for_project");
        let mut job_spec = jobsrv::JobSpec::new();
        job_spec.set_owner_id(group_id);
        job_spec.set_project(project.into());
        job_spec.set_target(target.to_owned());
        job_spec.set_channel(Channel::channel_for_group(group_id));

        let job: jobsrv::Job = job_spec.into();
        match self.create_job(&job) {
            Ok(job) => {
                debug!("Job created: {:?}", job);
                Ok(Some(job))
            }
            Err(err) => {
                warn!("Unable to create job, err: {:?}", err);
                Err(err)
            }
        }
    }

    /// Get a job from the database. If the job does not exist, but the database was active, we'll
    /// get a None result.
    ///
    /// # Errors
    ///
    /// * If a connection cannot be gotten from the pool
    /// * If the job cannot be selected from the database
    pub fn get_job(&self, get_job: &jobsrv::JobGet) -> Result<Option<jobsrv::Job>> {
        debug!("DataStore: get_job");
        let conn = self.diesel_pool.get_conn()?;
        let result = Job::get(get_job.get_id() as i64, &conn);
        if let diesel::QueryResult::Err(diesel::result::Error::NotFound) = result {
            return Ok(None);
        }

        let job: jobsrv::Job = result.map_err(Error::JobGet)?.into();

        Ok(Some(job))
    }

    /// Get the next pending job from the list of pending jobs
    /// Atomically set the job state to Dispatching, and set the worker id
    ///
    /// # Errors
    ///
    /// * If a connection cannot be gotten from the pool
    /// * If the pending jobs cannot be selected from the database
    /// * If the row returned cannot be translated into a Job
    pub fn next_pending_job(&self, worker: &str, target: &str) -> Result<Option<jobsrv::Job>> {
        debug!("DataStore: next_pending_job");
        let conn = self.diesel_pool.get_conn()?;
        let result = Job::get_next_pending_job(worker, target, &conn);

        if let diesel::QueryResult::Err(diesel::result::Error::NotFound) = result {
            return Ok(None);
        }

        let job: jobsrv::Job = result.map_err(Error::JobPending)?.into();

        Ok(Some(job))
    }

    /// Get a list of cancel-pending jobs
    ///
    /// # Errors
    ///
    /// * If a connection cannot be gotten from the pool
    /// * If the cancel pending jobs cannot be selected from the database
    /// * If the row returned cannot be translated into a Job
    pub fn get_cancel_pending_jobs(&self) -> Result<Vec<jobsrv::Job>> {
        debug!("DataStore: get_cancel_pending_jobs");
        let conn = self.diesel_pool.get_conn()?;
        let results = Job::get_jobs_by_state(jobsrv::JobState::CancelPending, &conn).map_err(Error::JobPending)?;
        let jobs: Vec<jobsrv::Job> = results.into_iter().map(|j| j.into()).collect();

        Ok(jobs)
    }

    /// Get a list of Dispatched jobs
    ///
    /// # Errors
    ///
    /// * If a connection cannot be gotten from the pool
    /// * If the cancel pending jobs cannot be selected from the database
    /// * If the row returned cannot be translated into a Job
    pub fn get_dispatched_jobs(&self) -> Result<Vec<jobsrv::Job>> {
        debug!("DataStore: get_dispatched_jobs");
        let conn = self.diesel_pool.get_conn()?;
        let results =
            Job::get_jobs_by_state(jobsrv::JobState::Dispatched, &conn).map_err(Error::JobGet)?;
        let jobs: Vec<jobsrv::Job> = results.into_iter().map(|j| j.into()).collect();

        Ok(jobs)
    }

    /// Count the number of jobs in a given state
    ///
    /// # Errors
    ///
    /// * If a connection cannot be gotten from the pool
    pub fn count_jobs(&self, job_state: jobsrv::JobState) -> Result<i64> {
        debug!("DataStore: count_jobs");
        let conn = self.diesel_pool.get_conn()?;
        let count = Job::count_jobs(job_state, &conn).map_err(Error::JobGet)?;

        Ok(count)
    }

    /// Updates a job. Currently, this entails updating the state,
    /// build start and stop times, and recording the identifier of
    /// the package the job produced, if any.
    ///
    /// # Errors
    ///
    /// * If a connection cannot be gotten from the pool
    /// * If the job cannot be updated in the database
    pub fn update_job(&self, job: &jobsrv::Job) -> Result<()> {
        debug!("DataStore: update_job");
        let conn = self.diesel_pool.get_conn()?;
        let job_id = job.get_id() as i64;
        let job_state = job.get_state().to_string();

        // Note: the following fields may all be NULL. As currently
        // coded, if they are NULL, then the corresponding fields in
        // the database will also be updated to be NULL. This should
        // be OK, though, because they shouldn't be changing anyway.
        let build_started_at = if job.has_build_started_at() {
            Some(job.get_build_started_at().parse::<DateTime<Utc>>().unwrap())
        } else {
            None
        };

        let build_finished_at = if job.has_build_finished_at() {
            Some(job.get_build_finished_at()
                    .parse::<DateTime<Utc>>()
                    .unwrap())
        } else {
            None
        };

        let ident = if job.has_package_ident() {
            Some(job.get_package_ident().to_string())
        } else {
            None
        };

        let (err_code, err_msg) = if job.has_error() {
            (Some(job.get_error().get_code() as i32), Some(job.get_error().get_msg().to_string()))
        } else {
            (None, None)
        };

        let job = UpdateJob { id: job_id,
                              job_state,
                              net_error_code: err_code,
                              net_error_msg: err_msg,
                              package_ident: ident,
                              build_started_at,
                              build_finished_at };
        Job::update_job_with_sync(&job, &conn).map_err(Error::JobSetState)?;

        Ok(())
    }

    /// Marks a given job's logs as having been archived. The location
    /// and mechanism for retrieval are dependent on the configured archiving
    /// mechanism.
    pub fn mark_as_archived(&self, job_id: u64) -> Result<()> {
        debug!("DataStore: mark_as_archived");
        let conn = self.diesel_pool.get_conn()?;
        Job::mark_as_archived(job_id as i64, &conn).map_err(Error::JobMarkArchived)?;

        Ok(())
    }

    pub fn create_job_group(&self,
                            msg: &jobsrv::JobGroupSpec,
                            project_tuples: Vec<(String, String)>)
                            -> Result<jobsrv::JobGroup> {
        debug!("DataStore: create_job_group");
        let conn = self.diesel_pool.get_conn()?;

        assert!(!project_tuples.is_empty());

        let root_project = format!("{}/{}", msg.get_origin(), msg.get_package());
        let target = msg.get_target().to_string();

        let (project_names, project_idents): (Vec<String>, Vec<String>) =
            project_tuples.iter().cloned().unzip();

        let result = Group::create_job_group(&root_project,
                                             &target,
                                             &project_names,
                                             &project_idents,
                                             &conn).map_err(Error::JobGroupCreate)?;
        let mut group: jobsrv::JobGroup = result.into();
        let mut projects = RepeatedField::new();

        for (name, ident) in project_tuples {
            let mut project = jobsrv::JobGroupProject::new();
            project.set_name(name);
            project.set_ident(ident);
            project.set_state(jobsrv::JobGroupProjectState::NotStarted);
            project.set_target(msg.get_target().to_string());
            projects.push(project);
        }

        group.set_projects(projects);

        Ok(group)
    }

    pub fn cancel_job_group(&self, group_id: u64) -> Result<()> {
        debug!("DataStore: cancel_job_group");
        let conn = self.diesel_pool.get_conn()?;
        let _ = conn.transaction::<_, Dre, _>(|| {
                        GroupProject::cancel_group_project(group_id as i64, &conn).unwrap();
                        Group::cancel_job_group(group_id as i64, &conn).unwrap();
                        Ok(())
                    });

        Ok(())
    }

    pub fn create_audit_entry(&self, msg: &jobsrv::JobGroupAudit) -> Result<()> {
        debug!("DataStore: create_audit_entry");
        let conn = self.diesel_pool.get_conn()?;
        let audit_job = NewAuditJob { group_id:       msg.get_group_id() as i64,
                                      operation:      msg.get_operation() as i16,
                                      trigger:        msg.get_trigger() as i16,
                                      requester_id:   msg.get_requester_id() as i64,
                                      requester_name: msg.get_requester_name(), };

        AuditJob::create(&audit_job, &conn).map_err(Error::JobGroupAudit)?;

        Ok(())
    }

    pub fn get_job_group_origin(&self,
                                msg: &jobsrv::JobGroupOriginGet)
                                -> Result<jobsrv::JobGroupOriginResponse> {
        debug!("DataStore: get_job_group_origin");
        let conn = self.diesel_pool.get_conn()?;
        let results = Group::get_job_groups_for_origin(msg.get_origin(),
                                                       i64::from(msg.get_limit()),
                                                       &conn).map_err(Error::JobGroupOriginGet)?;

        let mut response = jobsrv::JobGroupOriginResponse::new();
        let mut job_groups = RepeatedField::new();

        for result in results.into_iter() {
            let group: jobsrv::JobGroup = result.into();
            job_groups.push(group);
        }

        response.set_job_groups(job_groups);
        Ok(response)
    }

    pub fn get_job_group(&self, msg: &jobsrv::JobGroupGet) -> Result<Option<jobsrv::JobGroup>> {
        debug!("DataStore: get_job_group");
        let group_id = msg.get_group_id() as i64;
        let include_projects = msg.get_include_projects();

        let conn = self.diesel_pool.get_conn()?;
        let result = Group::get_job_group(group_id, &conn);
        if let diesel::QueryResult::Err(diesel::result::Error::NotFound) = result {
            warn!("JobGroup id {} not found", group_id);
            return Ok(None);
        }

        let mut group: jobsrv::JobGroup = result.map_err(Error::JobGroupGet)?.into();

        if include_projects {
            let projects = GroupProject::get_group_projects(group_id, &conn)?;
            let projects: Vec<jobsrv::JobGroupProject> =
                projects.into_iter().map(|p| p.into()).collect();

            group.set_projects(RepeatedField::from_vec(projects));
        }

        Ok(Some(group))
    }

    pub fn set_job_group_state(&self,
                               group_id: u64,
                               group_state: jobsrv::JobGroupState)
                               -> Result<()> {
        debug!("DataStore: set_job_group_state");
        let conn = self.diesel_pool.get_conn()?;
        Group::set_group_state(group_id as i64, group_state, &conn).map_err(Error::JobGroupSetState)?;

        Ok(())
    }

    pub fn set_job_group_project_state(&self,
                                       group_id: u64,
                                       project_name: &str,
                                       project_state: jobsrv::JobGroupProjectState)
                                       -> Result<()> {
        debug!("DataStore: set_job_group_project_state");
        let conn = self.diesel_pool.get_conn()?;
        GroupProject::set_group_project_state(group_id as i64, project_name, project_state, &conn).map_err(Error::JobGroupProjectSetState)?;

        Ok(())
    }

    pub fn set_job_group_job_state(&self, job: &jobsrv::Job) -> Result<()> {
        debug!("DataStore: set_job_group_job_state");
        let conn = self.diesel_pool.get_conn()?;
        let result = GroupProject::get_group_project_by_name(job.get_owner_id() as i64,
                                                             &job.get_project().get_name(),
                                                             &conn);
        if let diesel::QueryResult::Err(diesel::result::Error::NotFound) = result {
            warn!("No project found for job id: {}", job.get_id());
            return Err(Error::UnknownJobGroupProjectState);
        }

        let group_project = result.map_err(Error::JobGroupProjectSetState)?;

        let state = match job.get_state() {
            jobsrv::JobState::Complete => "Success",
            jobsrv::JobState::Rejected => "NotStarted", // retry submission
            jobsrv::JobState::Failed => "Failure",
            jobsrv::JobState::Pending
            | jobsrv::JobState::Processing
            | jobsrv::JobState::Dispatched => "InProgress",
            jobsrv::JobState::CancelPending
            | jobsrv::JobState::CancelProcessing
            | jobsrv::JobState::CancelComplete => "Canceled",
        };

        let update_ident = if job.get_state() == jobsrv::JobState::Complete {
            Some(job.get_package_ident().to_string())
        } else {
            None
        };

        let update_project = UpdateGroupProject { id:            group_project.id,
                                                  project_state: state.to_string(),
                                                  job_id:        job.get_id() as i64,
                                                  project_ident: update_ident, };

        GroupProject::update_group_project(&update_project, &conn)?;

        Ok(())
    }

    pub fn pending_job_groups(&self, count: i32) -> Result<Vec<jobsrv::JobGroup>> {
        debug!("DataStore: pending_job_groups");
        let mut groups = Vec::new();

        let conn = self.diesel_pool.get_conn()?;

        let job_groups = Group::pending_job_groups(count, &conn).map_err(Error::JobGroupPending)?;

        for job_group in job_groups.into_iter() {
            let job_group_id = job_group.id;
            let mut group: jobsrv::JobGroup = job_group.into();

            let job_group_projects =
                GroupProject::get_group_projects(job_group_id, &conn).map_err(Error::JobGroupPending)?;
            let projects: Vec<jobsrv::JobGroupProject> =
                job_group_projects.into_iter().map(|j| j.into()).collect();

            group.set_projects(RepeatedField::from_vec(projects));
            groups.push(group);
        }

        Ok(groups)
    }

    // Get a list of jobs with un-sync status
    pub fn sync_jobs(&self) -> Result<Vec<jobsrv::Job>> {
        debug!("DataStore: sync_jobs");
        let conn = self.diesel_pool.get_conn()?;
        let results = Job::sync_jobs(&conn).map_err(Error::SyncJobs)?;
        let jobs: Vec<jobsrv::Job> = results.into_iter().map(|j| j.into()).collect();

        Ok(jobs)
    }

    pub fn set_job_sync(&self, job_id: u64) -> Result<()> {
        debug!("DataStore: set_job_sync");
        let conn = self.diesel_pool.get_conn()?;
        Job::set_job_sync(job_id as i64, &conn).map_err(Error::SyncJobs)?;

        Ok(())
    }
}
