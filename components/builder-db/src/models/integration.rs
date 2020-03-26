use super::db_id_format;
use chrono::NaiveDateTime;
use diesel::{self,
             dsl::count,
             pg::PgConnection,
             result::QueryResult,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};

use crate::{bldr_core::metrics::CounterMetric,
            metrics::Counter,
            schema::integration::origin_integrations};

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct OriginIntegration {
    #[serde(with = "db_id_format")]
    pub id:          i64,
    pub origin:      String,
    pub integration: String,
    pub name:        String,
    pub body:        String,
    pub created_at:  Option<NaiveDateTime>,
    pub updated_at:  Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "origin_integrations"]
pub struct NewOriginIntegration<'a> {
    pub origin:      &'a str,
    pub integration: &'a str,
    pub name:        &'a str,
    pub body:        &'a str,
}

impl OriginIntegration {
    pub fn create(req: &NewOriginIntegration, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_integrations::table).values(req)
                                                       .execute(conn)
    }

    pub fn get(origin: &str,
               integration: &str,
               name: &str,
               conn: &PgConnection)
               -> QueryResult<OriginIntegration> {
        Counter::DBCall.increment();
        origin_integrations::table.filter(origin_integrations::origin.eq(origin))
                                  .filter(origin_integrations::name.eq(name))
                                  .filter(origin_integrations::integration.eq(integration))
                                  .get_result(conn)
    }

    pub fn delete(origin: &str,
                  integration: &str,
                  name: &str,
                  conn: &PgConnection)
                  -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_integrations::table
                .filter(origin_integrations::origin.eq(origin))
                .filter(origin_integrations::name.eq(name))
                .filter(origin_integrations::integration.eq(integration)),
        )
        .execute(conn)
    }

    pub fn list_for_origin_integration(origin: &str,
                                       integration: &str,
                                       conn: &PgConnection)
                                       -> QueryResult<Vec<OriginIntegration>> {
        Counter::DBCall.increment();
        origin_integrations::table.filter(origin_integrations::origin.eq(origin))
                                  .filter(origin_integrations::integration.eq(integration))
                                  .get_results(conn)
    }

    pub fn list_for_origin(origin: &str,
                           conn: &PgConnection)
                           -> QueryResult<Vec<OriginIntegration>> {
        Counter::DBCall.increment();
        origin_integrations::table.filter(origin_integrations::origin.eq(origin))
                                  .get_results(conn)
    }

    pub fn count_origin_integrations(origin: &str, conn: &PgConnection) -> QueryResult<i64> {
        Counter::DBCall.increment();
        origin_integrations::table.select(count(origin_integrations::id))
                                  .filter(origin_integrations::origin.eq(&origin))
                                  .first(conn)
    }
}
