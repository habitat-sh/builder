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
               ProtobufEnum,
               RepeatedField};

use crate::db::{config::DataStoreCfg,
                migration::setup_ids,
                pool::Pool,
                DbPool};

use crate::protocol::{jobsrv,
                      net::{ErrCode,
                            NetError},
                      originsrv};

use crate::error::{Error,
                   Result};

mod test;

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

    // #[cfg(test)] // TODO figure out if we  have to expose this for SchedulerDataStoreDb
    pub fn get_pool(&self) -> habitat_builder_db::diesel_pool::DbPool { self.diesel_pool.clone() }

    /// Create a new DataStore from a pre-existing pool; useful for testing the database.
    pub fn from_pool(pool: Pool, diesel_pool: DbPool) -> Self { DataStore { pool, diesel_pool } }

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
        let conn = self.pool.get()?;

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

            let rows = conn.query("SELECT * FROM insert_job_v3($1, $2, $3, $4, $5, $6, $7, $8, \
                                   $9)",
                                  &[&(job.get_owner_id() as i64),
                                    &(project.get_id() as i64),
                                    &project.get_name(),
                                    &(project.get_owner_id() as i64),
                                    &project.get_plan_path(),
                                    &project.get_vcs_type(),
                                    &vec![Some(project.get_vcs_data().to_string()), install_id],
                                    &channel,
                                    &job.get_target()])
                           .map_err(Error::JobCreate)?;
            let job = row_to_job(&rows.get(0))?;
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
        let conn = self.pool.get()?;
        let rows = &conn.query("SELECT * FROM get_job_v1($1)",
                               &[&(get_job.get_id() as i64)])
                        .map_err(Error::JobGet)?;

        if !rows.is_empty() {
            let row = rows.get(0);
            let job = row_to_job(&row)?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
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
        let conn = self.pool.get()?;
        let rows = &conn.query("SELECT * FROM next_pending_job_v2($1, $2)",
                               &[&worker, &target])
                        .map_err(Error::JobPending)?;
        if !rows.is_empty() {
            let row = rows.get(0);
            let job = row_to_job(&row)?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    /// Get a list of cancel-pending jobs
    ///
    /// # Errors
    ///
    /// * If a connection cannot be gotten from the pool
    /// * If the cancel pending jobs cannot be selected from the database
    /// * If the row returned cannot be translated into a Job
    pub fn get_cancel_pending_jobs(&self) -> Result<Vec<jobsrv::Job>> {
        let mut jobs = Vec::new();
        let conn = self.pool.get()?;
        let rows = &conn.query("SELECT * FROM get_cancel_pending_jobs_v1()", &[])
                        .map_err(Error::JobPending)?;
        for row in rows {
            let job = row_to_job(&row)?;
            jobs.push(job);
        }
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
        let mut jobs = Vec::new();
        let conn = self.pool.get()?;
        let rows = &conn.query("SELECT * FROM get_dispatched_jobs_v1()", &[])
                        .map_err(Error::JobGet)?;
        for row in rows {
            let job = row_to_job(&row)?;
            jobs.push(job);
        }
        Ok(jobs)
    }

    /// Count the number of jobs in a given state
    ///
    /// # Errors
    ///
    /// * If a connection cannot be gotten from the pool
    pub fn count_jobs(&self, job_state: jobsrv::JobState) -> Result<i64> {
        let conn = self.pool.get()?;
        let rows = &conn.query("SELECT * FROM count_jobs_v1($1)", &[&job_state.to_string()])
                        .map_err(Error::JobGet)?;
        assert!(rows.len() == 1);
        let count: i64 = rows.get(0).get("count_jobs_v1");
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
        let conn = self.pool.get()?;
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
            (Some(job.get_error().get_code() as i32), Some(job.get_error().get_msg()))
        } else {
            (None, None)
        };

        conn.execute("SELECT update_job_v3($1, $2, $3, $4, $5, $6, $7)",
                     &[&job_id,
                       &job_state,
                       &build_started_at,
                       &build_finished_at,
                       &ident,
                       &err_code,
                       &err_msg])
            .map_err(Error::JobSetState)?;

        Ok(())
    }

    /// Marks a given job's logs as having been archived. The location
    /// and mechanism for retrieval are dependent on the configured archiving
    /// mechanism.
    pub fn mark_as_archived(&self, job_id: u64) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute("SELECT mark_as_archived_v1($1)", &[&(job_id as i64)])
            .map_err(Error::JobMarkArchived)?;
        Ok(())
    }

    pub fn create_job_group(&self,
                            msg: &jobsrv::JobGroupSpec,
                            project_tuples: Vec<(String, String)>)
                            -> Result<jobsrv::JobGroup> {
        let conn = self.pool.get()?;

        assert!(!project_tuples.is_empty());

        let root_project = format!("{}/{}", msg.get_origin(), msg.get_package());

        let (project_names, project_idents): (Vec<String>, Vec<String>) =
            project_tuples.iter().cloned().unzip();

        let rows = conn.query("SELECT * FROM insert_group_v3($1, $2, $3, $4)",
                              &[&root_project,
                                &project_names,
                                &project_idents,
                                &msg.get_target()])
                       .map_err(Error::JobGroupCreate)?;

        let mut group = self.row_to_job_group(&rows.get(0))?;
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

        debug!("JobGroup created: {:?}", group);

        Ok(group)
    }

    pub fn cancel_job_group(&self, group_id: u64) -> Result<()> {
        let conn = self.pool.get()?;
        conn.query("SELECT cancel_group_v1($1)", &[&(group_id as i64)])
            .map_err(Error::JobGroupCancel)?;

        Ok(())
    }

    pub fn create_audit_entry(&self, msg: &jobsrv::JobGroupAudit) -> Result<()> {
        let conn = self.pool.get()?;
        conn.query("SELECT add_audit_jobs_entry_v1($1, $2, $3, $4, $5)",
                   &[&(msg.get_group_id() as i64),
                     &(msg.get_operation() as i16),
                     &(msg.get_trigger() as i16),
                     &(msg.get_requester_id() as i64),
                     &msg.get_requester_name().to_string()])
            .map_err(Error::JobGroupAudit)?;

        Ok(())
    }

    pub fn get_job_group_origin(&self,
                                msg: &jobsrv::JobGroupOriginGet)
                                -> Result<jobsrv::JobGroupOriginResponse> {
        let origin = msg.get_origin();
        let limit = msg.get_limit();

        let conn = self.pool.get()?;
        let rows = &conn.query("SELECT * FROM get_job_groups_for_origin_v2($1, $2)",
                               &[&origin, &(limit as i32)])
                        .map_err(Error::JobGroupOriginGet)?;

        let mut response = jobsrv::JobGroupOriginResponse::new();
        let mut job_groups = RepeatedField::new();

        for row in rows {
            let group = self.row_to_job_group(&row)?;
            job_groups.push(group);
        }

        response.set_job_groups(job_groups);
        Ok(response)
    }

    pub fn get_job_group(&self, msg: &jobsrv::JobGroupGet) -> Result<Option<jobsrv::JobGroup>> {
        let group_id = msg.get_group_id();
        let include_projects = msg.get_include_projects();

        let conn = self.pool.get()?;
        let rows = &conn.query("SELECT * FROM get_group_v1($1)", &[&(group_id as i64)])
                        .map_err(Error::JobGroupGet)?;

        if rows.is_empty() {
            warn!("JobGroup id {} not found", group_id);
            return Ok(None);
        }

        assert!(rows.len() == 1); // should never have more than one

        let mut group = self.row_to_job_group(&rows.get(0))?;

        if include_projects {
            let project_rows = &conn.query("SELECT * FROM get_group_projects_for_group_v1($1)",
                                           &[&(group_id as i64)])
                                    .map_err(Error::JobGroupGet)?;

            assert!(!project_rows.is_empty()); // should at least have one
            let projects = self.rows_to_job_group_projects(&project_rows)?;

            group.set_projects(projects);
        }

        Ok(Some(group))
    }

    fn row_to_job_group(&self, row: &postgres::rows::Row) -> Result<jobsrv::JobGroup> {
        let mut group = jobsrv::JobGroup::new();

        let id: i64 = row.get("id");
        group.set_id(id as u64);
        let js: String = row.get("group_state");
        let group_state = js.parse::<jobsrv::JobGroupState>()?;
        group.set_state(group_state);

        let created_at = row.get::<&str, DateTime<Utc>>("created_at");
        group.set_created_at(created_at.to_rfc3339());

        let project_name: String = row.get("project_name");
        group.set_project_name(project_name);

        let target: String = row.get("target");
        group.set_target(target);

        Ok(group)
    }

    fn row_to_job_group_project(&self,
                                row: &postgres::rows::Row)
                                -> Result<jobsrv::JobGroupProject> {
        let mut project = jobsrv::JobGroupProject::new();

        let name: String = row.get("project_name");
        let ident: String = row.get("project_ident");
        let state: String = row.get("project_state");
        let job_id: i64 = row.get("job_id");
        let target: String = row.get("target");
        let project_state = state.parse::<jobsrv::JobGroupProjectState>()?;

        project.set_name(name);
        project.set_ident(ident);
        project.set_state(project_state);
        project.set_target(target);
        project.set_job_id(job_id as u64);

        Ok(project)
    }

    fn rows_to_job_group_projects(&self,
                                  rows: &postgres::rows::Rows)
                                  -> Result<RepeatedField<jobsrv::JobGroupProject>> {
        let mut projects = RepeatedField::new();

        for row in rows {
            let project = self.row_to_job_group_project(&row)?;
            projects.push(project);
        }

        Ok(projects)
    }

    pub fn set_job_group_state(&self,
                               group_id: u64,
                               group_state: jobsrv::JobGroupState)
                               -> Result<()> {
        let conn = self.pool.get()?;
        let state = group_state.to_string();
        conn.execute("SELECT set_group_state_v1($1, $2)",
                     &[&(group_id as i64), &state])
            .map_err(Error::JobGroupSetState)?;
        Ok(())
    }

    pub fn set_job_group_project_state(&self,
                                       group_id: u64,
                                       project_name: &str,
                                       project_state: jobsrv::JobGroupProjectState)
                                       -> Result<()> {
        let conn = self.pool.get()?;
        let state = project_state.to_string();
        conn.execute("SELECT set_group_project_name_state_v1($1, $2, $3)",
                     &[&(group_id as i64), &project_name, &state])
            .map_err(Error::JobGroupProjectSetState)?;
        Ok(())
    }

    pub fn set_job_group_job_state(&self, job: &jobsrv::Job) -> Result<()> {
        let conn = self.pool.get()?;
        let rows = &conn.query("SELECT * FROM find_group_project_v1($1, $2)",
                               &[&(job.get_owner_id() as i64), &job.get_project().get_name()])
                        .map_err(Error::JobGroupProjectSetState)?;

        // No rows means this job might not be one we care about
        if rows.is_empty() {
            warn!("No project found for job id: {}", job.get_id());
            return Err(Error::UnknownJobGroupProjectState);
        }

        assert!(rows.len() == 1); // should never have more than one
        let pid: i64 = rows.get(0).get("id");

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

        if job.get_state() == jobsrv::JobState::Complete {
            let ident = job.get_package_ident().to_string();

            conn.execute("SELECT set_group_project_state_ident_v1($1, $2, $3, $4)",
                         &[&pid, &(job.get_id() as i64), &state, &ident])
                .map_err(Error::JobGroupProjectSetState)?;
        } else {
            conn.execute("SELECT set_group_project_state_v1($1, $2, $3)",
                         &[&pid, &(job.get_id() as i64), &state])
                .map_err(Error::JobGroupProjectSetState)?;
        };

        Ok(())
    }

    pub fn pending_job_groups(&self, count: i32) -> Result<Vec<jobsrv::JobGroup>> {
        let mut groups = Vec::new();

        let conn = self.pool.get()?;
        let group_rows = &conn.query("SELECT * FROM pending_groups_v1($1)", &[&count])
                              .map_err(Error::JobGroupPending)?;

        for group_row in group_rows {
            let mut group = self.row_to_job_group(&group_row)?;

            let project_rows = &conn.query("SELECT * FROM get_group_projects_for_group_v1($1)",
                                           &[&(group.get_id() as i64)])
                                    .map_err(Error::JobGroupPending)?;
            let projects = self.rows_to_job_group_projects(&project_rows)?;

            group.set_projects(projects);
            groups.push(group);
        }

        Ok(groups)
    }

    pub fn sync_jobs(&self) -> Result<Vec<jobsrv::Job>> {
        let mut jobs = Vec::new();
        let conn = self.pool.get()?;

        let rows = &conn.query("SELECT * FROM sync_jobs_v2()", &[])
                        .map_err(Error::SyncJobs)?;

        for row in rows.iter() {
            match row_to_job(&row) {
                Ok(job) => jobs.push(job),
                Err(e) => {
                    warn!("Failed to convert row to job {}", e);
                }
            };
        }

        Ok(jobs)
    }

    pub fn set_job_sync(&self, job_id: u64) -> Result<()> {
        let conn = self.pool.get()?;

        conn.query("SELECT * FROM set_jobs_sync_v2($1)", &[&(job_id as i64)])
            .map_err(Error::SyncJobs)?;

        Ok(())
    }
}

