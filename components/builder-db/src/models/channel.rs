use super::db_id_format;
use chrono::NaiveDateTime;
use std::time::Instant;

use crate::{models::{package::{BuilderPackageIdent,
                               PackageVisibility,
                               PackageWithVersionArray},
                     pagination::Paginate},
            schema::{audit::{audit_package,
                             audit_package_group},
                     channel::{origin_channel_packages,
                               origin_channels},
                     member::origin_members,
                     origin::origins,
                     package::{origin_packages,
                               origin_packages_with_version_array}}};

use crate::{bldr_core::metrics::{CounterMetric,
                                 HistogramMetric},
            hab_core::{package::PackageTarget,
                       ChannelIdent},
            metrics::{Counter,
                      Histogram}};

use diesel::{self,
             dsl::{count,
                   sql,
                   IntervalDsl},
             pg::{expression::dsl::any,
                  PgConnection},
             prelude::*,
             result::QueryResult,
             sql_types::Timestamptz,
             ExpressionMethods,
             NullableExpressionMethods,
             PgArrayExpressionMethods,
             QueryDsl,
             RunQueryDsl,
             Table,
             TextExpressionMethods};
use diesel_full_text_search::{to_tsquery,
                              TsQueryExtensions};

#[derive(AsExpression, Debug, Serialize, Deserialize, Queryable)]
pub struct Channel {
    #[serde(with = "db_id_format")]
    pub id:         i64,
    #[serde(with = "db_id_format")]
    pub owner_id:   i64,
    pub name:       String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub origin:     String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelWithPromotion {
    pub name:        String,
    pub created_at:  Option<NaiveDateTime>,
    pub promoted_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "origin_channels"]
pub struct CreateChannel<'a> {
    // This would be ChannelIdent, but Insertable requires implementing diesel::Expression
    pub name:     &'a str,
    pub owner_id: i64,
    pub origin:   &'a str,
}

#[derive(Clone, Debug)]
pub struct GetLatestPackage<'a> {
    pub ident:      &'a BuilderPackageIdent,
    pub visibility: &'a Vec<PackageVisibility>,
    pub channel:    &'a ChannelIdent,
    pub target:     &'a str,
}

pub struct ListChannelPackages<'a> {
    pub ident:      &'a BuilderPackageIdent,
    pub visibility: &'a Vec<PackageVisibility>,
    pub channel:    &'a ChannelIdent,
    pub origin:     &'a str,
    pub page:       i64,
    pub limit:      i64,
}

pub struct ListAllChannelPackages<'a> {
    pub visibility: &'a Vec<PackageVisibility>,
    pub channel:    &'a ChannelIdent,
    pub origin:     &'a str,
}

pub struct ListAllChannelPackagesForTarget<'a> {
    pub visibility: &'a Vec<PackageVisibility>,
    pub channel:    &'a ChannelIdent,
    pub origin:     &'a str,
    pub target:     &'a str,
}

impl Channel {
    // Here because it keeps it near the filter in Channel::list
    pub fn channel_for_group(group_id: u64) -> String { format!("bldr-{}", group_id) }

    pub fn list(origin: &str,
                include_sandbox_channels: bool,
                conn: &PgConnection)
                -> QueryResult<Vec<Channel>> {
        Counter::DBCall.increment();
        let mut query = origin_channels::table.select(origin_channels::table::all_columns())
                                              .filter(origin_channels::origin.eq(origin))
                                              .into_boxed();
        if !include_sandbox_channels {
            query = query.filter(origin_channels::name.not_like("bldr-%"));
        }
        query.order(origin_channels::name.asc()).get_results(conn)
    }

    pub fn get(origin: &str, channel: &ChannelIdent, conn: &PgConnection) -> QueryResult<Channel> {
        Counter::DBCall.increment();
        origin_channels::table.filter(origin_channels::origin.eq(origin))
                              .filter(origin_channels::name.eq(channel.as_str()))
                              .get_result(conn)
    }

