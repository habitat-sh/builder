use super::db_id_format;
use chrono::prelude::*;
use diesel::{deserialize::{self,
                           FromSql,
                           Queryable},
             dsl::count_star,
             pg::{Pg,
                  PgConnection},
             result::QueryResult,
             BoolExpressionMethods,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl,
             TextExpressionMethods};

use diesel::sql_types::{Array,
                        BigInt,
                        Bool,
                        Integer,
                        Nullable,
                        Record,
                        Text,
                        Timestamptz};

use protobuf::ProtobufEnum;

use crate::protocol::{jobsrv,
                      net,
                      originsrv};

use crate::{models::{package::{BuilderPackageIdent,
                               BuilderPackageTarget},
                     pagination::Paginate},
            schema::jobs::{audit_jobs,
                           group_projects,
                           groups,
                           job_graph,
                           jobs}};

use crate::{bldr_core::{metrics::{CounterMetric,
                                  HistogramMetric},
                        Error as BuilderError},
            functions::jobs as job_functions,
            hab_core::package::PackageTarget,
            metrics::{Counter,
                      Histogram},
            metrics_instrument_block};

use std::{fmt,
          str::FromStr,
          time::Instant};

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "jobs"]
pub struct Job {
    #[serde(with = "db_id_format")]
    pub id:                i64,
    #[serde(with = "db_id_format")]
    pub owner_id:          i64,
    pub job_state:         String,
    #[serde(with = "db_id_format")]
    pub project_id:        i64,
    pub project_name:      String,
    #[serde(with = "db_id_format")]
    pub project_owner_id:  i64,
    pub project_plan_path: String,
    pub vcs:               String,
    pub vcs_arguments:     Vec<Option<String>>,
    pub net_error_code:    Option<i32>,
    pub net_error_msg:     Option<String>,
    pub scheduler_sync:    bool,
    pub created_at:        Option<DateTime<Utc>>,
    pub updated_at:        Option<DateTime<Utc>>,
    pub build_started_at:  Option<DateTime<Utc>>,
    pub build_finished_at: Option<DateTime<Utc>>,
    pub package_ident:     Option<String>,
    pub archived:          bool,
    pub channel:           Option<String>,
    pub sync_count:        i32,
    pub worker:            Option<String>,
    pub target:            String,
}

#[derive(Insertable)]
#[table_name = "jobs"]
pub struct NewJob<'a> {
    pub owner_id:          i64,
    pub project_id:        i64,
    pub project_name:      &'a str,
    pub job_state:         &'a str,
    pub project_owner_id:  i64,
    pub project_plan_path: &'a str,
    pub vcs:               &'a str,
    pub vcs_arguments:     &'a Vec<Option<&'a str>>,
    // This would be ChannelIdent, but Insertable requires implementing diesel::Expression
    pub channel:           Option<&'a str>,
    pub target:            &'a str,
}

#[derive(AsChangeset)]
#[table_name = "jobs"]
pub struct UpdateJob {
    pub id:                i64,
    pub job_state:         String,
    pub net_error_code:    Option<i32>,
    pub net_error_msg:     Option<String>,
    pub build_started_at:  Option<DateTime<Utc>>,
    pub build_finished_at: Option<DateTime<Utc>>,
    pub package_ident:     Option<String>,
}

type JobFields = (BigInt,
                  BigInt,
                  Text,
                  BigInt,
                  Text,
                  BigInt,
                  Text,
                  Text,
                  Array<Nullable<Text>>,
                  Nullable<Integer>,
                  Nullable<Text>,
                  Bool,
                  Nullable<Timestamptz>,
                  Nullable<Timestamptz>,
                  Nullable<Timestamptz>,
                  Nullable<Timestamptz>,
                  Nullable<Text>,
                  Bool,
                  Nullable<Text>,
                  Integer,
                  Nullable<Text>,
                  Text);
pub type JobRecord = Record<JobFields>;

pub struct ListProjectJobs {
    pub name:  String,
    pub page:  i64,
    pub limit: i64,
}

impl Job {
    pub fn get(id: i64, conn: &PgConnection) -> QueryResult<Job> {
        Counter::DBCall.increment();
        jobs::table.filter(jobs::id.eq(id)).get_result(conn)
    }

    pub fn list(lpj: ListProjectJobs, conn: &PgConnection) -> QueryResult<(Vec<Job>, i64)> {
        jobs::table.filter(jobs::project_name.eq(lpj.name))
                   .order(jobs::created_at.desc())
                   .paginate(lpj.page)
                   .per_page(lpj.limit)
                   .load_and_count_records(conn)
    }

