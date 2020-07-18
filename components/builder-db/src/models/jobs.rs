use super::db_id_format;
use chrono::prelude::*;
use diesel::{dsl::count_star,
             pg::PgConnection,
             result::QueryResult,
             BoolExpressionMethods,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};

use diesel::{deserialize::{self,
                           FromSql,
                           Queryable},
             pg::Pg};

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

use crate::{models::pagination::Paginate,
            schema::jobs::{busy_workers,
                           group_projects,
                           groups,
                           jobs}};

use crate::functions::jobs as job_functions;

use crate::{bldr_core::metrics::CounterMetric,
            hab_core::package::PackageTarget,
            metrics::Counter};

#[derive(Clone, Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "jobs"]
#[diesel(deserialize_as = "Job")]
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

pub struct ListProjectJobs {
    pub name:  String,
    pub page:  i64,
    pub limit: i64,
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

impl Job {
    pub fn get(id: i64, conn: &PgConnection) -> QueryResult<Job> {
        Counter::DBCall.increment();
        jobs::table.filter(jobs::id.eq(id)).get_result(conn)
    }

    pub fn get_next_pending(worker: &str, target: &str, conn: &PgConnection) -> QueryResult<Job> {
        Counter::DBCall.increment();
        let result = jobs::table.select(job_functions::next_pending_job_v2(worker, target))
                                .first::<Vec<Job>>(conn)?; // should this be get_result?
        result.first()
              .ok_or(diesel::result::Error::NotFound)
              .map(|x| (*x).clone())
    }

    // job_state should be an enum, at least on the rust side (see  jobsrv::JobState)
    // Maybe this should filter by target.... (CanceledPending is ok, but Dispatched might not make
    // sense)
    pub fn get_job_by_state(job_state: &str, conn: &PgConnection) -> QueryResult<Vec<Job>> {
        Counter::DBCall.increment();
        jobs::table.filter(jobs::job_state.eq(job_state)).load(conn)
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

    pub fn update(job: &UpdateJob, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(jobs::table.find(job.id)).set(job)
                                                .execute(conn)
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
}

impl FromSql<JobRecord, diesel::pg::Pg> for Job {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let tuple = <_ as FromSql<JobRecord, Pg>>::from_sql(bytes)?;
        Ok(<Self as Queryable<JobFields, Pg>>::build(tuple))
    }
}

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

#[derive(Clone, Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "groups"]
pub struct Group {
    #[serde(with = "db_id_format")]
    pub id:           i64,
    pub group_state:  String,
    pub project_name: String,
    pub target:       String,
    pub created_at:   Option<DateTime<Utc>>,
    pub updated_at:   Option<DateTime<Utc>>,
}

// The ordering here must match the above, or strange errors occur.
pub type GroupFields = (BigInt, Text, Text, Text, Nullable<Timestamptz>, Nullable<Timestamptz>);

pub type GroupRecord = Record<GroupFields>;

impl Group {
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

    pub fn pending_job_groups(count: i32, conn: &PgConnection) -> QueryResult<Vec<Group>> {
        Counter::DBCall.increment();
        let result = groups::table.select(job_functions::pending_groups_v1(count))
                                  .get_result::<Vec<Group>>(conn)?;
        Ok(result)
    }

    pub fn insert_group(root_project: &str,
                        target: &str, // We want PackageTarget, but we can't have nice things
                        project_tuples: &Vec<(String, String)>,
                        conn: &PgConnection)
                        -> QueryResult<Group> {
        Counter::DBCall.increment();

        let (project_names, project_idents): (Vec<String>, Vec<String>) =
            project_tuples.iter().cloned().unzip();

        let result = groups::table.select(job_functions::insert_group_v3(root_project,
                                                                         project_names,
                                                                         project_idents,
                                                                         target.to_string()))
                                  .first::<Vec<Group>>(conn)?; // should this be get_result?
        result.first()
              .ok_or(diesel::result::Error::NotFound)
              .map(|x| (*x).clone())
    }

