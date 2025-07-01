use super::db_id_format;
use chrono::NaiveDateTime;
use diesel::{self,
             dsl::count,
             pg::PgConnection,
             result::QueryResult,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};

use crate::schema::secrets::*;

use crate::{bldr_core::metrics::CounterMetric,
            metrics::Counter};

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct OriginSecret {
    #[serde(with = "db_id_format")]
    pub id:         i64,
    pub owner_id:   Option<i64>,
    pub name:       String,
    pub value:      String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub origin:     String,
}

// This is the struct the hab client expects
#[derive(Debug, Serialize, Deserialize)]
pub struct OriginSecretWithOriginId {
    pub id:        String,
    pub origin_id: String,
    pub name:      String,
    pub value:     String,
}

#[derive(Insertable)]
#[diesel(table_name = origin_secrets)]
pub struct NewOriginSecret<'a> {
    pub owner_id: i64,
    pub origin:   &'a str,
    pub name:     &'a str,
    pub value:    &'a str,
}

impl OriginSecret {
    pub fn create(secret: &NewOriginSecret, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_secrets::table).values(secret)
                                                  .execute(conn)
    }

    pub fn get(origin: &str, name: &str, conn: &mut PgConnection) -> QueryResult<OriginSecret> {
        Counter::DBCall.increment();
        origin_secrets::table.filter(origin_secrets::name.eq(name))
                             .filter(origin_secrets::origin.eq(origin))
                             .get_result(conn)
    }

    pub fn delete(origin: &str, name: &str, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_secrets::table
                .filter(origin_secrets::name.eq(name))
                .filter(origin_secrets::origin.eq(origin)),
        )
        .execute(conn)
    }

    pub fn list(origin: &str, conn: &mut PgConnection) -> QueryResult<Vec<OriginSecret>> {
        Counter::DBCall.increment();
        origin_secrets::table.filter(origin_secrets::origin.eq(origin))
                             .get_results(conn)
    }

    pub fn count_origin_secrets(origin: &str, conn: &mut PgConnection) -> QueryResult<i64> {
        Counter::DBCall.increment();
        origin_secrets::table.select(count(origin_secrets::id))
                             .filter(origin_secrets::origin.eq(&origin))
                             .first(conn)
    }
}

impl From<OriginSecret> for OriginSecretWithOriginId {
    fn from(value: OriginSecret) -> OriginSecretWithOriginId {
        OriginSecretWithOriginId { id:        format!("{}", value.id),
                                   origin_id: "0".to_string(),
                                   name:      value.name,
                                   value:     value.value, }
    }
}