    pub fn create(job: &NewJob, conn: &PgConnection) -> QueryResult<Job> {
        Counter::DBCall.increment();
        diesel::insert_into(jobs::table).values(job)
                                        .get_result(conn)
    }

    pub fn count(job_state: jobsrv::JobState,
                 target: PackageTarget,
                 conn: &PgConnection)
                 -> QueryResult<i64> {
        Counter::DBCall.increment();
        jobs::table.select(count_star())
                   .filter(jobs::job_state.eq(job_state.to_string()))
                   .filter(jobs::target.eq(target.to_string()))
                   .first(conn)
    }

    pub fn set_job_sync(job_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(jobs::table.find(job_id)).set((jobs::scheduler_sync.eq(true),
                                                      jobs::sync_count.eq(jobs::sync_count - 1)))
                                                .execute(conn)
    }

    pub fn get_jobs_by_state(job_state: jobsrv::JobState,
                             conn: &PgConnection)
                             -> QueryResult<Vec<Job>> {
        Counter::DBCall.increment();
        jobs::table.filter(jobs::job_state.eq(job_state.to_string()))
                   .load(conn)
    }

    pub fn count_jobs(job_state: jobsrv::JobState, conn: &PgConnection) -> QueryResult<i64> {
        Counter::DBCall.increment();
        jobs::table.select(count_star())
                   .filter(jobs::job_state.eq(job_state.to_string()))
                   .first(conn)
    }

    pub fn mark_as_archived(job_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        diesel::update(jobs::table.find(job_id)).set(jobs::archived.eq(true))
                                                .execute(conn)
    }

    // Get the jobs with un-sync status
    pub fn sync_jobs(conn: &PgConnection) -> QueryResult<Vec<Job>> {
        Counter::DBCall.increment();
        jobs::table.filter(jobs::scheduler_sync.eq(false).or(jobs::sync_count.gt(0)))
                   .load(conn)
    }

    pub fn update_job_with_sync(job: &UpdateJob, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(jobs::table.find(job.id))
            .set((
                jobs::job_state.eq(&job.job_state),
                jobs::scheduler_sync.eq(false),
                jobs::sync_count.eq(jobs::sync_count + 1),
                jobs::build_started_at.eq(job.build_started_at),
                jobs::build_finished_at.eq(job.build_finished_at),
                jobs::package_ident.eq(&job.package_ident),
                jobs::net_error_code.eq(job.net_error_code),
                jobs::net_error_msg.eq(&job.net_error_msg),
            ))
            .execute(conn)
    }

    pub fn get_next_pending_job(worker: &str,
                                target: &str,
                                conn: &PgConnection)
                                -> QueryResult<Job> {
        Counter::DBCall.increment();
        let query = diesel::select(job_functions::next_pending_job_v2(worker, target));
        let job = query.first::<Job>(conn)?;

        Ok(job)
    }
}

impl FromSql<JobRecord, Pg> for Job {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let tuple = <_ as FromSql<JobRecord, Pg>>::from_sql(bytes)?;
        Ok(<Self as Queryable<JobFields, Pg>>::build(tuple))
    }
}

#[allow(clippy::from_over_into)]
impl Into<jobsrv::Job> for Job {
    fn into(self) -> jobsrv::Job {
        let mut job = jobsrv::Job::new();
        job.set_id(self.id as u64);
        job.set_owner_id(self.owner_id as u64);

        let job_state: jobsrv::JobState = self.job_state.parse().unwrap();
        job.set_state(job_state);

        job.set_created_at(self.created_at.unwrap().to_rfc3339());

        if let Some(start) = self.build_started_at {
            job.set_build_started_at(start.to_rfc3339());
        }
        if let Some(stop) = self.build_finished_at {
            job.set_build_finished_at(stop.to_rfc3339());
        }

        if let Some(ident_str) = self.package_ident {
            let ident: originsrv::OriginPackageIdent = ident_str.parse().unwrap();
            job.set_package_ident(ident);
        }

        let mut project = originsrv::OriginProject::new();
        project.set_id(self.project_id as u64);

        let name = self.project_name.clone();
        let name_for_split = self.project_name.clone();
        let name_split: Vec<&str> = name_for_split.split('/').collect();
        project.set_origin_name(name_split[0].to_string());
        project.set_package_name(name_split[1].to_string());
        project.set_name(name);

        project.set_owner_id(self.project_owner_id as u64);
        project.set_plan_path(self.project_plan_path.clone());

        match self.vcs.as_ref() {
            "git" => {
                let mut vcsa: Vec<Option<String>> = self.vcs_arguments;
                project.set_vcs_type(String::from("git"));
                project.set_vcs_data(vcsa.remove(0).expect("expected vcs data"));
                if !vcsa.is_empty() {
                    if let Some(install_id) = vcsa.remove(0) {
                        project.set_vcs_installation_id(install_id.parse::<u32>().unwrap());
                    }
                }
            }
            e => error!("Unknown VCS, {}", e),
        }
        job.set_project(project);

        if let Some(err_msg) = self.net_error_msg {
            let mut err = net::NetError::new();

            if let Some(net_err_code) = net::ErrCode::from_i32(self.net_error_code.unwrap()) {
                err.set_code(net_err_code);
                err.set_msg(err_msg);
                job.set_error(err);
            }
        }

        job.set_is_archived(self.archived);

        if let Some(channel) = self.channel {
            job.set_channel(channel);
        };

        if let Some(worker) = self.worker {
            job.set_worker(worker);
        };

        job.set_target(self.target.clone());
        job
    }
}

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "groups"]
pub struct Group {
    #[serde(with = "db_id_format")]
    pub id:           i64,
    pub group_state:  String, // This should be jobsrv::JobGroupState
    pub project_name: String,
    pub created_at:   Option<DateTime<Utc>>,
    pub updated_at:   Option<DateTime<Utc>>,
    pub target:       String,
}

