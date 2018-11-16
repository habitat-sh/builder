use super::db_id_format;
use chrono::NaiveDateTime;
use time::PreciseTime;

use diesel;
use diesel::dsl::sql;
use diesel::pg::expression::dsl::any;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::{
    ExpressionMethods, NullableExpressionMethods, PgArrayExpressionMethods, QueryDsl, RunQueryDsl,
    Table, TextExpressionMethods,
};

use models::package::{BuilderPackageIdent, Package, PackageVisibility};
use models::pagination::Paginate;
use protocol::jobsrv::JobGroupTrigger;
use schema::audit::{audit_package, audit_package_group};
use schema::channel::{origin_channel_packages, origin_channels};
use schema::origin::origins;
use schema::package::origin_packages;

use bldr_core::metrics::{CounterMetric, HistogramMetric};
use metrics::{Counter, Histogram};

#[derive(AsExpression, Debug, Serialize, Deserialize, Queryable)]
pub struct Channel {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub origin: String,
}

#[derive(Insertable)]
#[table_name = "origin_channels"]
pub struct CreateChannel<'a> {
    pub name: &'a str,
    pub owner_id: i64,
    pub origin: &'a str,
}

#[derive(Clone, Debug)]
pub struct GetLatestPackage<'a> {
    pub ident: &'a BuilderPackageIdent,
    pub visibility: &'a Vec<PackageVisibility>,
    pub channel: &'a str,
    pub target: &'a str,
}

pub struct ListChannelPackages<'a> {
    pub ident: &'a BuilderPackageIdent,
    pub visibility: &'a Vec<PackageVisibility>,
    pub channel: &'a str,
    pub origin: &'a str,
    pub page: i64,
    pub limit: i64,
}

impl Channel {
    pub fn list(
        origin: &str,
        include_sandbox_channels: bool,
        conn: &PgConnection,
    ) -> QueryResult<Vec<Channel>> {
        Counter::DBCall.increment();
        let mut query = origin_channels::table
            .select(origin_channels::table::all_columns())
            .filter(origin_channels::origin.eq(origin))
            .into_boxed();
        if !include_sandbox_channels {
            query = query.filter(origin_channels::name.not_like("bldr-%"));
        }
        query.order(origin_channels::name.asc()).get_results(conn)
    }

    pub fn get(origin: &str, channel: &str, conn: &PgConnection) -> QueryResult<Channel> {
        Counter::DBCall.increment();
        origin_channels::table
            .filter(origin_channels::origin.eq(origin))
            .filter(origin_channels::name.eq(channel))
            .get_result(conn)
    }

