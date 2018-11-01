use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::expression::dsl::any;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{Array, BigInt, Bool, Text};
use diesel::RunQueryDsl;
use diesel::{ExpressionMethods, PgArrayExpressionMethods, QueryDsl};
use models::package::{BuilderPackageIdent, Package, PackageVisibility, PackageVisibilityMapping};
use models::pagination::Paginate;
use protocol::jobsrv::JobGroupTrigger;
use schema::channel::*;

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable, Identifiable)]
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

#[derive(Clone, Debug)]
pub struct GetLatestPackage {
    pub ident: BuilderPackageIdent,
    pub visibility: Vec<PackageVisibility>,
    pub channel: String,
    pub target: String,
}

pub struct ListChannelPackages {
    pub ident: BuilderPackageIdent,
    pub visibility: Vec<PackageVisibility>,
    pub channel: String,
    pub origin: String,
    pub page: i64,
    pub limit: i64,
}

pub struct PromotePackages {
    pub channel_id: i64,
    pub pkg_ids: Vec<i64>,
}

pub struct DemotePackages {
    pub channel_id: i64,
    pub pkg_ids: Vec<i64>,
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

    pub fn get_latest_package(req: GetLatestPackage, conn: &PgConnection) -> QueryResult<Package> {
        let ident = req.ident;
        diesel::sql_query("select * from get_origin_channel_package_latest_v8($1, $2, $3, $4, $5)")
            .bind::<Text, _>(&ident.origin)
            .bind::<Text, _>(req.channel)
            .bind::<Array<Text>, _>(ident.clone().parts())
            .bind::<Text, _>(req.target)
            .bind::<Array<PackageVisibilityMapping>, _>(req.visibility)
            .get_result(conn)
    }

    pub fn list_packages(
        lcp: ListChannelPackages,
        conn: &PgConnection,
    ) -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        use schema::channel::{origin_channel_packages, origin_channels};
        use schema::origin::origins;
        use schema::package::origin_packages;

        origin_packages::table
            .inner_join(
                origin_channel_packages::table
                    .inner_join(origin_channels::table.inner_join(origins::table)),
            ).filter(origin_packages::ident_array.contains(lcp.ident.parts()))
            .filter(origin_packages::visibility.eq(any(lcp.visibility)))
            .filter(origins::name.eq(lcp.origin))
            .filter(origin_channels::name.eq(lcp.channel))
            .select(origin_packages::ident)
            .order(origin_packages::ident.asc())
            .paginate(lcp.page)
            .per_page(lcp.limit)
            .load_and_count_records(conn)
    }

    pub fn promote_packages(req: PromotePackages, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from promote_origin_package_group_v1($1, $2)")
            .bind::<BigInt, _>(req.channel_id)
            .bind::<Array<BigInt>, _>(req.pkg_ids)
            .execute(conn)
    }

    pub fn demote_packages(req: DemotePackages, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from demote_origin_package_group_v1($1, $2)")
            .bind::<BigInt, _>(req.channel_id)
            .bind::<Array<BigInt>, _>(req.pkg_ids)
            .execute(conn)
    }
}

#[derive(DbEnum, Debug, Clone, Serialize, Deserialize)]
pub enum PackageChannelTrigger {
    Unknown,
    BuilderUi,
    HabClient,
}

impl From<JobGroupTrigger> for PackageChannelTrigger {
    fn from(value: JobGroupTrigger) -> PackageChannelTrigger {
        match value {
            JobGroupTrigger::HabClient => PackageChannelTrigger::HabClient,
            JobGroupTrigger::BuilderUI => PackageChannelTrigger::BuilderUi,
            _ => PackageChannelTrigger::Unknown,
        }
    }
}

#[derive(DbEnum, Debug, Serialize, Deserialize)]
pub enum PackageChannelOperation {
    Promote,
    Demote,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageChannelAudit<'a> {
    pub origin: &'a str,
    pub ident: BuilderPackageIdent,
    pub channel: &'a str,
    pub operation: PackageChannelOperation,
    pub trigger: PackageChannelTrigger,
    pub requester_id: i64,
    pub requester_name: &'a str,
}

impl<'a> PackageChannelAudit<'a> {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageGroupChannelAudit<'a> {
    pub origin: &'a str,
    pub channel: &'a str,
    pub pkg_ids: Vec<i64>,
    pub operation: PackageChannelOperation,
    pub trigger: PackageChannelTrigger,
    pub requester_id: i64,
    pub requester_name: &'a str,
    pub group_id: i64,
}

impl<'a> PackageGroupChannelAudit<'a> {
    pub fn audit(req: PackageGroupChannelAudit, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query(
            "select * from add_audit_package_group_entry_v2($1, $2, $3, $4, $5, $6, $7, $8)",
        ).bind::<Text, _>(req.origin)
        .bind::<Text, _>(req.channel)
        .bind::<Array<BigInt>, _>(req.pkg_ids)
        .bind::<PackageChannelOperationMapping, _>(req.operation)
        .bind::<PackageChannelTriggerMapping, _>(req.trigger)
        .bind::<BigInt, _>(req.requester_id)
        .bind::<Text, _>(req.requester_name)
        .bind::<BigInt, _>(req.group_id)
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
    pub ident: BuilderPackageIdent,
    pub origin: String,
    pub channel: String,
}
pub struct OriginChannelDemote {
    pub ident: BuilderPackageIdent,
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