/// Translate a database `jobs` row to a `jobsrv::Job`.
///
/// # Errors
///
/// * If the job state is unknown
/// * If the VCS type is unknown
fn row_to_job(row: &postgres::rows::Row) -> Result<jobsrv::Job> {
    let mut job = jobsrv::Job::new();
    let id: i64 = row.get("id");
    job.set_id(id as u64);
    let owner_id: i64 = row.get("owner_id");
    job.set_owner_id(owner_id as u64);

    let js: String = row.get("job_state");
    let job_state: jobsrv::JobState = js.parse().map_err(Error::UnknownJobState)?;
    job.set_state(job_state);

    let created_at = row.get::<&str, DateTime<Utc>>("created_at");
    job.set_created_at(created_at.to_rfc3339());

    // Note: these may be null (e.g., a job is scheduled, but hasn't
    // started; a job has started and is currently running)
    if let Some(Ok(start)) = row.get_opt::<&str, DateTime<Utc>>("build_started_at") {
        job.set_build_started_at(start.to_rfc3339());
    }
    if let Some(Ok(stop)) = row.get_opt::<&str, DateTime<Utc>>("build_finished_at") {
        job.set_build_finished_at(stop.to_rfc3339());
    }

    // package_ident will only be present if the build succeeded
    if let Some(Ok(ident_str)) = row.get_opt::<&str, String>("package_ident") {
        let ident: originsrv::OriginPackageIdent = ident_str.parse().unwrap();
        job.set_package_ident(ident);
    }

    let mut project = originsrv::OriginProject::new();
    let project_id: i64 = row.get("project_id");
    project.set_id(project_id as u64);

    // only 'project_name' exists in the jobs table, but it's just
    // "origin/name", so we can set those fields in the Project
    // struct.
    //
    // 'package_ident' may be null, though, so we shouldn't use it to
    // get the origin and name.
    let name: String = row.get("project_name");
    let name_for_split = name.clone();
    let name_split: Vec<&str> = name_for_split.split('/').collect();
    project.set_origin_name(name_split[0].to_string());
    project.set_package_name(name_split[1].to_string());
    project.set_name(name);

    let project_owner_id: i64 = row.get("project_owner_id");
    project.set_owner_id(project_owner_id as u64);
    project.set_plan_path(row.get("project_plan_path"));

    let rvcs: String = row.get("vcs");
    match rvcs.as_ref() {
        "git" => {
            let mut vcsa: Vec<Option<String>> = row.get("vcs_arguments");
            project.set_vcs_type(String::from("git"));
            project.set_vcs_data(vcsa.remove(0).expect("expected vcs data"));
            if !vcsa.is_empty() {
                if let Some(install_id) = vcsa.remove(0) {
                    project.set_vcs_installation_id(
                        install_id
                            .parse::<u32>()
                            .map_err(Error::ParseVCSInstallationId)?,
                    );
                }
            }
        }
        e => {
            error!("Unknown VCS, {}", e);
            return Err(Error::UnknownVCS);
        }
    }
    job.set_project(project);

    if let Some(Ok(err_msg)) = row.get_opt::<&str, String>("net_error_msg") {
        let err_code: i32 = row.get("net_error_code");
        let mut err = NetError::new();

        if let Some(net_err_code) = ErrCode::from_i32(err_code) {
            err.set_code(net_err_code);
            err.set_msg(err_msg);
            job.set_error(err);
        }
    }

    job.set_is_archived(row.get("archived"));

    if let Some(Ok(channel)) = row.get_opt::<&str, String>("channel") {
        job.set_channel(channel);
    };

    if let Some(Ok(worker)) = row.get_opt::<&str, String>("worker") {
        job.set_worker(worker);
    };

    let target: String = row.get("target");
    job.set_target(target);

    Ok(job)
}