    pub fn create(channel: &CreateChannel, conn: &PgConnection) -> QueryResult<Channel> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_channels::table).values(channel)
                                                   .get_result(conn)
    }

    pub fn delete(origin: &str, channel: &ChannelIdent, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_channels::table
                .filter(origin_channels::origin.eq(origin))
                .filter(origin_channels::name.eq(channel.as_str())),
        )
        .execute(conn)
    }

    pub fn delete_channel_package(package_id: i64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_channel_packages::table
                .filter(origin_channel_packages::package_id.eq(package_id)),
        )
        .execute(conn)
    }

    pub fn get_latest_package(req: &GetLatestPackage,
                              conn: &PgConnection)
                              -> QueryResult<PackageWithVersionArray> {
        Counter::DBCall.increment();
        let ident = req.ident;
        let start_time = Instant::now();

        let result = PackageWithVersionArray::all()
            .inner_join(origin_channel_packages::table.inner_join(origin_channels::table))
            .filter(origin_packages_with_version_array::origin.eq(&ident.origin))
            .filter(origin_packages_with_version_array::name.eq(&ident.name))
            .filter(origin_packages_with_version_array::ident_array.contains(ident.clone().parts()))
            .filter(origin_channels::name.eq(req.channel.as_str()))
            .filter(origin_packages_with_version_array::target.eq(req.target))
            .filter(origin_packages_with_version_array::visibility.eq(any(req.visibility)))
            .order(sql::<PackageWithVersionArray>(
                "string_to_array(version_array[1],'.')::\
                 numeric[] desc, version_array[2] desc, \
                 ident_array[4] desc",
            ))
            .limit(1)
            .get_result(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall channel::get_latest_package time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::ChannelGetLatestPackageCallTime.set(duration_millis as f64);

        result
    }

    pub fn list_latest_packages(req: &ListAllChannelPackagesForTarget,
                                conn: &PgConnection)
                                -> QueryResult<(String, String, Vec<BuilderPackageIdent>)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();
        let channel = String::from(req.channel.as_str());
        let target = String::from(req.target);

        let query = origin_packages_with_version_array::table
            .inner_join(origin_channel_packages::table.inner_join(origin_channels::table))
            .filter(origin_packages_with_version_array::origin.eq(&req.origin))
            .filter(origin_channels::name.eq(&channel))
            .filter(origin_packages_with_version_array::target.eq(&target))
            .filter(origin_packages_with_version_array::visibility.eq(any(req.visibility)))
            .distinct_on(origin_packages_with_version_array::name)
            .select((
                origin_packages_with_version_array::name,
                origin_packages_with_version_array::ident,
            ))
            .order(origin_packages_with_version_array::name)
            .order(sql::<PackageWithVersionArray>(
                "name,\
                string_to_array(version_array[1],'.')::numeric[] desc,\
                version_array[2] desc,\
                ident_array[4] desc",
            ));

        // The query returns name, ident because of the way distinct works.
        // I could wrap it all in a subquery, but hit some snags doing that with Diesel.account
        // Instead, I'm going to just extract the Ident here
        let result: QueryResult<Vec<(String, BuilderPackageIdent)>> = query.get_results(conn);
        let result: QueryResult<Vec<BuilderPackageIdent>> =
            result.map(|v: Vec<(String, BuilderPackageIdent)>| {
                      v.iter().map(|(_, ident)| ident.clone()).collect()
                  });

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall channel::list_latest_package time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::ChannelListLatestPackagesCallTime.set(duration_millis as f64);

        result.map(|x| (channel, target, x))
    }

    pub fn list_packages(lcp: &ListChannelPackages,
                         conn: &PgConnection)
                         -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let mut query = origin_packages::table
            .inner_join(
                origin_channel_packages::table
                    .inner_join(origin_channels::table.inner_join(origins::table)),
            )
            .filter(origin_packages::origin.eq(&lcp.ident.origin))
            .into_boxed();
        // We need the into_boxed above to be able to conditionally filter and not break the
        // typesystem.
        if !lcp.ident.name.is_empty() {
            query = query.filter(origin_packages::name.eq(&lcp.ident.name))
        };
        let query = query.filter(origin_packages::ident_array.contains(lcp.ident.clone().parts()))
                         .filter(origin_packages::visibility.eq(any(lcp.visibility)))
                         .filter(origins::name.eq(lcp.origin))
                         .filter(origin_channels::name.eq(lcp.channel.as_str()))
                         .select(origin_packages::ident)
                         .order(origin_packages::ident.asc())
                         .paginate(lcp.page)
                         .per_page(lcp.limit);
        // helpful trick when debugging queries, this has Debug trait:
        // diesel::query_builder::debug_query::<diesel::pg::Pg, _>(&query)

        let result = query.load_and_count_records(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall channel::list_package time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::ChannelListPackagesCallTime.set(duration_millis as f64);

        // Package list for a whole origin is still not very
        // performant, and we want to track that
        if !lcp.ident.name.is_empty() {
            Histogram::ChannelListPackagesOriginOnlyCallTime.set(duration_millis as f64);
        } else {
            Histogram::ChannelListPackagesOriginNameCallTime.set(duration_millis as f64);
        }

        result
    }

    pub fn list_all_packages(lacp: &ListAllChannelPackages,
                             conn: &PgConnection)
                             -> QueryResult<Vec<BuilderPackageIdent>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        // TODO check that this join is using an appropriate index
        let result = origin_packages::table
            .inner_join(
                origin_channel_packages::table
                    .inner_join(origin_channels::table.inner_join(origins::table)),
            )
            .filter(origin_packages::visibility.eq(any(lacp.visibility)))
            .filter(origins::name.eq(lacp.origin))
            .filter(origin_channels::name.eq(lacp.channel.as_str()))
            .select(origin_packages::ident)
            .order(origin_packages::ident.asc())
            .get_results(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall channel::list_all_packages time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::ChannelListAllPackagesCallTime.set(duration_millis as f64);
        result
    }

    pub fn list_all_packages_by_channel_id(channel_id: i64,
                                           visibility: &[PackageVisibility],
                                           conn: &PgConnection)
                                           -> QueryResult<Vec<i64>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        // TODO check that this join is using an appropriate index
        let result =
            origin_packages::table.inner_join(origin_channel_packages::table)
                                  .filter(origin_packages::visibility.eq(any(visibility)))
                                  .filter(origin_channel_packages::channel_id.eq(channel_id))
                                  .select(origin_packages::id)
                                  .order(origin_packages::id)
                                  .get_results(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall channel::list_all_packages_by_channel_id time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::ChannelListAllPackagesCallTime.set(duration_millis as f64);
        result
    }

    pub fn count_origin_channels(origin: &str, conn: &PgConnection) -> QueryResult<i64> {
        Counter::DBCall.increment();
        origin_channels::table.select(count(origin_channels::id))
                              .filter(origin_channels::origin.eq(&origin))
                              .first(conn)
    }

    pub fn promote_packages(channel_id: i64,
                            package_ids: &[i64],
                            conn: &PgConnection)
                            -> QueryResult<usize> {
        Counter::DBCall.increment();
        let insert: Vec<(_, _)> = package_ids.iter()
                                             .map(|id| {
                                                 (origin_channel_packages::package_id.eq(id),
                            origin_channel_packages::channel_id.eq(channel_id))
                                             })
                                             .collect();
        diesel::insert_into(origin_channel_packages::table).values(insert)
                                                           .on_conflict_do_nothing()
                                                           .execute(conn)
    }

    pub fn demote_packages(channel_id: i64,
                           package_ids: &[i64],
                           conn: &PgConnection)
                           -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_channel_packages::table
                .filter(origin_channel_packages::channel_id.eq(channel_id))
                .filter(origin_channel_packages::package_id.eq(any(package_ids))),
        )
        .execute(conn)
    }

    //
    pub fn do_promote_or_demote_packages_cross_channels(ch_source: i64,
                                                        ch_target: i64,
                                                        promote: bool,
                                                        conn: &PgConnection)
                                                        -> QueryResult<Vec<i64>> {
        let pkg_ids: Vec<i64> =
            Channel::list_all_packages_by_channel_id(ch_source, &PackageVisibility::all(), conn)?;

        if promote {
            debug!("Bulk promoting Pkg IDs: {:?}", &pkg_ids);
            Channel::promote_packages(ch_target, &pkg_ids, conn)?;
        } else {
            debug!("Bulk demoting Pkg IDs: {:?}", &pkg_ids);
            Channel::demote_packages(ch_target, &pkg_ids, conn)?;
        }
        Ok(pkg_ids)
    }
}

