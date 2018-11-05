use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Text};
use diesel::RunQueryDsl;
use schema::secrets::*;

use bldr_core::metrics::CounterMetric;
use metrics::Counter;

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_secrets"]
pub struct OriginSecret {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub origin_id: i64,
    pub owner_id: Option<i64>, // can be null
    pub name: String,
    pub value: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl OriginSecret {
    pub fn create(
        origin_id: i64,
        name: &str,
        value: &str,
        conn: &PgConnection,
    ) -> QueryResult<usize> {
        Counter::DBCall.increment();
        // TODO FIX: We're missing setting the account id here
        diesel::sql_query("select * from insert_origin_secret_v1($1, $2, $3)")
            .bind::<BigInt, _>(origin_id)
            .bind::<Text, _>(name)
            .bind::<Text, _>(value)
            .execute(conn)
    }

    pub fn get(origin_id: i64, name: &str, conn: &PgConnection) -> QueryResult<OriginSecret> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_secret_v1($1, $2)")
            .bind::<BigInt, _>(origin_id)
            .bind::<Text, _>(name)
            .get_result(conn)
    }

    pub fn delete(origin_id: i64, name: &str, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from delete_origin_secret_v1($1, $2)")
            .bind::<BigInt, _>(origin_id)
            .bind::<Text, _>(name)
            .execute(conn)
    }

    pub fn list(origin_id: i64, conn: &PgConnection) -> QueryResult<Vec<OriginSecret>> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_secrets_for_origin_v1($1)")
            .bind::<BigInt, _>(origin_id)
            .get_results(conn)
    }
}
