use super::db_id_format;
use chrono::NaiveDateTime;
use diesel::{self,
             pg::PgConnection,
             result::QueryResult,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};

use crate::schema::settings::origin_package_settings;

use crate::models::package::PackageVisibility;

use crate::{bldr_core::metrics::CounterMetric,
            metrics::Counter};

#[derive(Debug,
         Serialize,
         Deserialize,
         QueryableByName,
         Queryable,
         Clone,
         AsExpression,
         PartialEq,
         Identifiable)]
#[table_name = "origin_package_settings"]
pub struct OriginPackageSettings {
    #[serde(with = "db_id_format")]
    pub id:         i64,
    pub origin:     String,
    pub name:       String,
    pub visibility: PackageVisibility,
    #[serde(with = "db_id_format")]
    pub owner_id:   i64,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable)]
#[table_name = "origin_package_settings"]
pub struct NewOriginPackageSettings<'a> {
    pub origin:     &'a str,
    pub name:       &'a str,
    pub visibility: &'a PackageVisibility,
    pub owner_id:   i64,
}

#[derive(AsChangeset, Debug)]
#[table_name = "origin_package_settings"]
pub struct UpdateOriginPackageSettings<'a> {
    pub origin:     &'a str,
    pub name:       &'a str,
    pub visibility: &'a PackageVisibility,
    pub owner_id:   i64,
}

#[derive(Debug)]
pub struct GetOriginPackageSettings<'a> {
    pub origin: &'a str,
    pub name:   &'a str,
}

impl OriginPackageSettings {
    pub fn get(req: &GetOriginPackageSettings,
               conn: &PgConnection)
               -> QueryResult<OriginPackageSettings> {
        Counter::DBCall.increment();
        origin_package_settings::table.filter(origin_package_settings::origin.eq(&req.origin))
                                      .filter(origin_package_settings::name.eq(&req.name))
                                      .get_result(conn)
    }

    pub fn create(req: &NewOriginPackageSettings,
                  conn: &PgConnection)
                  -> QueryResult<OriginPackageSettings> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_package_settings::table).values(req)
                                                           .get_result(conn)
    }

    pub fn update(req: &UpdateOriginPackageSettings,
                  conn: &PgConnection)
                  -> QueryResult<OriginPackageSettings> {
        Counter::DBCall.increment();
        diesel::update(origin_package_settings::table
            .filter(origin_package_settings::origin.eq(&req.origin))
            .filter(origin_package_settings::name.eq(&req.name)))
            .set(req)
            .get_result(conn)
    }
}
