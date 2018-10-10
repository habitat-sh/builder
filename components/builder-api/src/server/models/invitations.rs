use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::BigInt;
use diesel::RunQueryDsl;
use server::schema::invitation::*;

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_invitations"]
pub struct OriginInvitation {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub origin_id: i64,
    pub origin_name: String,
    #[serde(with = "db_id_format")]
    pub account_id: i64,
    pub account_name: String,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub ignored: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl OriginInvitation {
    pub fn list_by_account(
        owner_id: i64,
        conn: &PgConnection,
    ) -> QueryResult<Vec<OriginInvitation>> {
        diesel::sql_query("select * from get_origin_invitations_for_account_v1($1)")
            .bind::<BigInt, _>(owner_id)
            .get_results(conn)
    }
}
