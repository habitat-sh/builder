use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Bool, Text};
use diesel::RunQueryDsl;
use schema::invitation::*;

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

#[derive(Insertable)]
#[table_name = "origin_invitations"]
pub struct NewOriginInvitation<'a> {
    pub origin_id: i64,
    pub origin_name: &'a str,
    pub account_id: i64,
    pub account_name: &'a str,
    pub owner_id: i64,
}

impl OriginInvitation {
    pub fn create(req: &NewOriginInvitation, conn: &PgConnection) -> QueryResult<OriginInvitation> {
        diesel::sql_query("select * from insert_origin_invitation_v1($1, $2, $3, $4, $5)")
            .bind::<BigInt, _>(req.origin_id)
            .bind::<Text, _>(req.origin_name)
            .bind::<BigInt, _>(req.account_id)
            .bind::<Text, _>(req.account_name)
            .bind::<BigInt, _>(req.owner_id)
            .get_result(conn)
    }

    pub fn list_by_origin(
        origin_id: u64,
        conn: &PgConnection,
    ) -> QueryResult<Vec<OriginInvitation>> {
        diesel::sql_query("select * from get_origin_invitations_for_origin_v1($1)")
            .bind::<BigInt, _>(origin_id as i64)
            .get_results(conn)
    }

    pub fn list_by_account(
        owner_id: u64,
        conn: &PgConnection,
    ) -> QueryResult<Vec<OriginInvitation>> {
        diesel::sql_query("select * from get_origin_invitations_for_account_v1($1)")
            .bind::<BigInt, _>(owner_id as i64)
            .get_results(conn)
    }

    pub fn accept(invite_id: u64, ignore: bool, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from accept_origin_invitation_v1($1, $2)")
            .bind::<BigInt, _>(invite_id as i64)
            .bind::<Bool, _>(ignore)
            .execute(conn)
    }

    pub fn ignore(invite_id: u64, account_id: u64, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from ignore_origin_invitation_v1($1, $2)")
            .bind::<BigInt, _>(invite_id as i64)
            .bind::<BigInt, _>(account_id as i64)
            .execute(conn)
    }

    pub fn rescind(invite_id: u64, account_id: u64, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from rescind_origin_invitation_v1($1, $2)")
            .bind::<BigInt, _>(invite_id as i64)
            .bind::<BigInt, _>(account_id as i64)
            .execute(conn)
    }
}
