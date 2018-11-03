use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Bool, Text};
use diesel::RunQueryDsl;
use models::package::{PackageVisibility, PackageVisibilityMapping};
use protocol::originsrv;
use schema::project::*;

use bldr_core::metrics::CounterMetric;
use metrics::Counter;

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_projects"]
pub struct Project {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub origin_id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub origin_name: String,
    pub package_name: String,
    pub name: String,
    pub plan_path: String,
    pub visibility: PackageVisibility,
    pub vcs_type: String,
    pub vcs_data: String,
    #[serde(with = "db_id_format")]
    pub vcs_installation_id: i64,
    pub auto_build: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

pub struct NewProject<'a> {
    pub owner_id: i64,
    pub origin_name: &'a str,
    pub package_name: &'a str,
    pub plan_path: &'a str,
    pub vcs_type: &'a str,
    pub vcs_data: &'a str,
    pub install_id: i64,
    pub visibility: &'a PackageVisibility,
    pub auto_build: bool,
}

pub struct UpdateProject<'a> {
    pub id: i64,
    pub owner_id: i64,
    pub origin_id: i64,
    pub package_name: &'a str,
    pub plan_path: &'a str,
    pub vcs_type: &'a str,
    pub vcs_data: &'a str,
    pub install_id: i64,
    pub visibility: &'a PackageVisibility,
    pub auto_build: bool,
}

impl Project {
    pub fn get(name: &str, conn: &PgConnection) -> QueryResult<Project> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_project_v1($1)")
            .bind::<Text, _>(name)
            .get_result(conn)
    }

    pub fn delete(name: &str, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from delete_origin_project_v1($1)")
            .bind::<Text, _>(name)
            .execute(conn)
    }

    pub fn create(project: &NewProject, conn: &PgConnection) -> QueryResult<Project> {
        Counter::DBCall.increment();
        diesel::sql_query(
            "SELECT * FROM insert_origin_project_v6($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        ).bind::<Text, _>(project.origin_name)
        .bind::<Text, _>(project.package_name)
        .bind::<Text, _>(project.plan_path)
        .bind::<Text, _>(project.vcs_type)
        .bind::<Text, _>(project.vcs_data)
        .bind::<BigInt, _>(project.owner_id)
        .bind::<BigInt, _>(project.install_id)
        .bind::<PackageVisibilityMapping, _>(project.visibility)
        .bind::<Bool, _>(project.auto_build)
        .get_result(conn)
    }

    pub fn update(project: &UpdateProject, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::sql_query(
            "SELECT * FROM update_origin_project_v5($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        ).bind::<BigInt, _>(project.id)
        .bind::<BigInt, _>(project.origin_id)
        .bind::<Text, _>(project.package_name)
        .bind::<Text, _>(project.plan_path)
        .bind::<Text, _>(project.vcs_type)
        .bind::<Text, _>(project.vcs_data)
        .bind::<BigInt, _>(project.owner_id)
        .bind::<BigInt, _>(project.install_id)
        .bind::<PackageVisibilityMapping, _>(project.visibility)
        .bind::<Bool, _>(project.auto_build)
        .execute(conn)
    }

    pub fn list(origin: &str, conn: &PgConnection) -> QueryResult<Vec<Project>> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_project_list_v2($1)")
            .bind::<Text, _>(origin)
            .get_results(conn)
    }
}

impl Into<originsrv::OriginProject> for Project {
    fn into(self) -> originsrv::OriginProject {
        let mut proj = originsrv::OriginProject::new();
        proj.set_id(self.id as u64);
        proj.set_owner_id(self.owner_id as u64);
        proj.set_origin_id(self.origin_id as u64);
        proj.set_origin_name(self.origin_name);
        proj.set_package_name(self.package_name);
        proj.set_name(self.name);
        proj.set_plan_path(self.plan_path);
        proj.set_vcs_type(self.vcs_type);
        proj.set_vcs_data(self.vcs_data);
        proj.set_vcs_installation_id(self.vcs_installation_id as u32);
        proj.set_auto_build(self.auto_build);
        proj
    }
}
