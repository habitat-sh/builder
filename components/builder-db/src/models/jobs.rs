use super::db_id_format;
use chrono::prelude::*;
use diesel::dsl::count_star;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use protobuf::ProtobufEnum;

use crate::protocol::jobsrv;
use crate::protocol::net;
use crate::protocol::originsrv;

use crate::models::pagination::Paginate;
use crate::schema::jobs::{busy_workers, groups, jobs};

use crate::bldr_core::metrics::CounterMetric;
use crate::hab_core::package::PackageTarget;
use crate::metrics::Counter;

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "jobs"]
pub struct Job {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub job_state: String,
    #[serde(with = "db_id_format")]
    pub project_id: i64,
    pub project_name: String,
    #[serde(with = "db_id_format")]
    pub project_owner_id: i64,
    pub project_plan_path: String,
    pub vcs: String,
    pub vcs_arguments: Vec<Option<String>>,
    pub net_error_code: Option<i32>,
    pub net_error_msg: Option<String>,
    pub scheduler_sync: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub build_started_at: Option<DateTime<Utc>>,
    pub build_finished_at: Option<DateTime<Utc>>,
    pub package_ident: Option<String>,
    pub archived: bool,
    pub channel: Option<String>,
    pub sync_count: i32,
    pub worker: Option<String>,
    pub target: String,
}

#[derive(Insertable)]
#[table_name = "jobs"]
pub struct NewJob<'a> {
    pub owner_id: i64,
    pub project_id: i64,
    pub project_name: &'a str,
    pub project_owner_id: i64,
    pub project_plan_path: &'a str,
    pub vcs: &'a str,
    pub vcs_arguments: Vec<&'a str>,
    pub channel: &'a str,
    pub target: &'a str,
}

pub struct ListProjectJobs {
    pub name: String,
    pub page: i64,
    pub limit: i64,
}

impl Job {
    pub fn get(id: i64, conn: &PgConnection) -> QueryResult<Job> {
        Counter::DBCall.increment();
        jobs::table.filter(jobs::id.eq(id)).get_result(conn)
    }

    pub fn list(lpj: ListProjectJobs, conn: &PgConnection) -> QueryResult<(Vec<Job>, i64)> {
        jobs::table
            .filter(jobs::project_name.eq(lpj.name))
            .order(jobs::created_at.desc())
            .paginate(lpj.page)
            .per_page(lpj.limit)
            .load_and_count_records(conn)
    }

    pub fn create(job: &NewJob, conn: &PgConnection) -> QueryResult<Job> {
        Counter::DBCall.increment();
        diesel::insert_into(jobs::table)
            .values(job)
            .get_result(conn)
    }

    pub fn count(
        job_state: jobsrv::JobState,
        target: PackageTarget,
        conn: &PgConnection,
    ) -> QueryResult<i64> {
        Counter::DBCall.increment();
        jobs::table
            .select(count_star())
            .filter(jobs::job_state.eq(job_state.to_string()))
            .filter(jobs::target.eq(target.to_string()))
            .first(conn)
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

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "groups"]
pub struct Group {
    #[serde(with = "db_id_format")]
    pub id: i64,
    pub group_state: String,
    pub project_name: String,
    pub target: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Group {
    pub fn get_queued(project_name: &str, target: &str, conn: &PgConnection) -> QueryResult<Group> {
        Counter::DBCall.increment();
        groups::table
            .filter(groups::project_name.eq(project_name))
            .filter(groups::group_state.eq("Queued"))
            .filter(groups::target.eq(target))
            .get_result(conn)
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
    pub target: &'a str,
    pub ident: &'a str,
    pub job_id: i64,
    pub quarantined: bool,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "busy_workers"]
pub struct BusyWorker {
    pub target: String,
    pub ident: String,
    pub job_id: i64,
    pub quarantined: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
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
        diesel::delete(
            busy_workers::table
                .filter(busy_workers::ident.eq(ident))
                .filter(busy_workers::job_id.eq(job_id)),
        )
        .execute(conn)
    }
}