#[derive(Insertable)]
#[table_name = "groups"]
pub struct NewGroup<'a> {
    pub group_state:  &'a str,
    pub project_name: &'a str,
    pub target:       &'a str,
}

pub type GroupFields = (BigInt, Text, Text, Nullable<Timestamptz>, Nullable<Timestamptz>, Text);

pub type GroupRecord = Record<GroupFields>;

impl Group {
    pub fn create(group: &NewGroup, conn: &PgConnection) -> QueryResult<Group> {
        Counter::DBCall.increment();
        diesel::insert_into(groups::table).values(group)
                                          .get_result(conn)
    }

    pub fn get(id: i64, conn: &PgConnection) -> QueryResult<Group> {
        Counter::DBCall.increment();
        groups::table.filter(groups::id.eq(id)).get_result(conn)
    }

    pub fn get_queued(project_name: &str, target: &str, conn: &PgConnection) -> QueryResult<Group> {
        Counter::DBCall.increment();
        groups::table.filter(groups::project_name.eq(project_name))
                     .filter(groups::group_state.eq("Queued"))
                     .filter(groups::target.eq(target))
                     .get_result(conn)
    }

    pub fn get_all_queued(target: PackageTarget, conn: &PgConnection) -> QueryResult<Vec<Group>> {
        Counter::DBCall.increment();
        groups::table.filter(groups::group_state.eq("Queued"))
                     .filter(groups::target.eq(target.to_string()))
                     .get_results(conn)
    }

    pub fn get_all_dispatching(target: PackageTarget,
                               conn: &PgConnection)
                               -> QueryResult<Vec<Group>> {
        Counter::DBCall.increment();
        groups::table.filter(groups::group_state.eq("Dispatching"))
                     .filter(groups::target.eq(target.to_string()))
                     .get_results(conn)
    }

    pub fn get_pending(target: PackageTarget, conn: &PgConnection) -> QueryResult<Group> {
        Counter::DBCall.increment();
        groups::table.filter(groups::group_state.eq("Pending"))
                     .filter(groups::target.eq(target.to_string()))
                     .order(groups::created_at.asc())
                     .get_result(conn)
    }

    pub fn get_active(project_name: &str,
                      target: PackageTarget,
                      conn: &PgConnection)
                      -> QueryResult<Group> {
        Counter::DBCall.increment();
        groups::table.filter(groups::group_state.eq("Pending")
                                                .or(groups::group_state.eq("Dispatching")))
                     .filter(groups::target.eq(target.to_string()))
                     .filter(groups::project_name.eq(project_name))
                     .get_result(conn)
    }

    pub fn take_next_group_for_target(target: PackageTarget,
                                      conn: &PgConnection)
                                      -> QueryResult<Option<Group>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        // This might need to be a transaction if we wanted an async scheduler, but for now the
        // scheduler is single threaded. There is a possible race condition if a cancellation
        // message arrives just as the job is being dispatched. Alternately we can route all
        // cancelation through the scheduler, which would serialize things.
        let next_group: diesel::QueryResult<Group> =
            groups::table.filter(groups::group_state.eq("Queued"))
                     .filter(groups::target.eq(target.to_string()))
                     // This would change if we want a more sophisticated priority scheme for jobs           
                     .order(groups::created_at.asc())
                     .limit(1)
                     .get_result(conn);
        let result = match next_group {
            Ok(group) => {
                diesel::update(groups::table.filter(groups::id.eq(group.id)))
                    .set(groups::group_state.eq("Dispatching"))
                    .execute(conn)?;
                diesel::QueryResult::Ok(Some(group))
            }
            diesel::QueryResult::Err(diesel::result::Error::NotFound) => {
                diesel::QueryResult::Ok(None)
            }
            diesel::QueryResult::Err(x) => diesel::QueryResult::<Option<Group>>::Err(x),
        };

