use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::BigInt;
use diesel::RunQueryDsl;
use server::schema::origin::*;

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origins"]
pub struct Origin {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub default_package_visibility: String,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origins_with_stats"]
pub struct OriginWithStats {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub default_package_visibility: String,
    pub package_count: i64,
}

impl Origin {
    pub fn list(owner_id: i64, conn: &PgConnection) -> QueryResult<Vec<OriginWithStats>> {
        diesel::sql_query("select * from my_origins_with_stats_v1($1)")
            .bind::<BigInt, _>(owner_id)
            .get_results(conn)
    }
}
