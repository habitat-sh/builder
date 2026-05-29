use super::{db_id_format,
            db_optional_id_format};
use chrono::NaiveDateTime;
use diesel::{self,
             dsl::count,
             pg::PgConnection,
             result::QueryResult,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};

use crate::schema::project::origin_projects;

use crate::{bldr_core::metrics::CounterMetric,
            metrics::Counter};

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = origin_projects)]
pub struct Project {
    #[serde(with = "db_id_format")]
    pub id:                  i64,
    pub origin:              String,
    #[serde(with = "db_id_format")]
    pub owner_id:            i64,
    pub package_name:        String,
    pub name:                String,
    pub plan_path:           String,
    pub target:              String,
    pub vcs_type:            String,
    pub vcs_data:            String,
    #[serde(with = "db_optional_id_format")]
    pub vcs_installation_id: Option<i64>,
    pub auto_build:          bool,
    pub created_at:          Option<NaiveDateTime>,
    pub updated_at:          Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = origin_projects)]
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
#[diesel(table_name = origin_projects)]
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
    pub fn get(name: &str, target: &str, conn: &mut PgConnection) -> QueryResult<Project> {
        Counter::DBCall.increment();
        origin_projects::table.filter(origin_projects::name.eq(name))
                              .filter(origin_projects::target.eq(target))
                              .get_result(conn)
    }

    pub fn delete(name: &str, target: &str, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_projects::table
                .filter(origin_projects::name.eq(name))
                .filter(origin_projects::target.eq(target)),
        )
        .execute(conn)
    }

    pub fn create(project: &NewProject, conn: &mut PgConnection) -> QueryResult<Project> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_projects::table).values(project)
                                                   .get_result(conn)
    }

    pub fn update(project: &UpdateProject, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_projects::table.find(project.id)).set(project)
                                                               .execute(conn)
    }

    pub fn list(origin: &str, conn: &mut PgConnection) -> QueryResult<Vec<Project>> {
        Counter::DBCall.increment();
        origin_projects::table.filter(origin_projects::origin.eq(origin))
                              .get_results(conn)
    }

    pub fn count_origin_projects(origin: &str, conn: &mut PgConnection) -> QueryResult<i64> {
        Counter::DBCall.increment();
        origin_projects::table.select(count(origin_projects::id))
                              .filter(origin_projects::origin.eq(&origin))
                              .first(conn)
    }

    pub fn get_by_id(project_id: i64, conn: &mut PgConnection) -> QueryResult<Project> {
        Counter::DBCall.increment();
        origin_projects::table.find(project_id).get_result(conn)
    }
}
