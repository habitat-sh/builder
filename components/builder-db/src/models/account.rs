use super::db_id_format;
use chrono::NaiveDateTime;
use diesel::{
    self, pg::PgConnection, result::QueryResult, ExpressionMethods, QueryDsl, RunQueryDsl,
};
use schema::account::{account_tokens, accounts};

use bldr_core::metrics::CounterMetric;
use metrics::Counter;

#[derive(Debug, Identifiable, Serialize, Queryable)]
pub struct Account {
    #[serde(with = "db_id_format")]
    pub id: i64,
    pub email: String,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Identifiable, Debug, Serialize, Queryable)]
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
        Counter::DBCall.increment();
        accounts::table
            .filter(accounts::name.eq(name))
            .get_result(conn)
    }

    pub fn get_by_id(id: i64, conn: &PgConnection) -> QueryResult<Account> {
        Counter::DBCall.increment();
        accounts::table.find(id).get_result(conn)
    }

    pub fn create(account: &NewAccount, conn: &PgConnection) -> QueryResult<Account> {
        Counter::DBCall.increment();
        diesel::insert_into(accounts::table)
            .values(account)
            .get_result(conn)
    }

    pub fn find_or_create(account: &NewAccount, conn: &PgConnection) -> QueryResult<Account> {
        Counter::DBCall.increment();
        match diesel::insert_into(accounts::table)
            .values(account)
            .on_conflict(accounts::name)
            .do_nothing()
            .get_result(conn)
        {
            Ok(account) => Ok(account),
            Err(_) => accounts::table
                .filter(accounts::name.eq(account.name))
                .get_result(conn),
        }
    }

    pub fn update(id: u64, email: &str, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(accounts::table.find(id as i64))
            .set(accounts::email.eq(email))
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
        Counter::DBCall.increment();
        account_tokens::table
            .filter(account_tokens::account_id.eq(account_id as i64))
            .get_results(conn)
    }

    pub fn create(req: &NewAccountToken, conn: &PgConnection) -> QueryResult<AccountToken> {
        Counter::DBCall.increment();
        diesel::insert_into(account_tokens::table)
            .values(req)
            .on_conflict(account_tokens::account_id)
            .do_update()
            .set(account_tokens::token.eq(req.token))
            .get_result(conn)
    }

    pub fn delete(id: u64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(account_tokens::table.find(id as i64)).execute(conn)
    }
}
