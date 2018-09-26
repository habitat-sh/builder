use actix_web::{actix::Message, Error};
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::Text;
use diesel::RunQueryDsl;
use server::schema::channel::*;

#[derive(Debug, Serialize, QueryableByName)]
#[table_name = "origin_channel_packages"]
pub struct OriginChannelPackage {
    pub channel_id: i64,
    pub package_id: i64,
}

pub struct PromotePackage {
    pub ident: String,
    pub origin: String,
    pub channel: String,
}

impl Message for PromotePackage {
    type Result = Result<(), Error>;
}

impl OriginChannelPackage {
    pub fn promote(package: PromotePackage, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from promote_origin_package_v2($1, $2, $3)")
            .bind::<Text, _>(package.origin)
            .bind::<Text, _>(package.ident)
            .bind::<Text, _>(package.channel)
            .execute(conn)
    }
}