        let duration_millis = (start_time.elapsed().as_micros() as f64) / 1_000.0;
        trace!("DBCall Group ::take_next_group_for_target call time {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis);
        Histogram::GroupTakeNextGroupForTargetCallTime.set(duration_millis);
        result
    }

    pub fn get_job_group(group_id: i64, conn: &PgConnection) -> QueryResult<Group> {
        Counter::DBCall.increment();
        groups::table.filter(groups::id.eq(group_id)).first(conn)
    }

    pub fn set_group_state(group_id: i64,
                           group_state: jobsrv::JobGroupState,
                           conn: &PgConnection)
                           -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(groups::table.find(group_id))
            .set(groups::group_state.eq(group_state.to_string()))
            .execute(conn)
    }

    pub fn cancel_job_group(group_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(groups::table.find(group_id)).set(groups::group_state.eq("Canceled"))
                                                    .execute(conn)
    }

    pub fn get_job_groups_for_origin(origin: &str,
                                     limit: i64,
                                     conn: &PgConnection)
                                     -> QueryResult<Vec<Group>> {
        Counter::DBCall.increment();
        let origin_match = origin.to_owned() + "/%";
        groups::table.filter(groups::project_name.like(&origin_match))
                     .order(groups::created_at.desc())
                     .limit(limit)
                     .get_results(conn)
    }

    pub fn create_job_group(root_project: &str,
                            target: &str,
                            project_names: &[String],
                            project_idents: &[String],
                            conn: &PgConnection)
                            -> QueryResult<Group> {
        Counter::DBCall.increment();

        let query = diesel::select(job_functions::insert_group_v3(root_project,
                                                                  project_names,
                                                                  project_idents,
                                                                  target.to_string()));

        let result = query.get_result::<Group>(conn)?;
        Ok(result)
    }

    pub fn pending_job_groups(count: i32, conn: &PgConnection) -> QueryResult<Vec<Group>> {
        Counter::DBCall.increment();
        let result =
            diesel::select(job_functions::pending_groups_v1(count)).get_result::<Vec<Group>>(conn)?;
        Ok(result)
    }
}

impl FromSql<GroupRecord, Pg> for Group {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let tuple = <_ as FromSql<GroupRecord, Pg>>::from_sql(bytes)?;
        Ok(<Self as Queryable<GroupFields, Pg>>::build(tuple))
    }
}

#[allow(clippy::from_over_into)]
impl Into<jobsrv::JobGroup> for Group {
    fn into(self) -> jobsrv::JobGroup {
        let mut group = jobsrv::JobGroup::new();

        group.set_id(self.id as u64);

        let group_state = self.group_state.parse::<jobsrv::JobGroupState>().unwrap();
        group.set_state(group_state);
        group.set_created_at(self.created_at.unwrap().to_rfc3339());
        group.set_project_name(self.project_name);
        group.set_target(self.target);

        group
    }
}

////////////////////

#[derive(Clone, Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "audit_jobs"]
pub struct AuditJob {
    pub group_id:       i64,
    pub operation:      i16,
    pub trigger:        i16,
    pub requester_id:   i64,
    pub requester_name: String,
    pub created_at:     Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[table_name = "audit_jobs"]
pub struct NewAuditJob<'a> {
    pub group_id:       i64,
    pub operation:      i16,
    pub trigger:        i16,
    pub requester_id:   i64,
    pub requester_name: &'a str,
}

impl AuditJob {
    pub fn create(audit_job: &NewAuditJob, conn: &PgConnection) -> QueryResult<AuditJob> {
        Counter::DBCall.increment();
        diesel::insert_into(audit_jobs::table).values(audit_job)
                                              .get_result(conn)
    }

    pub fn get_for_group(g_id: i64, conn: &PgConnection) -> QueryResult<Vec<AuditJob>> {
        Counter::DBCall.increment();
        audit_jobs::table.filter(audit_jobs::group_id.eq(g_id))
                         .get_results(conn)
    }
}

pub struct NewBusyWorker<'a> {
    pub target:      &'a str,
    pub ident:       &'a str,
    pub job_id:      i64,
    pub quarantined: bool,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "group_projects"]
