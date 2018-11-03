use super::db_id_format;
use chrono::NaiveDateTime;

use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Text};
use diesel::RunQueryDsl;

use models::package::{PackageVisibility, PackageVisibilityMapping};
use protocol::originsrv;

use schema::member::*;
use schema::origin::*;

use bldr_core::metrics::CounterMetric;
use metrics::Counter;

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
    pub default_package_visibility: PackageVisibility,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origins_with_secret_key"]
pub struct OriginWithSecretKey {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub private_key_name: Option<String>,
    pub default_package_visibility: PackageVisibility,
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
    pub default_package_visibility: PackageVisibility,
    pub package_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Queryable, QueryableByName)]
#[table_name = "origin_members"]
pub struct OriginMember {
    #[serde(with = "db_id_format")]
    pub origin_id: i64,
    #[serde(with = "db_id_format")]
    pub account_id: i64,
    pub origin_name: String,
    pub account_name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

// #[derive(Insertable)]
// #[table_name = "origins"]
// TODO: Make this directly insertable?
pub struct NewOrigin<'a> {
    pub name: &'a str,
    pub owner_id: i64,
    pub owner_name: &'a str,
    pub default_package_visibility: &'a PackageVisibility,
}

impl Origin {
    pub fn get(origin: &str, conn: &PgConnection) -> QueryResult<OriginWithSecretKey> {
        Counter::DBCall.increment();
        diesel::sql_query(
            "select * from origins_with_secret_key_full_name_v2 where name = $1 limit 1",
        ).bind::<Text, _>(origin)
        .get_result(conn)
    }

    pub fn list(owner_id: i64, conn: &PgConnection) -> QueryResult<Vec<OriginWithStats>> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from my_origins_with_stats_v2($1)")
            .bind::<BigInt, _>(owner_id)
            .get_results(conn)
    }

    pub fn create(req: &NewOrigin, conn: &PgConnection) -> QueryResult<Origin> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from insert_origin_v3($1, $2, $3, $4)")
            .bind::<Text, _>(req.name)
            .bind::<BigInt, _>(req.owner_id)
            .bind::<Text, _>(req.owner_name)
            .bind::<PackageVisibilityMapping, _>(req.default_package_visibility)
            .get_result(conn)
    }

    pub fn update(name: &str, dpv: PackageVisibility, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from update_origin_v2($1, $2)")
            .bind::<Text, _>(name)
            .bind::<PackageVisibilityMapping, _>(dpv)
            .execute(conn)
    }

    pub fn check_membership(
        origin: &str,
        account_id: u64,
        conn: &PgConnection,
    ) -> QueryResult<bool> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from check_account_in_origin_members_v1($1, $2)")
            .bind::<Text, _>(origin)
            .bind::<BigInt, _>(account_id as i64)
            .execute(conn)
            .and_then(|s| Ok(s > 0))
    }
}

impl OriginMember {
    pub fn list(origin: &str, conn: &PgConnection) -> QueryResult<Vec<OriginMember>> {
        use schema::member::origin_members;
        use schema::origin::origins;

        Counter::DBCall.increment();
        origin_members::table
            .select(origin_members::table::all_columns())
            .inner_join(origins::table)
            .filter(origins::name.eq(origin))
            .order(origin_members::account_name.asc())
            .get_results(conn)
    }

    pub fn delete(origin_id: u64, account_name: &str, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::sql_query("select * from delete_origin_member_v1($1, $2)")
            .bind::<BigInt, _>(origin_id as i64)
            .bind::<Text, _>(account_name)
            .execute(conn)
    }
}

impl Into<originsrv::Origin> for Origin {
    fn into(self) -> originsrv::Origin {
        let mut orig = originsrv::Origin::new();
        orig.set_id(self.id as u64);
        orig.set_owner_id(self.owner_id as u64);
        orig.set_name(self.name);
        orig.set_default_package_visibility(self.default_package_visibility.into());
        orig
    }
}

impl From<originsrv::Origin> for Origin {
    fn from(origin: originsrv::Origin) -> Origin {
        Origin {
            id: origin.get_id() as i64,
            owner_id: origin.get_owner_id() as i64,
            name: origin.get_name().to_string(),
            default_package_visibility: PackageVisibility::from(
                origin.get_default_package_visibility(),
            ),
            created_at: None,
            updated_at: None,
        }
    }
}
