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

use std::{io,
          sync::Arc};

use chrono::{DateTime,
             Utc};
use diesel::{result::Error as Dre,
             Connection};
use protobuf::{self,
               RepeatedField};

use crate::db::{config::DataStoreCfg,
                migration::setup_ids,
                models::jobs::{AuditJob,
                               Group,
                               GroupProject,
                               Job,
                               NewAuditJob,
                               NewJob,
                               UpdateGroupProject,
                               UpdateJob},
                pool::Pool,
                DbPool};

use crate::protocol::jobsrv;

use crate::error::{Error,
                   Result};

/// DataStore inherints being Send + Sync by virtue of having only one member, the pool itself.
#[derive(Clone)]
pub struct DataStore {
    pool:        Pool,
    diesel_pool: DbPool,
}

impl DataStore {
    /// Create a new DataStore.
    ///
    /// * Can fail if the pool cannot be created
    /// * Blocks creation of the datastore on the existince of the pool; might wait indefinetly.
    pub fn new(cfg: &DataStoreCfg) -> Self {
        let pool = Pool::new(cfg);
        let diesel_pool = DbPool::new(&cfg);
        DataStore { pool, diesel_pool }
    }

    /// Create a new DataStore from a pre-existing pool; useful for testing the database.
    pub fn from_pool(pool: Pool, diesel_pool: DbPool, _: Vec<u32>, _: Arc<String>) -> Self {
        DataStore { pool, diesel_pool }
    }

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
        let conn = self.diesel_pool.get_conn()?;

        // TODO: What job never has a channel?
        // DB schema is nullable for some reason
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

            // TODO: vcs_argument is taking an option in its vec... why?
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