    pub fn cancel_job_group(group_id: i64, conn: &PgConnection) -> QueryResult<()> {
        Counter::DBCall.increment();
        groups::table.select(job_functions::cancel_group_v1(group_id))
                     .execute(conn)?;
        Ok(())
    }

    pub fn get_job_group(group_id: i64, conn: &PgConnection) -> QueryResult<Group> {
        Counter::DBCall.increment();

        groups::table.filter(groups::id.eq(group_id)).first(conn)
    }
}

impl FromSql<GroupRecord, diesel::pg::Pg> for Group {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let tuple = <_ as FromSql<GroupRecord, Pg>>::from_sql(bytes)?;
        Ok(<Self as Queryable<GroupFields, Pg>>::build(tuple))
    }
}

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

pub struct NewBusyWorker<'a> {
    pub target:      &'a str,
    pub ident:       &'a str,
    pub job_id:      i64,
    pub quarantined: bool,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "busy_workers"]
pub struct BusyWorker {
    pub target:      String,
    pub ident:       String,
    pub job_id:      i64,
    pub quarantined: bool,
    pub created_at:  Option<DateTime<Utc>>,
    pub updated_at:  Option<DateTime<Utc>>,
}

impl BusyWorker {
    pub fn list(conn: &PgConnection) -> QueryResult<Vec<BusyWorker>> {
        Counter::DBCall.increment();
        busy_workers::table.get_results(conn)
    }

    pub fn create(req: &NewBusyWorker, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::insert_into(busy_workers::table)
            .values((
                busy_workers::target.eq(req.target),
                busy_workers::ident.eq(req.ident),
                busy_workers::job_id.eq(req.job_id),
                busy_workers::quarantined.eq(req.quarantined),
            ))
            .on_conflict((busy_workers::ident, busy_workers::job_id))
            .do_update()
            .set(busy_workers::quarantined.eq(req.quarantined))
            .execute(conn)
    }

    pub fn delete(ident: &str, job_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(busy_workers::table.filter(busy_workers::ident.eq(ident))
                                          .filter(busy_workers::job_id.eq(job_id))).execute(conn)
    }
}

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "group_projects"]
pub struct GroupProject {
    pub id:            i64,
    pub owner_id:      i64,
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
    pub updated_at:    Option<DateTime<Utc>>,
}

impl GroupProject {
    pub fn get_group_projects(group_id: i64,
                              conn: &PgConnection)
                              -> QueryResult<Vec<GroupProject>> {
        Counter::DBCall.increment();
        group_projects::table.filter(group_projects::owner_id.eq(group_id))
                             .get_results(conn)
    }

    // The only consumer of this only wants the id, maybe simplify
    //
    pub fn get_group_project_by_name(group_id: i64,
                                     name: &str,
                                     conn: &PgConnection)
                                     -> QueryResult<GroupProject> {
        Counter::DBCall.increment();
        group_projects::table.filter(group_projects::owner_id.eq(group_id))
                             .filter(group_projects::project_name.eq(name))
                             // We would like to assume that group_id/name form
                             // a unique id. However the group_projects table lacks a constraint
                             // to that effect, so we're just trusting things here.
                             // TODO ADD A CONSTRAINT
                             .first(conn)
    }

    // Diesel interprets a 'None' in an option field as not needing update
    // http://diesel.rs/guides/all-about-updates/#aschangeset
    // TODO Figure out timestamp now for updated_at (not implemented)
    // https://github.com/diesel-rs/diesel/issues/91
    pub fn update_group_project(project: &UpdateGroupProject,
                                conn: &PgConnection)
                                -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(group_projects::table.find(project.id)).set(project)
                                                              .execute(conn)
    }
}
impl Into<jobsrv::JobGroupProject> for GroupProject {
    fn into(self) -> jobsrv::JobGroupProject {
        let mut project = jobsrv::JobGroupProject::new();

        // TODO: Should this return a result?
        let project_state = self.project_state
                                .parse::<jobsrv::JobGroupProjectState>()
                                .unwrap();

        project.set_name(self.project_name);
        project.set_ident(self.project_ident);
        project.set_state(project_state);
        project.set_target(self.target);
        project.set_job_id(self.job_id as u64);

        project
    }
}
