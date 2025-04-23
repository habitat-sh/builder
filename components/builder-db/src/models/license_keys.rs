use super::db_id_format;
use chrono::NaiveDateTime;
use diesel::{self,
             pg::PgConnection,
             result::QueryResult,
             ExpressionMethods,
             OptionalExtension,
             QueryDsl,
             RunQueryDsl};

use crate::{bldr_core::metrics::CounterMetric,
            metrics::Counter,
            schema::license_keys::license_keys};

#[derive(Debug, Identifiable, Serialize, Queryable)]
pub struct LicenseKey {
    #[serde(with = "db_id_format")]
    pub id:              i64,
    #[serde(with = "db_id_format")]
    pub account_id:      i64,
    pub license_key:     String,
    pub expiration_date: String,
    pub created_at:      Option<NaiveDateTime>,
}

pub struct NewLicenseKey<'a> {
    pub account_id:      i64,
    pub license_key:     &'a str,
    pub expiration_date: &'a str,
}

impl LicenseKey {
    pub fn create(req: &NewLicenseKey, conn: &PgConnection) -> QueryResult<LicenseKey> {
        Counter::DBCall.increment();

        diesel::insert_into(license_keys::table).values((
            license_keys::account_id.eq(req.account_id),
            license_keys::license_key.eq(req.license_key),
            license_keys::expiration_date.eq(req.expiration_date),
        ))
        .on_conflict(license_keys::account_id)
        .do_update()
        .set((
            license_keys::license_key.eq(req.license_key),
            license_keys::expiration_date.eq(req.expiration_date),
        ))
        .get_result(conn)
    }

    pub fn delete_by_account_id(account_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();

        diesel::delete(license_keys::table.filter(license_keys::account_id.eq(account_id)))
            .execute(conn)
    }

    pub fn get_by_account_id(account_id: i64,
                             conn: &PgConnection)
                             -> QueryResult<Option<LicenseKey>> {
        Counter::DBCall.increment();

        license_keys::table.filter(license_keys::account_id.eq(account_id))
                           .first::<LicenseKey>(conn)
                           .optional()
    }
}
