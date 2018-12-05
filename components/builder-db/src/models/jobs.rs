use super::db_id_format;
use chrono::prelude::*;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use protobuf::ProtobufEnum;

use protocol::jobsrv;
use protocol::net;
use protocol::originsrv;

use models::pagination::Paginate;
use schema::jobs::jobs;

use bldr_core::metrics::CounterMetric;
use metrics::Counter;

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
    pub vcs_arguments: Vec<String>,
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
            .paginate(lpj.page)
            .per_page(lpj.limit)
            .load_and_count_records(conn)
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

        // Note: these may be null (e.g., a job is scheduled, but hasn't
        // started; a job has started and is currently running)
        if let Some(start) = self.build_started_at {
            job.set_build_started_at(start.to_rfc3339());
        }
        if let Some(stop) = self.build_finished_at {
            job.set_build_finished_at(stop.to_rfc3339());
        }

        // package_ident will only be present if the build succeeded
        if let Some(ident_str) = self.package_ident {
            let ident: originsrv::OriginPackageIdent = ident_str.parse().unwrap();
            job.set_package_ident(ident);
        }

        let mut project = originsrv::OriginProject::new();
        project.set_id(self.project_id as u64);

        // only 'project_name' exists in the jobs table, but it's just
        // "origin/name", so we can set those fields in the Project
        // struct.
        //
        // 'package_ident' may be null, though, so we shouldn't use it to
        // get the origin and name.
        let name = self.project_name.clone();
        let name_for_split = self.project_name.clone();
        let name_split: Vec<&str> = name_for_split.split("/").collect();
        project.set_origin_name(name_split[0].to_string());
        project.set_package_name(name_split[1].to_string());
        project.set_name(name);

        project.set_owner_id(self.project_owner_id as u64);
        project.set_plan_path(self.project_plan_path.clone());

        match self.vcs.as_ref() {
            "git" => {
                let mut vcsa: Vec<String> = self.vcs_arguments;
                project.set_vcs_type(String::from("git"));
                project.set_vcs_data(vcsa.remove(0));
                if vcsa.len() > 0 {
                    let install_id = vcsa.remove(0);
                    project.set_vcs_installation_id(install_id.parse::<u32>().unwrap());
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

        job
    }
}
