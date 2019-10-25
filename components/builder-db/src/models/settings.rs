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
         Identifiable)]
#[table_name = "origin_package_settings"]
pub struct OriginPackageSettings {
    #[serde(with = "db_id_format")]
    pub id: i64,
    pub origin: String,
    pub name: String,
    pub visibility: PackageVisibility,
    pub owner_id: i64,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable)]
#[table_name = "origin_package_settings"]
pub struct NewOriginPackageSettings {
    pub origin:     String,
    pub name:       String,
    pub visibility: PackageVisibility,
    pub owner_id:   i64,
}

#[derive(AsChangeset, Debug)]
#[table_name = "origin_package_settings"]
pub struct UpdateOriginPackageSettings {
    pub origin:     String,
    pub name:       String,
    pub visibility: PackageVisibility,
    pub owner_id:   i64,
}

#[derive(Debug)]
pub struct GetOriginPackageSettings {
    pub origin: String,
    pub name:   String,
}

#[derive(Debug)]
pub struct DeleteOriginPackageSettings {
    pub origin: String,
    pub name:   String,
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

    // pub fn delete(req: &DeleteOriginPackageSettings, conn: &PgConnection) -> QueryResult<usize> {
    //     unimplemented!();
    //     Counter::DBCall.increment();
    //     diesel::delete(origin_package_settings::table.filter(origin_package_settings::name.
    // eq(self.name))).execute(conn) }

    pub fn create(req: &NewOriginPackageSettings,
                  conn: &PgConnection)
                  -> QueryResult<OriginPackageSettings> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_package_settings::table).values(req)
                                                           .get_result(conn)
    }

    pub fn update(req: &UpdateOriginPackageSettings, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_package_settings::table
            .filter(origin_package_settings::origin.eq(&req.origin))
            .filter(origin_package_settings::name.eq(&req.name)))
            .set(req)
            .execute(conn)
    }

    // pub fn list(origin: &str, conn: &PgConnection) -> QueryResult<Vec<OriginPackageSettings>> {
    //     unimplemented!();
    //     Counter::DBCall.increment();
    //     origin_projects::table.filter(origin_package_settings::origin.eq(origin))
    //                           .get_results(conn)
    // }
}
