use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Text};
use diesel::RunQueryDsl;
use server::schema::account::*;

// Accounts
#[derive(Debug, Serialize, QueryableByName)]
#[table_name = "accounts"]
pub struct Account {
    pub id: i64,
    pub email: String,
    pub name: String,
}

pub struct GetAccount {
    pub name: String,
}

pub struct GetAccountById {
    pub id: i64,
}

pub struct CreateAccount {
    pub name: String,
    pub email: String,
}

pub struct UpdateAccount {
    pub id: i64,
    pub email: String,
}

pub struct FindOrCreateAccount {
    name: String,
    email: String,
}

impl Account {
    pub fn get(account: GetAccount, conn: &PgConnection) -> QueryResult<Account> {
        diesel::sql_query("SELECT * FROM get_account_by_name_v1($1)")
            .bind::<Text, _>(account.name)
            .get_result(conn)
    }

    pub fn get_by_id(account: GetAccountById, conn: &PgConnection) -> QueryResult<Account> {
        diesel::sql_query("SELECT * FROM get_account_by_id_v1($1)")
            .bind::<BigInt, _>(account.id)
            .get_result(conn)
    }

    pub fn create(account: CreateAccount, conn: &PgConnection) -> QueryResult<Account> {
        diesel::sql_query("SELECT * FROM select_or_insert_account_v1($1, $2)")
            .bind::<Text, _>(account.name)
            .bind::<Text, _>(account.email)
            .get_result(conn)
    }

    pub fn find_or_create(
        account: FindOrCreateAccount,
        conn: &PgConnection,
    ) -> QueryResult<Account> {
        diesel::sql_query("SELECT * FROM select_or_insert_account_v1($1, $2)")
            .bind::<Text, _>(account.name)
            .bind::<Text, _>(account.email)
            .get_result(conn)
    }

    pub fn update(account: UpdateAccount, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("SELECT update_account_v1($1, $2)")
            .bind::<BigInt, _>(account.id)
            .bind::<Text, _>(account.email)
            .execute(conn)
    }
}
