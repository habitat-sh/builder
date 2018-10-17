use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Bool, Text};
use diesel::RunQueryDsl;
use hab_core::package::PackageIdent;
use schema::channel::*;

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
#[table_name = "origin_channels"]
pub struct Channel {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub origin_id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

pub struct CreateChannel {
    pub channel: String,
    pub owner_id: i64,
    pub origin: String,
}

pub struct ListChannels {
    pub origin: String,
    pub include_sandbox_channels: bool,
}

pub struct GetChannel {
    pub origin: String,
    pub channel: String,
}

pub struct DeleteChannel {
    pub origin: String,
    pub channel: String,
}

impl Channel {
    pub fn list(channel: ListChannels, conn: &PgConnection) -> QueryResult<Vec<Channel>> {
        diesel::sql_query("select * from get_origin_channels_for_origin_v3($1, $2)")
            .bind::<Text, _>(channel.origin)
            .bind::<Bool, _>(channel.include_sandbox_channels)
            .get_results(conn)
    }

    pub fn get(channel: GetChannel, conn: &PgConnection) -> QueryResult<Channel> {
        diesel::sql_query("select * from get_origin_channel_v1($1, $2)")
            .bind::<Text, _>(channel.origin)
            .bind::<Text, _>(channel.channel)
            .get_result(conn)
    }

    pub fn create(channel: CreateChannel, conn: &PgConnection) -> QueryResult<Channel> {
        diesel::sql_query("select * from insert_origin_channel_v2($1, $2, $3)")
            .bind::<Text, _>(channel.origin)
            .bind::<BigInt, _>(channel.owner_id)
            .bind::<Text, _>(channel.channel)
            .get_result(conn)
    }

    pub fn delete(channel: DeleteChannel, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from delete_origin_channel_v2($1, $2)")
            .bind::<Text, _>(channel.channel)
            .bind::<Text, _>(channel.origin)
            .execute(conn)
    }
}

#[derive(DbEnum, Debug, Serialize, Deserialize)]
pub enum PackageChannelTrigger {
    Unknown,
    BuilderUi,
    HabClient,
}

#[derive(DbEnum, Debug, Serialize, Deserialize)]
pub enum PackageChannelOperation {
    Promote,
    Demote,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageChannelAudit {
    pub origin: String,
    pub ident: PackageIdent,
    pub channel: String,
    pub operation: PackageChannelOperation,
    pub trigger: PackageChannelTrigger,
    pub requester_id: i64,
    pub requester_name: String,
}

impl PackageChannelAudit {
    pub fn audit(pca: PackageChannelAudit, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from add_audit_package_entry_v3($1, $2, $3, $4, $5, $6, $7)")
            .bind::<Text, _>(pca.origin)
            .bind::<Text, _>(pca.ident.to_string())
            .bind::<Text, _>(pca.channel)
            .bind::<PackageChannelOperationMapping, _>(pca.operation)
            .bind::<PackageChannelTriggerMapping, _>(pca.trigger)
            .bind::<BigInt, _>(pca.requester_id)
            .bind::<Text, _>(pca.requester_name)
            .execute(conn)
    }
}

#[derive(Debug, Serialize, QueryableByName)]
#[table_name = "origin_channel_packages"]
pub struct OriginChannelPackage {
    pub channel_id: i64,
    pub package_id: i64,
}

pub struct OriginChannelPromote {
    pub ident: PackageIdent,
    pub origin: String,
    pub channel: String,
}
pub struct OriginChannelDemote {
    pub ident: PackageIdent,
    pub origin: String,
    pub channel: String,
}

impl OriginChannelPackage {
    pub fn promote(package: OriginChannelPromote, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from promote_origin_package_v3($1, $2, $3)")
            .bind::<Text, _>(package.origin)
            .bind::<Text, _>(package.ident.to_string())
            .bind::<Text, _>(package.channel)
            .execute(conn)
    }
    pub fn demote(package: OriginChannelDemote, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from demote_origin_package_v3($1, $2, $3)")
            .bind::<Text, _>(package.origin)
            .bind::<Text, _>(package.ident.to_string())
            .bind::<Text, _>(package.channel)
            .execute(conn)
    }
}
