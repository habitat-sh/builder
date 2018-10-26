use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Text};
use diesel::RunQueryDsl;
use models::package::{PackageVisibility, PackageVisibilityMapping};
use protocol::originsrv;
use schema::origin::*;

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

#[derive(Clone, Serialize, Deserialize)]
pub struct CreateOrigin {
    pub name: String,
    pub owner_id: i64,
    pub owner_name: String,
    pub default_package_visibility: PackageVisibility,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateOrigin {
    pub name: String,
    pub default_package_visibility: PackageVisibility,
}

impl Origin {
    pub fn get(origin: &str, conn: &PgConnection) -> QueryResult<OriginWithSecretKey> {
        diesel::sql_query(
            "select * from origins_with_secret_key_full_name_v2 where name = $1 limit 1",
        ).bind::<Text, _>(origin)
        .get_result(conn)
    }

    pub fn list(owner_id: i64, conn: &PgConnection) -> QueryResult<Vec<OriginWithStats>> {
        diesel::sql_query("select * from my_origins_with_stats_v2($1)")
            .bind::<BigInt, _>(owner_id)
            .get_results(conn)
    }

    pub fn create(origin: CreateOrigin, conn: &PgConnection) -> QueryResult<Origin> {
        diesel::sql_query("select * from insert_origin_v3($1, $2, $3, $4)")
            .bind::<Text, _>(origin.name)
            .bind::<BigInt, _>(origin.owner_id)
            .bind::<Text, _>(origin.owner_name)
            .bind::<PackageVisibilityMapping, _>(origin.default_package_visibility)
            .get_result(conn)
    }

    pub fn update(origin: UpdateOrigin, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from update_origin_v2($1, $2)")
            .bind::<Text, _>(origin.name)
            .bind::<PackageVisibilityMapping, _>(origin.default_package_visibility)
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