pub struct GroupProject {
    pub id:            i64,
    pub owner_id:      i64, // This is the id of the associated group (should have been group_id)
    pub project_name:  String,
    pub project_ident: String,
    pub project_state: String, // Enum?
    pub job_id:        i64,
    pub target:        String, // PackageTarget?
    pub created_at:    Option<DateTime<Utc>>,
    pub updated_at:    Option<DateTime<Utc>>,
}

#[derive(AsChangeset)]
#[table_name = "group_projects"]
pub struct UpdateGroupProject {
    pub id:            i64,
    pub project_state: String,
    pub job_id:        i64,
    pub project_ident: Option<String>,
}

impl GroupProject {
    pub fn get_group_projects(group_id: i64,
                              conn: &PgConnection)
                              -> QueryResult<Vec<GroupProject>> {
        Counter::DBCall.increment();
        group_projects::table.filter(group_projects::owner_id.eq(group_id))
                             .get_results(conn)
    }

    pub fn set_group_project_state(group_id: i64,
                                   project_name: &str,
                                   project_state: jobsrv::JobGroupProjectState,
                                   conn: &PgConnection)
                                   -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(
            group_projects::table
                .filter(group_projects::owner_id.eq(group_id))
                .filter(group_projects::project_name.eq(project_name)),
        )
        .set(group_projects::project_state.eq(project_state.to_string()))
        .execute(conn)
    }

    pub fn get_group_project_by_name(group_id: i64,
                                     name: &str,
                                     conn: &PgConnection)
                                     -> QueryResult<GroupProject> {
        Counter::DBCall.increment();
        let query = group_projects::table.filter(group_projects::owner_id.eq(group_id))
                                         .filter(group_projects::project_name.eq(name));
        let group_project = query.first(conn)?;

        Ok(group_project)
    }

    pub fn update_group_project(project: &UpdateGroupProject,
                                conn: &PgConnection)
                                -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(group_projects::table.find(project.id)).set(project)
                                                              .execute(conn)
    }

    pub fn cancel_group_project(group_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(
            group_projects::table
                .filter(group_projects::owner_id.eq(group_id))
                .filter(group_projects::project_state.eq("NotStarted")),
        )
        .set(group_projects::project_state.eq("Canceled"))
        .execute(conn)
    }
}

#[allow(clippy::from_over_into)]
impl Into<jobsrv::JobGroupProject> for GroupProject {
    fn into(self) -> jobsrv::JobGroupProject {
        let mut project = jobsrv::JobGroupProject::new();

        project.set_name(self.project_name);
        project.set_ident(self.project_ident);
        let project_state = self.project_state
                                .parse::<jobsrv::JobGroupProjectState>()
                                .unwrap();
        project.set_state(project_state);
        project.set_target(self.target);
        project.set_job_id(self.job_id as u64);

        project
    }
}

////////////////////////////////////////////////////
//

// This needs to be kept in sync with the enum in the database
#[derive(DbEnum,
         Clone,
         Copy,
         PartialEq,
         Eq,
         Debug,
         Hash,
         Serialize,
         Deserialize,
         ToSql,
         FromSql)]
#[PgType = "job_exec_state"]
#[postgres(name = "job_exec_state")]
pub enum JobExecState {
    #[postgres(name = "pending")]
    Pending,
    #[postgres(name = "waiting_on_dependency")]
    WaitingOnDependency,
    #[postgres(name = "ready")]
    Ready,
    #[postgres(name = "running")]
    Running,
    #[postgres(name = "complete")]
    Complete,
    #[postgres(name = "job_failed")]
    JobFailed,
    #[postgres(name = "dependency_failed")]
    DependencyFailed,
    #[postgres(name = "cancel_pending")]
    CancelPending,
    #[postgres(name = "cancel_complete")]
    CancelComplete,
}

impl fmt::Display for JobExecState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match *self {
            JobExecState::Pending => "pending",
            JobExecState::WaitingOnDependency => "waiting_on_dependency",
            JobExecState::Ready => "ready",
            JobExecState::Running => "running",
            JobExecState::Complete => "complete",
            JobExecState::JobFailed => "job_failed",
            JobExecState::DependencyFailed => "dependency_failed",
            JobExecState::CancelPending => "cancel_pending",
            JobExecState::CancelComplete => "cancel_complete",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for JobExecState {
    type Err = BuilderError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            "pending" => Ok(JobExecState::Pending),
            "waiting_on_dependency" => Ok(JobExecState::WaitingOnDependency),
            "ready" => Ok(JobExecState::Ready),
            "running" => Ok(JobExecState::Running),
            "complete" => Ok(JobExecState::Complete),
            "job_failed" => Ok(JobExecState::JobFailed),
            "dependency_failed" => Ok(JobExecState::DependencyFailed),
            "cancel_pending" => Ok(JobExecState::CancelPending),
            "cancel_complete" => Ok(JobExecState::CancelComplete),
            _ => {
                Err(BuilderError::JobExecStateConversionError(format!(
                    "Could not convert {} to a JobExecState",
                    value
                )))
            }
        }
    }
}