#[derive(DbEnum, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PackageChannelTrigger {
    Unknown,
    BuilderUi,
    HabClient,
}

#[derive(Clone, DbEnum, Debug, Serialize, Deserialize, PartialEq)]
pub enum PackageChannelOperation {
    Promote,
    Demote,
}

pub struct ListEvents {
    pub account_id: Option<i64>,
    pub page:       i64,
    pub limit:      i64,
    pub channel:    String,
    pub from_date:  NaiveDateTime,
    pub to_date:    NaiveDateTime,
    pub query:      String,
}

#[derive(Debug, Serialize, Deserialize, Queryable, PartialEq)]
pub struct AuditPackage {
    pub package_ident:  BuilderPackageIdent,
    pub channel:        String,
    pub operation:      PackageChannelOperation,
    pub trigger:        PackageChannelTrigger,
    #[serde(with = "db_id_format")]
    pub requester_id:   i64,
    pub requester_name: String,
    pub created_at:     Option<NaiveDateTime>,
    pub origin:         String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AuditPackageEvent {
    pub operation:     PackageChannelOperation,
    pub created_at:    Option<NaiveDateTime>,
    pub origin:        String,
    pub channel:       String,
    pub package_ident: BuilderPackageIdent,
}

impl AuditPackage {
    pub fn list(el: ListEvents, conn: &PgConnection) -> QueryResult<(Vec<AuditPackage>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let mut query = audit_package::table
            .left_join(
                origin_packages::table.on(origin_packages::ident.eq(audit_package::package_ident)),
            )
            .select(audit_package::all_columns)
            .distinct_on((audit_package::package_ident, audit_package::created_at))
            .into_boxed();

        if !el.query.is_empty() {
            query = query.filter(
                to_tsquery(format!("{}:*", el.query)).matches(origin_packages::ident_vector),
            );
        }

        if let Some(session_id) = el.account_id {
            let origins = origin_members::table.select(origin_members::origin)
                                               .filter(origin_members::account_id.eq(session_id));
            query = query.filter(
                origin_packages::visibility
                    .eq(any(PackageVisibility::private()))
                    .and(origin_packages::origin.eq_any(origins))
                    .or(origin_packages::visibility.eq(PackageVisibility::Public))
                    .or(audit_package::requester_id.eq(session_id)),
            );
        } else {
            query = query.filter(origin_packages::visibility.eq(PackageVisibility::Public));
        }

        // to_date is inclusive, add '1' to the to_date so we can easily compare using less than
        query = query.filter(
            audit_package::created_at
                .ge(el.from_date.into_sql::<Timestamptz>().nullable())
                .and(
                    audit_package::created_at
                        .lt((el.to_date.into_sql::<Timestamptz>() + 1.days()).nullable()),
                ),
        );

        if !el.channel.is_empty() {
            query = query.filter(audit_package::channel.eq(el.channel));
        }

        let query = query.order((audit_package::created_at.desc(),
                                 audit_package::package_ident.desc()))
                         .paginate(el.page)
                         .per_page(el.limit);
        let (events, total_count): (std::vec::Vec<AuditPackage>, i64) =
            query.load_and_count_records(conn)?;
        let duration_millis = start_time.elapsed().as_millis();
        Histogram::DbCallTime.set(duration_millis as f64);

        Ok((events, total_count))
    }
}

impl From<AuditPackage> for AuditPackageEvent {
    fn from(value: AuditPackage) -> AuditPackageEvent {
        AuditPackageEvent { operation:     value.operation,
                            created_at:    value.created_at,
                            origin:        value.origin.clone(),
                            channel:       value.channel.clone(),
                            package_ident: value.package_ident, }
    }
}

impl From<Channel> for ChannelWithPromotion {
    fn from(value: Channel) -> ChannelWithPromotion {
        ChannelWithPromotion { name:        value.name.clone(),
                               created_at:  value.created_at,
                               promoted_at: value.updated_at, }
    }
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[table_name = "audit_package"]
pub struct PackageChannelAudit<'a> {
    pub package_ident:  BuilderPackageIdent,
    // This would be ChannelIdent, but Insertable requires implementing diesel::Expression
    pub channel:        &'a str,
    pub operation:      PackageChannelOperation,
    pub trigger:        PackageChannelTrigger,
    pub requester_id:   i64,
    pub requester_name: &'a str,
    pub origin:         &'a str,
}

impl<'a> PackageChannelAudit<'a> {
    pub fn audit(pca: &PackageChannelAudit, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::insert_into(audit_package::table).values(pca)
                                                 .execute(conn)
    }
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[table_name = "audit_package_group"]
pub struct PackageGroupChannelAudit<'a> {
    pub origin:         &'a str,
    // This would be ChannelIdent, but Insertable requires implementing diesel::Expression
    pub channel:        &'a str,
    pub package_ids:    Vec<i64>,
    pub operation:      PackageChannelOperation,
    pub trigger:        PackageChannelTrigger,
    pub requester_id:   i64,
    pub requester_name: &'a str,
    pub group_id:       i64,
}

impl<'a> PackageGroupChannelAudit<'a> {
    pub fn audit(req: PackageGroupChannelAudit, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::insert_into(audit_package_group::table).values(req)
                                                       .execute(conn)
    }
}

#[derive(Debug, Serialize, Queryable)]
pub struct OriginChannelPackage {
    pub channel_id: i64,
    pub package_id: i64,
}

pub struct OriginChannelPromote {
    pub ident:   BuilderPackageIdent,
    pub target:  PackageTarget,
    pub origin:  String,
    pub channel: ChannelIdent,
}

pub struct OriginChannelDemote {
    pub ident:   BuilderPackageIdent,
    pub target:  PackageTarget,
    pub origin:  String,
    pub channel: ChannelIdent,
}

impl OriginChannelPackage {
    pub fn promote(package: OriginChannelPromote, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        // If this looks bad, it is. To ensure we get values here or die we have to execute queries
        // to get the IDs first. I can hear the groaning already, "Why can't we just do a
        // sub-select and let the database barf on insert" Great question - Because the
        // typechecking happens in Rust and you wanted a type-safe language
        let channel_id = origin_channels::table.filter(origin_channels::name.eq(package.channel
                                                                                       .as_str()))
                                               .filter(origin_channels::origin.eq(package.origin))
                                               .select(origin_channels::id)
                                               .limit(1)
                                               .get_result::<i64>(conn)?;
        let package_id =
            origin_packages::table.filter(origin_packages::ident.eq(package.ident.to_string()))
                                  .filter(origin_packages::target.eq(package.target.to_string()))
                                  .select(origin_packages::id)
                                  .limit(1)
                                  .get_result::<i64>(conn)?;

        diesel::insert_into(origin_channel_packages::table)
            .values((
                origin_channel_packages::channel_id.eq(channel_id),
                origin_channel_packages::package_id.eq(package_id),
            ))
            .on_conflict_do_nothing()
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
                            .filter(origin_channels::name.eq(package.channel.as_str()))
                            .filter(origin_channels::origin.eq(package.origin))
                            .single_value()),
                )
                .filter(
                    origin_channel_packages::package_id
                        .nullable()
                        .eq(origin_packages::table
                            .select(origin_packages::id)
                            .filter(origin_packages::ident.eq(package.ident.to_string()))
                            .filter(origin_packages::target.eq(package.target.to_string()))
                            .single_value()),
                ),
        )
        .execute(conn)
    }
}
