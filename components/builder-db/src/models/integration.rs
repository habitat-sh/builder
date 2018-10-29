use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::Text;
use diesel::RunQueryDsl;
use schema::integration::*;

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_integrations"]
pub struct OriginIntegration {
    #[serde(with = "db_id_format")]
    pub id: i64,
    pub origin: String,
    pub integration: String,
    pub name: String,
    pub body: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "origin_integrations"]
pub struct NewOriginIntegration<'a> {
    pub origin: &'a str,
    pub integration: &'a str,
    pub name: &'a str,
    pub body: &'a str,
}

impl OriginIntegration {
    pub fn create(req: &NewOriginIntegration, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from upsert_origin_integration_v1($1, $2, $3, $4)")
            .bind::<Text, _>(req.origin)
            .bind::<Text, _>(req.integration)
            .bind::<Text, _>(req.name)
            .bind::<Text, _>(req.body)
            .execute(conn)
    }

    pub fn get(
        origin: &str,
        integration: &str,
        name: &str,
        conn: &PgConnection,
    ) -> QueryResult<OriginIntegration> {
        diesel::sql_query("select * from get_origin_integration_v1($1, $2, $3)")
            .bind::<Text, _>(origin)
            .bind::<Text, _>(integration)
            .bind::<Text, _>(name)
            .get_result(conn)
    }

    pub fn delete(
        origin: &str,
        integration: &str,
        name: &str,
        conn: &PgConnection,
    ) -> QueryResult<usize> {
        diesel::sql_query("select * from delete_origin_integration_v1($1, $2, $3)")
            .bind::<Text, _>(origin)
            .bind::<Text, _>(integration)
            .bind::<Text, _>(name)
            .execute(conn)
    }

    pub fn list_for_origin_integration(
        origin: &str,
        integration: &str,
        conn: &PgConnection,
    ) -> QueryResult<Vec<OriginIntegration>> {
        diesel::sql_query("select * from get_origin_integrations_v1($1, $2)")
            .bind::<Text, _>(origin)
            .bind::<Text, _>(integration)
            .get_results(conn)
    }

    pub fn list_for_origin(
        origin: &str,
        conn: &PgConnection,
    ) -> QueryResult<Vec<OriginIntegration>> {
        diesel::sql_query("select * from get_origin_integrations_for_origin_v1($1)")
            .bind::<Text, _>(origin)
            .get_results(conn)
    }
}