    pub fn create(channel: CreateChannel, conn: &PgConnection) -> QueryResult<Channel> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_channels::table)
            .values(&channel)
            .get_result(conn)
    }

    pub fn delete(origin: &str, channel: &str, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_channels::table
                .filter(origin_channels::origin.eq(origin))
                .filter(origin_channels::name.eq(channel)),
        ).execute(conn)
    }

    pub fn get_latest_package(req: GetLatestPackage, conn: &PgConnection) -> QueryResult<Package> {
        Counter::DBCall.increment();
        let ident = req.ident;
        let start_time = PreciseTime::now();

        let result = Package::all()
            .inner_join(origin_channel_packages::table.inner_join(origin_channels::table))
            .filter(origin_packages::origin.eq(&ident.origin))
            .filter(origin_channels::name.eq(req.channel))
            .filter(origin_packages::target.eq(req.target))
            .filter(origin_packages::visibility.eq(any(req.visibility)))
            .filter(origin_packages::ident_array.contains(ident.clone().parts()))
            .order(sql::<Package>(
                "to_semver(ident_array[3]) desc, ident_array[4] desc",
            )).limit(1)
            .get_result(conn);

        let end_time = PreciseTime::now();
        trace!(
            "DBCall channel::get_latest_package time: {} ms",
            start_time.to(end_time).num_milliseconds()
        );
        Histogram::DbCallTime.set(start_time.to(end_time).num_milliseconds() as f64);

        result
    }

    pub fn list_packages(
        lcp: ListChannelPackages,
        conn: &PgConnection,
    ) -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();

        origin_packages::table
            .inner_join(
                origin_channel_packages::table
                    .inner_join(origin_channels::table.inner_join(origins::table)),
            ).filter(origin_packages::ident_array.contains(lcp.ident.clone().parts()))
            .filter(origin_packages::visibility.eq(any(lcp.visibility)))
            .filter(origins::name.eq(lcp.origin))
            .filter(origin_channels::name.eq(lcp.channel))
            .select(origin_packages::ident)
            .order(origin_packages::ident.asc())
            .paginate(lcp.page)
            .per_page(lcp.limit)
            .load_and_count_records(conn)
    }

    pub fn promote_packages(
        channel_id: i64,
        package_ids: Vec<i64>,
        conn: &PgConnection,
    ) -> QueryResult<usize> {
        Counter::DBCall.increment();
        let insert: Vec<(_, _)> = package_ids
            .iter()
            .map(|id| {
                (
                    origin_channel_packages::package_id.eq(id),
                    origin_channel_packages::channel_id.eq(channel_id),
                )
            }).collect();
        diesel::insert_into(origin_channel_packages::table)
            .values(insert)
            .execute(conn)
    }

    pub fn demote_packages(
        channel_id: i64,
        package_ids: Vec<i64>,
        conn: &PgConnection,
    ) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_channel_packages::table
                .filter(origin_channel_packages::channel_id.eq(channel_id))
                .filter(origin_channel_packages::package_id.eq(any(package_ids))),
        ).execute(conn)
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

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[table_name = "audit_package"]
pub struct PackageChannelAudit<'a> {
    pub package_ident: BuilderPackageIdent,
    pub channel: &'a str,
    pub operation: PackageChannelOperation,
    pub trigger: PackageChannelTrigger,
    pub requester_id: i64,
    pub requester_name: &'a str,
    pub origin: &'a str,
}

impl<'a> PackageChannelAudit<'a> {
    pub fn audit(pca: PackageChannelAudit, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::insert_into(audit_package::table)
            .values(&pca)
            .execute(conn)
    }
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[table_name = "audit_package_group"]
pub struct PackageGroupChannelAudit<'a> {
    pub origin: &'a str,
    pub channel: &'a str,
    pub package_ids: Vec<i64>,
    pub operation: PackageChannelOperation,
    pub trigger: PackageChannelTrigger,
    pub requester_id: i64,
    pub requester_name: &'a str,
    pub group_id: i64,
}

impl<'a> PackageGroupChannelAudit<'a> {
    pub fn audit(req: PackageGroupChannelAudit, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::insert_into(audit_package_group::table)
            .values(req)
            .execute(conn)
    }
}

#[derive(Debug, Serialize, Queryable)]
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
        Counter::DBCall.increment();
        // If this looks bad, it is. To ensure we get values here or die we have to execute queries to get the IDs first.
        // I can hear the groaning already, "Why can't we just do a sub-select and let the database barf on insert"
        // Great question - Because the typechecking happens in Rust and you wanted a type-safe language
        let channel_id = origin_channels::table
            .filter(origin_channels::name.eq(package.channel))
            .filter(origin_channels::origin.eq(package.origin))
            .select(origin_channels::id)
            .limit(1)
            .get_result::<i64>(conn)?;
        let package_id = origin_packages::table
            .filter(origin_packages::ident.eq(package.ident.to_string()))
            .select(origin_packages::id)
            .limit(1)
            .get_result::<i64>(conn)?;

        diesel::insert_into(origin_channel_packages::table)
            .values((
                origin_channel_packages::channel_id.eq(channel_id),
                origin_channel_packages::package_id.eq(package_id),
            )).on_conflict_do_nothing()
            .execute(conn)
    }
    pub fn demote(package: OriginChannelDemote, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_channel_packages::table
                .filter(
                    origin_channel_packages::channel_id
                        .nullable()
                        .eq(origin_channels::table
                            .select(origin_channels::id)
                            .filter(origin_channels::name.eq(package.channel))
                            .filter(origin_channels::origin.eq(package.origin))
                            .single_value()),
                ).filter(
                    origin_channel_packages::package_id
                        .nullable()
                        .eq(origin_packages::table
                            .select(origin_packages::id)
                            .filter(origin_packages::ident.eq(package.ident.to_string()))
                            .single_value()),
                ),
        ).execute(conn)
    }
}
