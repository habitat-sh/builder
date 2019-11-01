use super::{db_id_format,
            db_optional_id_format};
use chrono::NaiveDateTime;
use diesel::{self,
             pg::PgConnection,
             result::QueryResult,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};

use crate::{protocol::originsrv,
            schema::project::origin_projects};

use crate::{bldr_core::metrics::CounterMetric,
            metrics::Counter};

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "origin_projects"]
pub struct Project {
    #[serde(with = "db_id_format")]
    pub id: i64,
    pub origin: String,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub package_name: String,
    pub name: String,
    pub plan_path: String,
    pub target: String,
    pub vcs_type: String,
    pub vcs_data: String,
    #[serde(with = "db_optional_id_format")]
    pub vcs_installation_id: Option<i64>,
    pub auto_build: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "origin_projects"]
pub struct NewProject<'a> {
    pub owner_id:            i64,
    pub origin:              &'a str,
    pub name:                &'a str,
    pub package_name:        &'a str,
    pub plan_path:           &'a str,
    pub target:              &'a str,
    pub vcs_type:            &'a str,
    pub vcs_data:            &'a str,
    pub vcs_installation_id: Option<i64>,
    pub auto_build:          bool,
}

#[derive(AsChangeset)]
#[table_name = "origin_projects"]
pub struct UpdateProject<'a> {
    pub id:                  i64,
    pub owner_id:            i64,
    pub origin:              &'a str,
    pub package_name:        &'a str,
    pub plan_path:           &'a str,
    pub target:              &'a str,
    pub vcs_type:            &'a str,
    pub vcs_data:            &'a str,
    pub vcs_installation_id: Option<i64>,
    pub auto_build:          bool,
}

impl Project {
    pub fn get(name: &str, target: &str, conn: &PgConnection) -> QueryResult<Project> {
        Counter::DBCall.increment();
        origin_projects::table.filter(origin_projects::name.eq(name))
                              .filter(origin_projects::target.eq(target))
                              .get_result(conn)
    }

    pub fn delete(name: &str, target: &str, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(origin_projects::table.filter(origin_projects::name.eq(name)).filter(origin_projects::target.eq(target))).execute(conn)
    }

    pub fn create(project: &NewProject, conn: &PgConnection) -> QueryResult<Project> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_projects::table).values(project)
                                                   .get_result(conn)
    }

    pub fn update(project: &UpdateProject, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_projects::table.find(project.id)).set(project)
                                                               .execute(conn)
    }

    pub fn list(origin: &str, conn: &PgConnection) -> QueryResult<Vec<Project>> {
        Counter::DBCall.increment();
        origin_projects::table.filter(origin_projects::origin.eq(origin))
                              .get_results(conn)
    }
}

impl Into<originsrv::OriginProject> for Project {
    fn into(self) -> originsrv::OriginProject {
        let mut proj = originsrv::OriginProject::new();
        proj.set_id(self.id as u64);
        proj.set_owner_id(self.owner_id as u64);
        proj.set_origin_name(self.origin);
        proj.set_package_name(self.package_name);
        proj.set_name(self.name);
        proj.set_plan_path(self.plan_path);
        proj.set_target(self.target);
        proj.set_vcs_type(self.vcs_type);
        proj.set_vcs_data(self.vcs_data);
        if let Some(install_id) = self.vcs_installation_id {
            proj.set_vcs_installation_id(install_id as u32);
        }
        proj.set_auto_build(self.auto_build);
        proj
    }
}
