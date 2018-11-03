use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Binary, Text};
use diesel::RunQueryDsl;
use schema::key::*;

use bldr_core::metrics::CounterMetric;
use metrics::Counter;

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_public_encryption_keys"]
pub struct OriginPublicEncryptionKey {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub origin_id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub revision: String,
    pub full_name: String,
    pub body: Vec<u8>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_private_encryption_keys"]
pub struct OriginPrivateEncryptionKey {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub origin_id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub revision: String,
    pub full_name: String,
    pub body: Vec<u8>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_secret_keys"]
pub struct OriginPrivateSigningKey {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub origin_id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub revision: String,
    pub full_name: String,
    pub body: Vec<u8>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_public_keys"]
pub struct OriginPublicSigningKey {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub origin_id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub revision: String,
    pub full_name: String,
    pub body: Vec<u8>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "origin_public_encryption_keys"]
pub struct NewOriginPublicEncryptionKey<'a> {
    pub owner_id: i64,
    pub origin_id: i64,
    pub name: &'a str,
    pub revision: &'a str,
    pub body: &'a [u8],
}

#[derive(Insertable)]
#[table_name = "origin_private_encryption_keys"]
pub struct NewOriginPrivateEncryptionKey<'a> {
    pub owner_id: i64,
    pub origin_id: i64,
    pub name: &'a str,
    pub revision: &'a str,
    pub body: &'a [u8],
}

#[derive(Insertable)]
#[table_name = "origin_secret_keys"]
pub struct NewOriginPrivateSigningKey<'a> {
    pub owner_id: i64,
    pub origin_id: i64,
    pub name: &'a str,
    pub revision: &'a str,
    pub body: &'a [u8],
}

#[derive(Insertable)]
#[table_name = "origin_public_keys"]
pub struct NewOriginPublicSigningKey<'a> {
    pub owner_id: i64,
    pub origin_id: i64,
    pub name: &'a str,
    pub revision: &'a str,
    pub body: &'a [u8],
}

impl OriginPublicEncryptionKey {
    pub fn get(
        origin: &str,
        revision: &str,
        conn: &PgConnection,
    ) -> QueryResult<OriginPublicEncryptionKey> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_public_encryption_key_v1($1, $2)")
            .bind::<Text, _>(origin)
            .bind::<Text, _>(revision)
            .get_result(conn)
    }

    pub fn create(
        req: &NewOriginPublicEncryptionKey,
        conn: &PgConnection,
    ) -> QueryResult<OriginPublicEncryptionKey> {
        Counter::DBCall.increment();
        let full_name = format!("{}-{}", req.name, req.revision);
        diesel::sql_query(
            "select * from insert_origin_public_encryption_key_v1($1, $2, $3, $4, $5, $6)",
        ).bind::<BigInt, _>(req.origin_id)
        .bind::<BigInt, _>(req.owner_id)
        .bind::<Text, _>(req.name)
        .bind::<Text, _>(req.revision)
        .bind::<Text, _>(full_name)
        .bind::<Binary, _>(req.body)
        .get_result(conn)
    }

    pub fn latest(origin: &str, conn: &PgConnection) -> QueryResult<OriginPublicEncryptionKey> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_public_encryption_key_latest_v1($1)")
            .bind::<Text, _>(origin)
            .get_result(conn)
    }

    pub fn list(origin: &str, conn: &PgConnection) -> QueryResult<Vec<OriginPublicEncryptionKey>> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_public_encryption_keys_for_origin_v1($1)")
            .bind::<Text, _>(origin)
            .get_results(conn)
    }
}

impl OriginPrivateEncryptionKey {
    pub fn get(origin: &str, conn: &PgConnection) -> QueryResult<OriginPrivateEncryptionKey> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_private_encryption_key_v1($1)")
            .bind::<Text, _>(origin)
            .get_result(conn)
    }

    pub fn create(
        req: &NewOriginPrivateEncryptionKey,
        conn: &PgConnection,
    ) -> QueryResult<OriginPrivateEncryptionKey> {
        Counter::DBCall.increment();
        let full_name = format!("{}-{}", req.name, req.revision);
        diesel::sql_query(
            "select * from insert_origin_private_encryption_key_v1($1, $2, $3, $4, $5, $6)",
        ).bind::<BigInt, _>(req.origin_id)
        .bind::<BigInt, _>(req.owner_id)
        .bind::<Text, _>(req.name)
        .bind::<Text, _>(req.revision)
        .bind::<Text, _>(full_name)
        .bind::<Binary, _>(req.body)
        .get_result(conn)
    }
}

impl OriginPublicSigningKey {
    pub fn get(
        origin: &str,
        revision: &str,
        conn: &PgConnection,
    ) -> QueryResult<OriginPublicSigningKey> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_public_key_v1($1, $2)")
            .bind::<Text, _>(origin)
            .bind::<Text, _>(revision)
            .get_result(conn)
    }

    pub fn create(
        req: &NewOriginPublicSigningKey,
        conn: &PgConnection,
    ) -> QueryResult<OriginPublicSigningKey> {
        Counter::DBCall.increment();
        let full_name = format!("{}-{}", req.name, req.revision);
        diesel::sql_query("select * from insert_origin_public_key_v1($1, $2, $3, $4, $5, $6)")
            .bind::<BigInt, _>(req.origin_id)
            .bind::<BigInt, _>(req.owner_id)
            .bind::<Text, _>(req.name)
            .bind::<Text, _>(req.revision)
            .bind::<Text, _>(full_name)
            .bind::<Binary, _>(req.body)
            .get_result(conn)
    }

    pub fn latest(origin: &str, conn: &PgConnection) -> QueryResult<OriginPublicSigningKey> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_public_key_latest_v1($1)")
            .bind::<Text, _>(origin)
            .get_result(conn)
    }

    pub fn list(origin_id: u64, conn: &PgConnection) -> QueryResult<Vec<OriginPublicSigningKey>> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_public_keys_for_origin_v1($1)")
            .bind::<BigInt, _>(origin_id as i64)
            .get_results(conn)
    }
}

impl OriginPrivateSigningKey {
    pub fn get(origin: &str, conn: &PgConnection) -> QueryResult<OriginPrivateSigningKey> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from get_origin_secret_key_v1($1)")
            .bind::<Text, _>(origin)
            .get_result(conn)
    }

    pub fn create(
        req: &NewOriginPrivateSigningKey,
        conn: &PgConnection,
    ) -> QueryResult<OriginPrivateSigningKey> {
        Counter::DBCall.increment();
        let full_name = format!("{}-{}", req.name, req.revision);
        diesel::sql_query("select * from insert_origin_secret_key_v1($1, $2, $3, $4, $5, $6)")
            .bind::<BigInt, _>(req.origin_id)
            .bind::<BigInt, _>(req.owner_id)
            .bind::<Text, _>(req.name)
            .bind::<Text, _>(req.revision)
            .bind::<Text, _>(full_name)
            .bind::<Binary, _>(req.body)
            .get_result(conn)
    }
}