#[derive(Insertable)]
#[table_name = "job_graph"]
pub struct NewJobGraphEntry<'a> {
    pub group_id:         i64,
    pub project_name:     &'a str, // ideally this would be an id in projects table
    pub job_id:           Option<i64>,
    pub job_state:        JobExecState,    // Should be enum
    pub manifest_ident:   &'a str,         //
    pub as_built_ident:   Option<&'a str>, //
    pub dependencies:     &'a [i64],
    pub waiting_on_count: i32,
    pub target_platform:  BuilderPackageTarget, // PackageTarget?
}

impl<'a> NewJobGraphEntry<'a> {
    pub fn new(group_id: i64,
               project_name: &'a str,
               manifest_ident: &'a str,
               job_state: JobExecState,
               dependency_ids: &'a [i64],
               target_platform: BuilderPackageTarget)
               -> Self {
        NewJobGraphEntry { group_id,
                           project_name,
                           job_id: None,
                           job_state,
                           manifest_ident,
                           as_built_ident: None,
                           dependencies: dependency_ids,
                           waiting_on_count: dependency_ids.len() as i32,
                           target_platform }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "job_graph"]
pub struct JobGraphEntry {
    pub id:               i64,
    pub group_id:         i64, /* This is the id of the associated group (should have been
                                * group_id) */
    pub project_name:     String, // ideally this would be an id in projects table
    pub job_id:           Option<i64>,
    pub job_state:        JobExecState, // Should be enum
    pub manifest_ident:   String,       //
    pub as_built_ident:   Option<BuilderPackageIdent>, // TODO revisit if needed
    pub dependencies:     Vec<i64>,
    pub waiting_on_count: i32,
    pub target_platform:  BuilderPackageTarget, // PackageTarget?
    pub created_at:       DateTime<Utc>,
    pub updated_at:       DateTime<Utc>,
}

#[derive(AsChangeset)]
#[table_name = "job_graph"]
pub struct UpdateJobGraphEntry<'a> {
    pub id:             i64,
    pub job_state:      JobExecState, // Should be enum
    pub job_id:         i64,
    pub as_built_ident: Option<&'a str>, //
}

#[derive(Default, Debug, Clone, PartialEq)]
// Names are kept brief here , but we should revisit this
pub struct JobStateCounts {
    pub pd: i64, // Pending
    pub wd: i64, // WaitingOnDependency
    pub rd: i64, // Ready
    pub rn: i64, // Running
    pub ct: i64, // Complete
    pub jf: i64, // JobFailed
    pub df: i64, // DependencyFailed
    pub cp: i64, // CancelPending
    pub cc: i64, // CancelComplete
}

