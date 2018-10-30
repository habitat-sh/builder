use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Text};
use diesel::RunQueryDsl;
use schema::account::*;

#[derive(Identifiable, Debug, Serialize, QueryableByName)]
#[table_name = "accounts"]
pub struct Account {
    #[serde(with = "db_id_format")]
    pub id: i64,
    pub email: String,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Identifiable, Debug, Serialize, QueryableByName)]
#[table_name = "account_tokens"]
pub struct AccountToken {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub account_id: i64,
    pub token: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "accounts"]
pub struct NewAccount<'a> {
    pub email: &'a str,
    pub name: &'a str,
}

impl Account {
    pub fn get(name: &str, conn: &PgConnection) -> QueryResult<Account> {
        diesel::sql_query("SELECT * FROM get_account_by_name_v1($1)")
            .bind::<Text, _>(name)
            .get_result(conn)
    }

    pub fn get_by_id(id: u64, conn: &PgConnection) -> QueryResult<Account> {
        diesel::sql_query("SELECT * FROM get_account_by_id_v1($1)")
            .bind::<BigInt, _>(id as i64)
            .get_result(conn)
    }

    pub fn create(account: &NewAccount, conn: &PgConnection) -> QueryResult<Account> {
        diesel::sql_query("SELECT * FROM select_or_insert_account_v1($1, $2)")
            .bind::<Text, _>(account.name)
            .bind::<Text, _>(account.email)
            .get_result(conn)
    }

    pub fn find_or_create(name: &str, email: &str, conn: &PgConnection) -> QueryResult<Account> {
        diesel::sql_query("SELECT * FROM select_or_insert_account_v1($1, $2)")
            .bind::<Text, _>(name)
            .bind::<Text, _>(email)
            .get_result(conn)
    }

    pub fn update(id: u64, email: &str, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("SELECT update_account_v1($1, $2)")
            .bind::<BigInt, _>(id as i64)
            .bind::<Text, _>(email)
            .execute(conn)
    }
}

#[derive(Insertable)]
#[table_name = "account_tokens"]
pub struct NewAccountToken<'a> {
    pub account_id: i64,
    pub token: &'a str,
}

impl AccountToken {
    pub fn list(account_id: u64, conn: &PgConnection) -> QueryResult<Vec<AccountToken>> {
        diesel::sql_query("SELECT * FROM get_account_tokens_v1($1)")
            .bind::<BigInt, _>(account_id as i64)
            .get_results(conn)
    }

    pub fn create(req: &NewAccountToken, conn: &PgConnection) -> QueryResult<Account> {
        diesel::sql_query("SELECT * FROM insert_account_token_v1($1, $2)")
            .bind::<BigInt, _>(req.account_id)
            .bind::<Text, _>(req.token)
            .get_result(conn)
    }

    pub fn delete(id: u64, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("SELECT revoke_account_token_v1($1)")
            .bind::<BigInt, _>(id as i64)
            .execute(conn)
    }
}
