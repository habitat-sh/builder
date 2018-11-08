use super::db_id_format;
use chrono::NaiveDateTime;

use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::QueryResult;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use models::channel::{Channel, CreateChannel};
use models::package::PackageVisibility;
use protocol::originsrv;

use schema::member::origin_members;
use schema::origin::{origins, origins_with_secret_key, origins_with_stats};

use bldr_core::metrics::CounterMetric;
use metrics::Counter;

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[table_name = "origins"]
pub struct Origin {
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub default_package_visibility: PackageVisibility,
}

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct OriginWithSecretKey {
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub private_key_name: Option<String>,
    pub default_package_visibility: PackageVisibility,
}

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct OriginWithStats {
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub default_package_visibility: PackageVisibility,
    pub package_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Queryable, QueryableByName, Insertable)]
#[table_name = "origin_members"]
pub struct OriginMember {
    #[serde(with = "db_id_format")]
    pub account_id: i64,
    pub origin: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "origins"]
pub struct NewOrigin<'a> {
    pub name: &'a str,
    pub owner_id: i64,
    pub default_package_visibility: &'a PackageVisibility,
}

impl Origin {
    pub fn get(origin: &str, conn: &PgConnection) -> QueryResult<OriginWithSecretKey> {
        Counter::DBCall.increment();
        origins_with_secret_key::table
            .find(origin)
            .limit(1)
            .get_result(conn)
    }

    pub fn list(owner_id: i64, conn: &PgConnection) -> QueryResult<Vec<OriginWithStats>> {
        Counter::DBCall.increment();
        origins_with_stats::table
            .inner_join(origin_members::table)
            .select(origins_with_stats::table::all_columns())
            .filter(origin_members::account_id.eq(owner_id))
            .order(origins_with_stats::name.asc())
            .get_results(conn)
    }

    pub fn create(req: &NewOrigin, conn: &PgConnection) -> QueryResult<Origin> {
        Counter::DBCall.increment();
        let new_origin = diesel::insert_into(origins::table)
            .values(req)
            .get_result(conn)?;

        OriginMember::add(req.name, req.owner_id, conn)?;
        Channel::create(
            CreateChannel {
                name: "unstable",
                owner_id: req.owner_id,
                origin: req.name,
            },
            conn,
        )?;
        Channel::create(
            CreateChannel {
                name: "stable",
                owner_id: req.owner_id,
                origin: req.name,
            },
            conn,
        )?;

        Ok(new_origin)
    }

    pub fn update(name: &str, dpv: PackageVisibility, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origins::table.find(name))
            .set(origins::default_package_visibility.eq(dpv))
            .execute(conn)
    }

    pub fn check_membership(
        origin: &str,
        account_id: i64,
        conn: &PgConnection,
    ) -> QueryResult<bool> {
        Counter::DBCall.increment();
        origin_members::table
            .filter(origin_members::origin.eq(origin))
            .filter(origin_members::account_id.eq(account_id))
            .execute(conn)
            .and_then(|s| Ok(s > 0))
    }
}

impl OriginMember {
    pub fn list(origin: &str, conn: &PgConnection) -> QueryResult<Vec<String>> {
        use schema::account::accounts;
        use schema::member::origin_members;

        Counter::DBCall.increment();
        origin_members::table
            .inner_join(accounts::table)
            .select(accounts::name)
            .filter(origin_members::origin.eq(origin))
            .order(accounts::name.asc())
            .get_results(conn)
    }

    pub fn delete(origin: &str, account_name: &str, conn: &PgConnection) -> QueryResult<usize> {
        use schema::account::accounts;

        Counter::DBCall.increment();
        diesel::delete(
            origin_members::table
                .filter(origin_members::origin.eq(origin))
                .filter(
                    origin_members::account_id.nullable().eq(accounts::table
                        .select(accounts::id)
                        .filter(accounts::name.eq(account_name))
                        .single_value()),
                ),
        ).execute(conn)
    }

    pub fn add(origin: &str, account_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        diesel::insert_into(origin_members::table)
            .values((
                origin_members::origin.eq(origin),
                origin_members::account_id.eq(account_id),
            )).execute(conn)
    }
}

impl Into<originsrv::Origin> for Origin {
    fn into(self) -> originsrv::Origin {
        let mut orig = originsrv::Origin::new();
        orig.set_owner_id(self.owner_id as u64);
        orig.set_name(self.name);
        orig.set_default_package_visibility(self.default_package_visibility.into());
        orig
    }
}

impl From<originsrv::Origin> for Origin {
    fn from(origin: originsrv::Origin) -> Origin {
        Origin {
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
