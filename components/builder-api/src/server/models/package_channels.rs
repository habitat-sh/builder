use actix_web::{actix::Message, Error};
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::Text;
use diesel::RunQueryDsl;
use hab_core::package::PackageIdent;
use server::schema::channel::*;

#[derive(Debug, Serialize, QueryableByName)]
#[table_name = "origin_channel_packages"]
pub struct OriginChannelPackage {
    pub channel_id: i64,
    pub package_id: i64,
}

pub struct PromotePackage {
    pub ident: PackageIdent,
    pub origin: String,
    pub channel: String,
}

pub struct DemotePackage {
    pub ident: PackageIdent,
    pub origin: String,
    pub channel: String,
}

pub enum PackageChannelOperation {
    Promote,
    Demote,
}

pub struct AuditPackageRankChange {
    pub ident: PackageIdent,
    pub origin: String,
    pub channel: String,
    pub operation: PackageChannelOperation,
    pub session_id: i64,
}

impl Message for PromotePackage {
    type Result = Result<(), Error>;
}

impl Message for DemotePackage {
    type Result = Result<(), Error>;
}

impl Message for AuditPackageRankChange {
    type Result = Result<(), Error>;
}

impl OriginChannelPackage {
    pub fn promote(package: PromotePackage, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from promote_origin_package_v2($1, $2, $3)")
            .bind::<Text, _>(package.origin)
            .bind::<Text, _>(package.ident.to_string())
            .bind::<Text, _>(package.channel)
            .execute(conn)
    }

    pub fn demote(package: DemotePackage, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from demote_origin_package_v2($1, $2, $3)")
            .bind::<Text, _>(package.origin)
            .bind::<Text, _>(package.ident.to_string())
            .bind::<Text, _>(package.channel)
            .execute(conn)
    }

    pub fn audit(package: AuditPackageRankChange, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from demote_origin_package_v2($1, $2, $3)")
            .bind::<Text, _>(package.origin)
            .bind::<Text, _>(package.ident.to_string())
            .bind::<Text, _>(package.channel)
            .execute(conn)
    }
}