impl JobGraphEntry {
    pub fn create(req: &NewJobGraphEntry, conn: &PgConnection) -> QueryResult<JobGraphEntry> {
        Counter::DBCall.increment();
        let start_time = Instant::now();
        let query = diesel::insert_into(job_graph::table).values(req);

        // let debug = diesel::query_builder::debug_query::<diesel::pg::Pg, _>(&query);
        // let out = format!("{:?}", debug);

        let result = query.get_result(conn);

        let duration_millis = (start_time.elapsed().as_micros() as f64) / 1_000.0;
        trace!("DBCall JobGraphEntry::create time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis);
        Histogram::JobGraphEntryCreateCallTime.set(duration_millis);
        result
    }

    pub fn create_batch(req: &[NewJobGraphEntry],
                        conn: &PgConnection)
                        -> QueryResult<JobGraphEntry> {
        Counter::DBCall.increment();
        let start_time = std::time::Instant::now();

        let query = diesel::insert_into(job_graph::table).values(req);
        // let debug = diesel::query_builder::debug_query::<diesel::pg::Pg, _>(&query);

        let result = query.get_result(conn);

        let duration_millis = (start_time.elapsed().as_micros() as f64) / 1_000.0;
        trace!("DBCall JobGraphEntry::create_batch time: {} ms",
               duration_millis);
        Histogram::JobGraphEntryCreateBatchCallTime.set(duration_millis);
        Histogram::DbCallTime.set(duration_millis);
        result
    }

    pub fn get(id: i64, conn: &PgConnection) -> QueryResult<JobGraphEntry> {
        metrics_instrument_block!(JobGraphEntry, get, {
            job_graph::table.filter(job_graph::id.eq(id))
                            .get_result(conn)
        })
    }

    pub fn get_by_job_id(job_id: i64, conn: &PgConnection) -> QueryResult<JobGraphEntry> {
        Counter::DBCall.increment();
        job_graph::table.filter(job_graph::job_id.eq(job_id))
                        .first(conn)
    }

    pub fn list_group(group_id: i64, conn: &PgConnection) -> QueryResult<Vec<JobGraphEntry>> {
        Counter::DBCall.increment();
        job_graph::table.filter(job_graph::group_id.eq(group_id))
                        .get_results(conn)
    }

    pub fn list_group_by_state(group_id: i64,
                               state: JobExecState,
                               conn: &PgConnection)
                               -> QueryResult<Vec<JobGraphEntry>> {
        Counter::DBCall.increment();
        job_graph::table.filter(job_graph::group_id.eq(group_id))
                        .filter(job_graph::job_state.eq(state))
                        .get_results(conn)
    }

    pub fn count_by_state(group_id: i64,
                          job_state: JobExecState,
                          conn: &PgConnection)
                          -> QueryResult<i64> {
        Counter::DBCall.increment();

        job_graph::table.select(count_star())
                        .filter(job_graph::group_id.eq(group_id))
                        .filter(job_graph::job_state.eq(job_state))
                        .first(conn)
    }

    pub fn count_all_states(gid: i64,
                            conn: &diesel::pg::PgConnection)
                            -> QueryResult<JobStateCounts> {
        Counter::DBCall.increment();
        let start_time = std::time::Instant::now();

        let j =
            JobStateCounts { pd: JobGraphEntry::count_by_state(gid, JobExecState::Pending, conn)?,
                             wd: JobGraphEntry::count_by_state(gid,
                                                               JobExecState::WaitingOnDependency,
                                                               conn)?,
                             rd: JobGraphEntry::count_by_state(gid, JobExecState::Ready, conn)?,
                             rn: JobGraphEntry::count_by_state(gid, JobExecState::Running, conn)?,
                             ct: JobGraphEntry::count_by_state(gid, JobExecState::Complete, conn)?,
                             jf: JobGraphEntry::count_by_state(gid, JobExecState::JobFailed, conn)?,
                             df: JobGraphEntry::count_by_state(gid,
                                                               JobExecState::DependencyFailed,
                                                               conn)?,
                             cp: JobGraphEntry::count_by_state(gid,
                                                               JobExecState::CancelPending,
                                                               conn)?,
                             cc: JobGraphEntry::count_by_state(gid,
                                                               JobExecState::CancelComplete,
                                                               conn)?, };
        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall JobGraphEntry::count_all_states time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Ok(j)
    }

    // Do we want this for other states?
    // This will require an index most likely or create a linear search
    pub fn count_ready_for_target(target: BuilderPackageTarget,
                                  conn: &PgConnection)
                                  -> QueryResult<i64> {
        job_graph::table.select(count_star())
                        .filter(job_graph::target_platform.eq(target.to_string()))
                        .filter(job_graph::job_state.eq(JobExecState::Ready))
                        .first(conn)
    }

    pub fn bulk_update_state(group_id: i64,
                             required_job_state: JobExecState,
                             new_job_state: JobExecState,
                             conn: &PgConnection)
                             -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(
            job_graph::table
                .filter(job_graph::job_state.eq(required_job_state))
                .filter(job_graph::group_id.eq(group_id)),
        )
        .set(job_graph::job_state.eq(new_job_state))
        .execute(conn)
    }

    pub fn set_job_id(id: i64, job_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(job_graph::table.find(id)).set(job_graph::job_id.eq(job_id))
                                                 .execute(conn)
    }