            let rows = Job::create(&new_job, &conn).map_err(Error::JobCreate)?;
            let job = rows.into();
            Ok(job)
        } else {
            Err(Error::UnknownVCS)
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
        let conn = self.diesel_pool.get_conn()?;
        let id = get_job.get_id() as i64;
        let result = Job::get(id, &conn);

        let job = if let diesel::QueryResult::Err(diesel::result::Error::NotFound) = result {
            None
        } else {
            let jobsrv: jobsrv::Job = result.map_err(Error::JobGet)?.into();
            Some(jobsrv)
        };
        Ok(job)
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
        let conn = self.diesel_pool.get_conn()?;

        let result = Job::get_next_pending(worker, target, &conn);
        let job = if let diesel::QueryResult::Err(diesel::result::Error::NotFound) = result {
            None
        } else {
            let jobsrv: jobsrv::Job = result.map_err(Error::JobPending)?.into();
            Some(jobsrv)
        };
        Ok(job)
    }

    /// Get a list of cancel-pending jobs
    ///
    /// # Errors
    ///
    /// * If a connection cannot be gotten from the pool
    /// * If the cancel pending jobs cannot be selected from the database
    /// * If the row returned cannot be translated into a Job
    pub fn get_cancel_pending_jobs(&self) -> Result<Vec<jobsrv::Job>> {
        let conn = self.diesel_pool.get_conn()?;

        let result = Job::get_job_by_state("CancelPending", &conn).map_err(Error::JobPending)?;
        let jobs: Vec<jobsrv::Job> = result.into_iter().map(|x| x.into()).collect();
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
        let conn = self.diesel_pool.get_conn()?;

        let result = Job::get_job_by_state("Dispatched", &conn).map_err(Error::JobGet)?;
        let jobs: Vec<jobsrv::Job> = result.into_iter().map(|x| x.into()).collect();
        Ok(jobs)
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

        Job::update_with_sync(&job, &conn).map_err(Error::JobSetState)?;

        Ok(())
    }

    /// Marks a given job's logs as having been archived. The location
    /// and mechanism for retrieval are dependent on the configured archiving
    /// mechanism.
    pub fn mark_as_archived(&self, job_id: u64) -> Result<()> {
        let conn = self.diesel_pool.get_conn()?;
        Job::mark_as_archived(job_id as i64, &conn).map_err(Error::JobMarkArchived)?;
        Ok(())
    }

    pub fn create_job_group(&self,
                            msg: &jobsrv::JobGroupSpec, // This should someday be jobs::Group
                            project_tuples: Vec<(String, String)>)
                            -> Result<jobsrv::JobGroup> {
        let conn = self.diesel_pool.get_conn()?;

        assert!(!project_tuples.is_empty());

        let root_project = format!("{}/{}", msg.get_origin(), msg.get_package());
        let target = msg.get_target().to_string();
        let result = Group::insert_group(&root_project, &target, &project_tuples, &conn)?;
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
        let conn = self.diesel_pool.get_conn()?;
        Group::cancel_job_group(group_id as i64, &conn).map_err(Error::JobGroupCancel)?;
        Ok(())
    }

    pub fn create_audit_entry(&self, msg: &jobsrv::JobGroupAudit) -> Result<()> {
        let conn = self.diesel_pool.get_conn()?;
        let audit_job = NewAuditJob { group_id:       msg.get_group_id() as i64,
                                      operation:      msg.get_operation() as i16,
                                      trigger:        msg.get_trigger() as i16,
                                      requester_id:   msg.get_requester_id() as i64,
                                      requester_name: msg.get_requester_name(),
                                      created_at:     None, }; // TODO FIX

        AuditJob::create(&audit_job, &conn).map_err(Error::JobGroupAudit)?;

        Ok(())
    }

    pub fn get_job_group_origin(&self,
                                msg: &jobsrv::JobGroupOriginGet)
                                -> Result<jobsrv::JobGroupOriginResponse> {
        let origin = msg.get_origin();
        let limit = msg.get_limit();

        let conn = self.diesel_pool.get_conn()?;
        let rows = Group::get_all_for_origin(origin, limit as i64, &conn)
                   .map_err(Error::JobGroupOriginGet)?;

        let mut response = jobsrv::JobGroupOriginResponse::new();
        let mut job_groups = RepeatedField::new();

        for row in rows.into_iter() {
            let group: jobsrv::JobGroup = row.into();
            job_groups.push(group);
        }

        response.set_job_groups(job_groups);
        Ok(response)
    }

    pub fn get_job_group(&self, msg: &jobsrv::JobGroupGet) -> Result<Option<jobsrv::JobGroup>> {
        let group_id = msg.get_group_id() as i64;
        let include_projects = msg.get_include_projects();

        let conn = self.diesel_pool.get_conn()?;
        let mut group: jobsrv::JobGroup =
            Group::get_job_group(group_id, &conn).map_err(Error::JobGroupGet)?
                                                 .into();

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
        let conn = self.diesel_pool.get_conn()?;
        let state = group_state.to_string();
        Group::set_group_state(group_id as i64, &state, &conn).map_err(Error::JobGroupSetState)?;
        Ok(())
    }

    pub fn set_job_group_project_state(&self,
                                       group_id: u64,
                                       project_name: &str,
                                       project_state: jobsrv::JobGroupProjectState)
                                       -> Result<()> {
        let conn = self.diesel_pool.get_conn()?;

        GroupProject::update_group_project_state(group_id as i64,
                                                 project_name,
                                                 project_state,
                                                 &conn).map_err(Error::JobGroupProjectSetState)?;
        Ok(())
    }

    pub fn set_job_group_job_state(&self, job: &jobsrv::Job) -> Result<()> {
        let conn = self.diesel_pool.get_conn()?;
        let group_project =
            GroupProject::get_group_project_by_name(job.get_owner_id() as i64,
                                                    &job.get_project().get_name(),
                                                    &conn).map_err(Error::JobGroupProjectSetState)?;
        // TODO Capture not found and returnErr(Error::UnknownJobGroupProjectState);

        // TODO This should not be here; we need first class types
        // Mapping JobState to a string version of JobGroupState
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

        let update = UpdateGroupProject { id:            group_project.id,
                                          project_state: state.to_string(),
                                          job_id:        job.get_id() as i64,
                                          project_ident: update_ident,
                                          updated_at:    None, /* TODO THIS MIGHT NEED TO BE
                                                                * FILLED OUT BY US, figure this
                                                                * out! */ };

        GroupProject::update_group_project(&update, &conn)?;

        Ok(())
    }

    pub fn pending_job_groups(&self, count: i32) -> Result<Vec<jobsrv::JobGroup>> {
        let mut groups = Vec::new();

        let conn = self.diesel_pool.get_conn()?;

        let group_rows = Group::pending_job_groups(count, &conn).map_err(Error::JobGroupPending)?;

        for group_row in group_rows.into_iter() {
            let row_id = group_row.id;
            let mut group: jobsrv::JobGroup = group_row.into();

            let projects =
                GroupProject::get_group_projects(row_id, &conn).map_err(Error::JobGroupPending)?;
            let projects: Vec<jobsrv::JobGroupProject> =
                projects.into_iter().map(|x| x.into()).collect();

            group.set_projects(RepeatedField::from_vec(projects));
            groups.push(group);
        }

        Ok(groups)
    }

    pub fn sync_jobs(&self) -> Result<Vec<jobsrv::Job>> {
        let conn = self.diesel_pool.get_conn()?;
        let rows = Job::sync_jobs(&conn).map_err(Error::SyncJobs)?;
        let result: Vec<jobsrv::Job> = rows.into_iter().map(|x| x.into()).collect();
        Ok(result)
    }

    pub fn set_job_sync(&self, job_id: u64) -> Result<()> {
        let conn = self.diesel_pool.get_conn()?;
        Job::set_job_sync(job_id as i64, &conn).map_err(Error::SyncJobs)?;
        Ok(())
    }
}
