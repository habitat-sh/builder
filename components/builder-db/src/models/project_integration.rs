use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::Text;
use diesel::RunQueryDsl;
use protocol::originsrv;
use schema::project_integration::*;

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_project_integrations"]
pub struct ProjectIntegration {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub project_id: i64,
    #[serde(with = "db_id_format")]
    pub integration_id: i64,
    pub origin: String,
    pub body: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

// #[derive(Insertable)]
// #[table_name = "origin_project_integrations"]
// TODO : Make this directly insertable?
pub struct NewProjectIntegration<'a> {
    pub origin: &'a str,
    pub name: &'a str,
    pub integration: &'a str,
    pub body: &'a str,
}

impl ProjectIntegration {
    pub fn get(
        origin: &str,
        name: &str,
        integration: &str,
        conn: &PgConnection,
    ) -> QueryResult<ProjectIntegration> {
        diesel::sql_query("select * from get_origin_project_integrations_v2($1, $2, $3)")
            .bind::<Text, _>(origin)
            .bind::<Text, _>(name)
            .bind::<Text, _>(integration)
            .get_result(conn)
    }

    pub fn list(
        origin: &str,
        name: &str,
        conn: &PgConnection,
    ) -> QueryResult<Vec<ProjectIntegration>> {
        diesel::sql_query("select * from get_origin_project_integrations_for_project_v2($1, $2)")
            .bind::<Text, _>(origin)
            .bind::<Text, _>(name)
            .get_results(conn)
    }

    pub fn delete(
        origin: &str,
        name: &str,
        integration: &str,
        conn: &PgConnection,
    ) -> QueryResult<usize> {
        diesel::sql_query("select * from delete_origin_project_integration_v1($1, $2, $3)")
            .bind::<Text, _>(origin)
            .bind::<Text, _>(name)
            .bind::<Text, _>(integration)
            .execute(conn)
    }

    pub fn create(req: NewProjectIntegration, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("SELECT * FROM upsert_origin_project_integration_v3($1, $2, $3, $4)")
            .bind::<Text, _>(req.origin)
            .bind::<Text, _>(req.name)
            .bind::<Text, _>(req.integration)
            .bind::<Text, _>(req.body)
            .execute(conn)
    }
}

impl Into<originsrv::OriginProjectIntegration> for ProjectIntegration {
    fn into(self) -> originsrv::OriginProjectIntegration {
        let mut opi = originsrv::OriginProjectIntegration::new();
        opi.set_origin(self.origin);
        opi.set_body(self.body);
        opi
    }
}