    // Consider making this a stored procedure or a transaction.
    pub fn take_next_job_for_target(target: BuilderPackageTarget,
                                    conn: &PgConnection)
                                    -> QueryResult<Option<JobGraphEntry>> {
        Counter::DBCall.increment();
        // TODO make this a transaction
        // Logically this is going to be a select over the target for job_graph entries that are in
        // state Ready sorted by some sort of priority.
        // conn.transaction::(<_, Error, _>)|| {
        let next_job: QueryResult<JobGraphEntry> =
            job_graph::table
            .filter(job_graph::target_platform.eq(target.to_string()))
            .filter(job_graph::job_state.eq(JobExecState::Ready))
            // This is the effective priority of a job; right now we select the oldest entry, but
            // in the future we may want to prioritize finishing one group before starting the next, or by
            // some precomputed metric (e.g. total number of transitive deps or some other parallelisim maximizing
            // heuristic
            .order((job_graph::group_id, job_graph::created_at.asc(), job_graph::id))
            .limit(1)
            .get_result(conn);
        match next_job {
            Ok(job) => {
                // This should be done in a transaction
                diesel::update(job_graph::table.find(job.id))
                    .set(job_graph::job_state.eq(JobExecState::Running))
                    .execute(conn)?;
                Ok(Some(job))
            }
            diesel::QueryResult::Err(diesel::result::Error::NotFound) => Ok(None),
            diesel::QueryResult::Err(x) => diesel::QueryResult::Err(x), /* this turns QueryResult<X> into QueryResult<Option<X>> */
        }
    }

    pub fn transitive_rdeps_for_id(id: i64, conn: &PgConnection) -> QueryResult<Vec<i64>> {
        Counter::DBCall.increment();
        diesel::select(job_functions::t_rdeps_for_id(id)).get_results(conn)
    }

    pub fn transitive_deps_for_id(id: i64, conn: &PgConnection) -> QueryResult<Vec<i64>> {
        Counter::DBCall.increment();
        diesel::select(job_functions::t_deps_for_id(id)).get_results(conn)
    }

    pub fn transitive_deps_for_id_and_group(id: i64,
                                            group_id: i64,
                                            conn: &PgConnection)
                                            -> QueryResult<Vec<i64>> {
        Counter::DBCall.increment();
        let result = diesel::select(job_functions::t_deps_for_id_group(id, group_id))
            .get_results::<i64>(conn)?;
        Ok(result)
    }

    pub fn mark_job_complete(id: i64,
                             as_built: &BuilderPackageIdent,
                             conn: &PgConnection)
                             -> QueryResult<i32> {
        Counter::DBCall.increment();
        diesel::select(job_functions::job_graph_mark_complete(
            id,
            &as_built.to_string(),
        ))
        .get_result::<i32>(conn)
    }

    pub fn mark_job_failed(id: i64, conn: &PgConnection) -> QueryResult<i32> {
        Counter::DBCall.increment();
        diesel::select(job_functions::job_graph_mark_failed(id)).get_result::<i32>(conn)
    }

    // Updates jobs when group is dispatched; jobs are moved from Pending to WaitingOnDependency, or
    // if there are zero dependencies, moved to Ready
    // Need an index to make this reasonably fast
    // Returns number of ready jobs
    pub fn group_dispatched_update_jobs(group_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        JobGraphEntry::bulk_update_state(group_id,
                                         JobExecState::Pending,
                                         JobExecState::WaitingOnDependency,
                                         conn)?;
        // See job_graph_mark_complete for another instance of this query pattern
        // perhaps it should be abstracted out
        diesel::update(
            job_graph::table
                .filter(job_graph::group_id.eq(group_id))
                .filter(job_graph::job_state.eq(JobExecState::WaitingOnDependency))
                .filter(job_graph::waiting_on_count.eq(0)),
        )
        .set(job_graph::job_state.eq(JobExecState::Ready))
        .execute(conn)
    }
}

#[allow(clippy::from_over_into)]
impl Into<jobsrv::JobGroupProject> for JobGraphEntry {
    fn into(self) -> jobsrv::JobGroupProject {
        let project_state = match self.job_state {
            JobExecState::Pending | JobExecState::WaitingOnDependency | JobExecState::Ready => {
                jobsrv::JobGroupProjectState::NotStarted
            }

            JobExecState::Running => jobsrv::JobGroupProjectState::InProgress,
            JobExecState::Complete => jobsrv::JobGroupProjectState::Success,
            JobExecState::JobFailed => jobsrv::JobGroupProjectState::Failure,
            JobExecState::DependencyFailed => jobsrv::JobGroupProjectState::Skipped,
            JobExecState::CancelPending | JobExecState::CancelComplete => {
                jobsrv::JobGroupProjectState::Canceled
            }
        };

        let mut project = jobsrv::JobGroupProject::new();

        project.set_name(self.project_name);
        let as_built_ident = if let Some(ident) = self.as_built_ident {
            ident.to_string()
        } else {
            "Not yet built".to_string()
        };
        project.set_ident(as_built_ident);
        project.set_state(project_state);
        project.set_target(self.target_platform.to_string());
        if let Some(id) = self.job_id {
            project.set_job_id(id as u64)
        };
        project
    }
}
