use std::{fmt,
          io::Write,
          ops::Deref,
          str::{self,
                FromStr},
          time::Instant};

use chrono::NaiveDateTime;

use diesel::{self,
             deserialize::{self,
                           FromSql},
             dsl::{count,
                   sql},
             pg::{expression::dsl::any,
                  upsert::{excluded,
                           on_constraint},
                  Pg,
                  PgConnection},
             prelude::*,
             result::QueryResult,
             serialize::{self,
                         IsNull,
                         Output,
                         ToSql},
             sql_types::Text,
             PgArrayExpressionMethods,
             RunQueryDsl};
use diesel_full_text_search::{to_tsquery,
                              TsQueryExtensions};

use super::db_id_format;
use crate::{hab_core::{self,
                       package::{FromArchive,
                                 Identifiable,
                                 PackageArchive,
                                 PackageIdent,
                                 PackageTarget},
                       ChannelIdent},
            models::{channel::{Channel,
                               OriginChannelPackage,
                               OriginChannelPromote},
                     pagination::*,
                     settings::OriginPackageSettings}};

use crate::schema::{channel::{origin_channel_packages,
                              origin_channels},
                    member::origin_members,
                    origin::origins,
                    package::{origin_package_versions,
                              origin_packages,
                              origin_packages_with_version_array,
                              packages_with_channel_platform},
                    settings::origin_package_settings};

use crate::{bldr_core::metrics::{CounterMetric,
                                 HistogramMetric},
            metrics::{Counter,
                      Histogram},
            protocol::originsrv::{OriginPackage,
                                  OriginPackageIdent,
                                  OriginPackageVisibility}};

#[derive(Debug,
         Serialize,
         Deserialize,
         QueryableByName,
         Queryable,
         Clone,
         Identifiable)]
#[table_name = "origin_packages"]
pub struct Package {
    #[serde(with = "db_id_format")]
    pub id:          i64,
    #[serde(with = "db_id_format")]
    pub owner_id:    i64,
    pub name:        String,
    pub ident:       BuilderPackageIdent,
    pub ident_array: Vec<String>,
    pub checksum:    String,
    pub manifest:    String,
    pub config:      String,
    pub target:      BuilderPackageTarget,
    pub deps:        Vec<BuilderPackageIdent>,
    pub tdeps:       Vec<BuilderPackageIdent>,
    pub build_deps:  Vec<BuilderPackageIdent>,
    pub build_tdeps: Vec<BuilderPackageIdent>,
    pub exposes:     Vec<i32>,
    pub visibility:  PackageVisibility,
    pub created_at:  Option<NaiveDateTime>,
    pub updated_at:  Option<NaiveDateTime>,
    pub origin:      String,
}

#[derive(Debug,
         Serialize,
         Deserialize,
         QueryableByName,
         Queryable,
         Clone,
         Identifiable)]
#[table_name = "origin_packages_with_version_array"]
pub struct PackageWithVersionArray {
    #[serde(with = "db_id_format")]
    pub id:            i64,
    #[serde(with = "db_id_format")]
    pub owner_id:      i64,
    pub name:          String,
    pub ident:         BuilderPackageIdent,
    pub ident_array:   Vec<String>,
    pub checksum:      String,
    pub manifest:      String,
    pub config:        String,
    pub target:        BuilderPackageTarget,
    pub deps:          Vec<BuilderPackageIdent>,
    pub tdeps:         Vec<BuilderPackageIdent>,
    pub exposes:       Vec<i32>,
    pub created_at:    Option<NaiveDateTime>,
    pub updated_at:    Option<NaiveDateTime>,
    pub visibility:    PackageVisibility,
    pub origin:        String,
    pub build_deps:    Vec<BuilderPackageIdent>,
    pub build_tdeps:   Vec<BuilderPackageIdent>,
    pub version_array: Vec<Option<String>>,
}

#[derive(Debug,
         Serialize,
         Deserialize,
         QueryableByName,
         Queryable,
         Clone,
         Identifiable,
         PartialEq)]
#[table_name = "packages_with_channel_platform"]
pub struct PackageWithChannelPlatform {
    #[serde(with = "db_id_format")]
    pub id:          i64,
    #[serde(with = "db_id_format")]
    pub owner_id:    i64,
    pub name:        String,
    pub ident:       BuilderPackageIdent,
    pub ident_array: Vec<String>,
    pub checksum:    String,
    pub manifest:    String,
    pub config:      String,
    pub target:      BuilderPackageTarget,
    pub deps:        Vec<BuilderPackageIdent>,
    pub tdeps:       Vec<BuilderPackageIdent>,
    pub build_deps:  Vec<BuilderPackageIdent>,
    pub build_tdeps: Vec<BuilderPackageIdent>,
    pub exposes:     Vec<i32>,
    pub visibility:  PackageVisibility,
    pub created_at:  Option<NaiveDateTime>,
    pub updated_at:  Option<NaiveDateTime>,
    pub origin:      String,
    pub channels:    Vec<String>,
    pub platforms:   Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackageIdentWithChannelPlatform {
    pub origin:    String,
    pub name:      String,
    pub version:   Option<String>,
    pub release:   Option<String>,
    pub channels:  Vec<String>,
    pub platforms: Vec<String>,
}

/// We literally never want to select `ident_vector`
/// so we provide this type and constant to pass to `.select`

type AllColumns = (origin_packages::id,
                   origin_packages::owner_id,
                   origin_packages::name,
                   origin_packages::ident,
                   origin_packages::ident_array,
                   origin_packages::checksum,
                   origin_packages::manifest,
                   origin_packages::config,
                   origin_packages::target,
                   origin_packages::deps,
                   origin_packages::tdeps,
                   origin_packages::build_deps,
                   origin_packages::build_tdeps,
                   origin_packages::exposes,
                   origin_packages::visibility,
                   origin_packages::created_at,
                   origin_packages::updated_at,
                   origin_packages::origin);

pub const ALL_COLUMNS: AllColumns = (origin_packages::id,
                                     origin_packages::owner_id,
                                     origin_packages::name,
                                     origin_packages::ident,
                                     origin_packages::ident_array,
                                     origin_packages::checksum,
                                     origin_packages::manifest,
                                     origin_packages::config,
                                     origin_packages::target,
                                     origin_packages::deps,
                                     origin_packages::tdeps,
                                     origin_packages::build_deps,
                                     origin_packages::build_tdeps,
                                     origin_packages::exposes,
                                     origin_packages::visibility,
                                     origin_packages::created_at,
                                     origin_packages::updated_at,
                                     origin_packages::origin);

type All = diesel::dsl::Select<origin_packages::table, AllColumns>;

type AllColumnsWithVersion = (origin_packages_with_version_array::id,
                              origin_packages_with_version_array::owner_id,
                              origin_packages_with_version_array::name,
                              origin_packages_with_version_array::ident,
                              origin_packages_with_version_array::ident_array,
                              origin_packages_with_version_array::checksum,
                              origin_packages_with_version_array::manifest,
                              origin_packages_with_version_array::config,
                              origin_packages_with_version_array::target,
                              origin_packages_with_version_array::deps,
                              origin_packages_with_version_array::tdeps,
                              origin_packages_with_version_array::exposes,
                              origin_packages_with_version_array::created_at,
                              origin_packages_with_version_array::updated_at,
                              origin_packages_with_version_array::visibility,
                              origin_packages_with_version_array::origin,
                              origin_packages_with_version_array::build_deps,
                              origin_packages_with_version_array::build_tdeps,
                              origin_packages_with_version_array::version_array);

pub const ALL_COLUMNS_WITH_VERSION: AllColumnsWithVersion =
    (origin_packages_with_version_array::id,
     origin_packages_with_version_array::owner_id,
     origin_packages_with_version_array::name,
     origin_packages_with_version_array::ident,
     origin_packages_with_version_array::ident_array,
     origin_packages_with_version_array::checksum,
     origin_packages_with_version_array::manifest,
     origin_packages_with_version_array::config,
     origin_packages_with_version_array::target,
     origin_packages_with_version_array::deps,
     origin_packages_with_version_array::tdeps,
     origin_packages_with_version_array::exposes,
     origin_packages_with_version_array::created_at,
     origin_packages_with_version_array::updated_at,
     origin_packages_with_version_array::visibility,
     origin_packages_with_version_array::origin,
     origin_packages_with_version_array::build_deps,
     origin_packages_with_version_array::build_tdeps,
     origin_packages_with_version_array::version_array);

type AllWithVersion =
    diesel::dsl::Select<origin_packages_with_version_array::table, AllColumnsWithVersion>;

#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
#[table_name = "origin_packages"]
pub struct NewPackage {
    pub origin:      String,
    #[serde(with = "db_id_format")]
    pub owner_id:    i64,
    pub name:        String,
    pub ident:       BuilderPackageIdent,
    pub ident_array: Vec<String>,
    pub checksum:    String,
    pub manifest:    String,
    pub config:      String,
    pub target:      BuilderPackageTarget,
    pub deps:        Vec<BuilderPackageIdent>,
    pub tdeps:       Vec<BuilderPackageIdent>,
    pub build_deps:  Vec<BuilderPackageIdent>,
    pub build_tdeps: Vec<BuilderPackageIdent>,
    pub exposes:     Vec<i32>,
    pub visibility:  PackageVisibility,
}

#[derive(Debug)]
pub struct GetLatestPackage {
    pub ident:      BuilderPackageIdent,
    pub target:     BuilderPackageTarget,
    pub visibility: Vec<PackageVisibility>,
}

#[derive(Debug)]
pub struct GetPackage {
    pub ident:      BuilderPackageIdent,
    pub visibility: Vec<PackageVisibility>,
    pub target:     BuilderPackageTarget,
}

#[derive(Debug)]
pub struct GetPackageGroup {
    pub pkgs:       Vec<BuilderPackageIdent>,
    pub visibility: Vec<PackageVisibility>,
}

#[derive(Debug)]
pub struct DeletePackage {
    pub ident:  BuilderPackageIdent,
    pub target: BuilderPackageTarget,
}

#[derive(Debug)]
pub struct UpdatePackageVisibility {
    pub visibility: PackageVisibility,
    pub ids:        Vec<i64>,
}

pub struct ListPackages {
    pub ident:      BuilderPackageIdent,
    pub visibility: Vec<PackageVisibility>,
    pub page:       i64,
    pub limit:      i64,
}

pub struct SearchPackages {
    pub query:      String,
    pub account_id: Option<i64>,
    pub page:       i64,
    pub limit:      i64,
}
#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct OriginPackageVersions {
    pub origin:        String,
    pub name:          String,
    pub version:       String,
    #[serde(with = "db_id_format")]
    pub release_count: i64,
    pub latest:        String,
    pub platforms:     Vec<String>,
    pub visibility:    PackageVisibility,
}

#[derive(DbEnum,
         Debug,
         Eq,
         Hash,
         Serialize,
         Deserialize,
         PartialEq,
         Clone,
         ToSql,
         FromSql)]
#[PgType = "origin_package_visibility"]
#[postgres(name = "origin_package_visibility")]
pub enum PackageVisibility {
    #[postgres(name = "public")]
    #[serde(rename = "public")]
    Public,
    #[postgres(name = "private")]
    #[serde(rename = "private")]
    Private,
    #[postgres(name = "hidden")]
    #[serde(rename = "hidden")]
    Hidden,
}

impl fmt::Display for PackageVisibility {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = match *self {
            PackageVisibility::Public => "public",
            PackageVisibility::Private => "private",
            PackageVisibility::Hidden => "hidden",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for PackageVisibility {
    type Err = ();

    fn from_str(s: &str) -> Result<PackageVisibility, ()> {
        match s {
            "public" => Ok(PackageVisibility::Public),
            "private" => Ok(PackageVisibility::Private),
            "hidden" => Ok(PackageVisibility::Hidden),
            _ => Err(()),
        }
    }
}

impl PackageVisibility {
    pub fn all() -> Vec<Self> {
        vec![PackageVisibility::Public,
             PackageVisibility::Private,
             PackageVisibility::Hidden,]
    }

    pub fn private() -> Vec<Self> { vec![PackageVisibility::Private, PackageVisibility::Hidden] }
}

impl PackageWithVersionArray {
    pub fn all() -> AllWithVersion {
        origin_packages_with_version_array::table.select(ALL_COLUMNS_WITH_VERSION)
    }
}

impl Package {
    pub fn get_without_target(ident: BuilderPackageIdent,
                              visibility: Vec<PackageVisibility>,
                              conn: &PgConnection)
                              -> QueryResult<Package> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = Self::all().filter(origin_packages::ident.eq(ident))
                                .filter(origin_packages::visibility.eq(any(visibility)))
                                .get_result(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get_without_target time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetWithoutTargetCallTime.set(duration_millis as f64);
        result
    }

    pub fn get(req: GetPackage, conn: &PgConnection) -> QueryResult<Package> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = Self::all().filter(origin_packages::ident.eq(req.ident))
                                .filter(origin_packages::visibility.eq(any(req.visibility)))
                                .filter(origin_packages::target.eq(req.target))
                                .get_result(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetCallTime.set(duration_millis as f64);
        result
    }

    pub fn delete(req: DeletePackage, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_packages::table
                .filter(origin_packages::ident.eq(req.ident))
                .filter(origin_packages::target.eq(req.target)),
        )
        .execute(conn)
    }

    pub fn get_group(req: GetPackageGroup, conn: &PgConnection) -> QueryResult<Vec<Package>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = Self::all().filter(origin_packages::ident.eq(any(req.pkgs)))
                                .filter(origin_packages::visibility.eq(any(req.visibility)))
                                .get_results(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get_group time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetGroupCallTime.set(duration_millis as f64);
        result
    }

    pub fn get_all(req_ident: &BuilderPackageIdent,
                   conn: &PgConnection)
                   -> QueryResult<Vec<Package>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result =
            Self::all().filter(origin_packages::origin.eq(&req_ident.origin))
                       .filter(origin_packages::name.eq(&req_ident.name))
                       .filter(origin_packages::ident_array.contains(req_ident.clone().parts()))
                       .get_results(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get_all time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetAllCallTime.set(duration_millis as f64);
        result
    }

    pub fn get_latest(req: GetLatestPackage,
                      conn: &PgConnection)
                      -> QueryResult<PackageWithVersionArray> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_packages_with_version_array::table
            .filter(origin_packages_with_version_array::origin.eq(&req.ident.origin.clone()))
            .filter(origin_packages_with_version_array::name.eq(&req.ident.name.clone()))
            .filter(origin_packages_with_version_array::ident_array.contains(req.ident.parts()))
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
        trace!("DBCall package::get_latest time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetLatestCallTime.set(duration_millis as f64);

        result
    }

    pub fn get_all_latest(conn: &PgConnection) -> QueryResult<Vec<PackageWithVersionArray>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();
        let result = origin_packages_with_version_array::table
            .distinct_on((
                origin_packages_with_version_array::origin,
                origin_packages_with_version_array::name,
                origin_packages_with_version_array::target,
            ))
            .order(sql::<PackageWithVersionArray>(
                "origin, name, target, string_to_array(version_array[1],'.')::\
                numeric[] desc, ident_array[4] desc",
            ))
            .get_results(conn);
        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get_all_latest time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetAllLatestCallTime.set(duration_millis as f64);
        result
    }

    pub fn create(package: &NewPackage, conn: &PgConnection) -> QueryResult<Package> {
        Counter::DBCall.increment();
        let pkg = diesel::insert_into(origin_packages::table)
            .values(package)
            .returning(ALL_COLUMNS)
            .on_conflict(on_constraint("origin_packages_ident_target_key"))
            .do_update()
            .set((
                origin_packages::origin.eq(excluded(origin_packages::origin)),
                origin_packages::owner_id.eq(excluded(origin_packages::owner_id)),
                origin_packages::name.eq(excluded(origin_packages::name)),
                origin_packages::ident.eq(excluded(origin_packages::ident)),
                origin_packages::checksum.eq(excluded(origin_packages::checksum)),
                origin_packages::manifest.eq(excluded(origin_packages::manifest)),
                origin_packages::config.eq(excluded(origin_packages::config)),
                origin_packages::target.eq(excluded(origin_packages::target)),
                origin_packages::deps.eq(excluded(origin_packages::deps)),
                origin_packages::tdeps.eq(excluded(origin_packages::tdeps)),
                origin_packages::build_deps.eq(excluded(origin_packages::build_deps)),
                origin_packages::build_tdeps.eq(excluded(origin_packages::build_tdeps)),
                origin_packages::exposes.eq(excluded(origin_packages::exposes)),
                origin_packages::visibility.eq(excluded(origin_packages::visibility)),
            ))
            .get_result::<Package>(conn)?;

        OriginChannelPackage::promote(OriginChannelPromote { ident:   package.ident.clone(),
                                                             target:  package.target.0,
                                                             origin:  package.origin.clone(),
                                                             channel: ChannelIdent::unstable(), },
                                      conn)?;
        Ok(pkg)
    }

    pub fn update_visibility(vis: PackageVisibility,
                             idt: BuilderPackageIdent,
                             conn: &PgConnection)
                             -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_packages::table.filter(origin_packages::ident.eq(idt)))
            .set(origin_packages::visibility.eq(vis))
            .execute(conn)
    }

    pub fn update_visibility_bulk(req: UpdatePackageVisibility,
                                  conn: &PgConnection)
                                  -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_packages::table.filter(origin_packages::id.eq_any(req.ids)))
            .set(origin_packages::visibility.eq(req.visibility))
            .execute(conn)
    }

    pub fn list(pl: ListPackages,
                conn: &PgConnection)
                -> QueryResult<(Vec<PackageWithChannelPlatform>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let mut query = packages_with_channel_platform::table
            .filter(packages_with_channel_platform::origin.eq(&pl.ident.origin))
            .into_boxed();
        // We need the into_boxed above to be able to conditionally filter and not break the
        // typesystem.
        if pl.ident.name != "" {
            query = query.filter(packages_with_channel_platform::name.eq(&pl.ident.name))
        };
        let query = query.filter(packages_with_channel_platform::ident_array.contains(pl.ident
                                                                                        .clone()
                                                                                        .parts()))
                         .filter(packages_with_channel_platform::visibility.eq(any(pl.visibility)))
                         .order(packages_with_channel_platform::ident.desc())
                         .paginate(pl.page)
                         .per_page(pl.limit);

        // helpful trick when debugging queries, this has Debug trait:
        // diesel::query_builder::debug_query::<diesel::pg::Pg, _>(&query)

        let (mut pkgs, _): (std::vec::Vec<PackageWithChannelPlatform>, i64) =
            query.load_and_count_records(conn)?;

        let duration_millis = start_time.elapsed().as_millis();
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageListCallTime.set(duration_millis as f64);

        // Package list for a whole origin is still not very
        // performant, and we want to track that
        if pl.ident.name != "" {
            Histogram::PackageListOriginOnlyCallTime.set(duration_millis as f64);
        } else {
            Histogram::PackageListOriginNameCallTime.set(duration_millis as f64);
        }

        trace!(target: "habitat_builder_api::server::resources::pkgs::versions", "Package::list for {:?}, returned {} items", pl.ident, pkgs.len());

        // Note: dedup here as packages_with_channel_platform can return
        // duplicate rows. TODO: Look for a performant Postgresql fix
        // and possibly rethink the channels design
        pkgs.dedup();
        trace!(target: "habitat_builder_api::server::resources::pkgs::versions", "Package::list for {:?} after de-dup has {} items", pl.ident, pkgs.len());

        let new_count = pkgs.len() as i64;
        Ok((pkgs, new_count))
    }

    pub fn list_distinct(pl: ListPackages,
                         conn: &PgConnection)
                         -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let mut query = origin_packages::table.select(sql("concat_ws('/', ident_array[1], \
                                                           ident_array[2]) as ident"))
                                              .filter(origin_packages::origin.eq(&pl.ident.origin))
                                              .into_boxed();
        // We need the into_boxed above to be able to conditionally filter and not break the
        // typesystem.
        if pl.ident.name != "" {
            query = query.filter(origin_packages::name.eq(&pl.ident.name))
        };
        let query = query
            .filter(origin_packages::ident_array.contains(pl.ident.clone().parts()))
            .filter(origin_packages::visibility.eq(any(pl.visibility)))
            // This is because diesel doesn't yet support group_by
            // see: https://github.com/diesel-rs/diesel/issues/210
            .filter(sql("TRUE GROUP BY ident_array[2], ident_array[1]"))
            .order(sql::<BuilderPackageIdent>("ident ASC"))
            .paginate(pl.page)
            .per_page(pl.limit);

        // helpful trick when debugging queries, this has Debug trait:
        // diesel::query_builder::debug_query::<diesel::pg::Pg, _>(&query)

        let result = query.load_and_count_records(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::list_distinct time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageListDistinctCallTime.set(duration_millis as f64);
        // Package list for a whole origin is still not very
        // performant, and we want to track that
        if pl.ident.name != "" {
            Histogram::PackageListDistinctOriginOnlyCallTime.set(duration_millis as f64);
        } else {
            Histogram::PackageListDistinctOriginNameCallTime.set(duration_millis as f64);
        }

        result
    }

    pub fn distinct_for_origin(pl: ListPackages,
                               conn: &PgConnection)
                               -> QueryResult<(Vec<OriginPackageSettings>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_package_settings::table
            .select(origin_package_settings::all_columns)
            .filter(origin_package_settings::origin.eq(&pl.ident.origin))
            .filter(origin_package_settings::visibility.eq(any(pl.visibility)))
            .order(origin_package_settings::origin.asc())
            .order(origin_package_settings::name.asc())
            .paginate(pl.page)
            .per_page(pl.limit)
            .load_and_count_records(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::list_distinct_for_origin time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageListDistinctForOriginCallTime.set(duration_millis as f64);
        result
    }

    pub fn list_package_channels(ident: &BuilderPackageIdent,
                                 target: PackageTarget,
                                 visibility: Vec<PackageVisibility>,
                                 conn: &PgConnection)
                                 -> QueryResult<Vec<Channel>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_packages::table
            .inner_join(origin_channel_packages::table.inner_join(origin_channels::table))
            .select(origin_channels::table::all_columns())
            .filter(origin_packages::ident.eq(ident))
            .filter(origin_packages::target.eq(target.to_string()))
            .filter(origin_packages::visibility.eq(any(visibility)))
            .order(origin_channels::name.desc())
            .get_results(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::list_package_channels time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageListPackageChannelsCallTime.set(duration_millis as f64);
        result
    }

    pub fn list_package_versions(ident: &BuilderPackageIdent,
                                 visibility: Vec<PackageVisibility>,
                                 conn: &PgConnection)
                                 -> QueryResult<Vec<OriginPackageVersions>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_package_versions::table
            .filter(origin_package_versions::origin.eq(ident.origin()))
            .filter(origin_package_versions::name.eq(ident.name()))
            .filter(origin_package_versions::visibility.eq(any(visibility)))
            .order(sql::<OriginPackageVersions>(
                "string_to_array(version_array[1],'.')::numeric[]desc, version_array[2] desc",
            ))
            .get_results(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::list_package_versions time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageListPackageVersionsCallTime.set(duration_millis as f64);
        result
    }

    pub fn count_origin_packages(origin: &str, conn: &PgConnection) -> QueryResult<i64> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_packages::table.select(count(origin_packages::id))
                                           .filter(origin_packages::origin.eq(&origin))
                                           .first(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::count_origin_packages time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageCountOriginPackages.set(duration_millis as f64);
        result
    }

    pub fn search(sp: SearchPackages,
                  conn: &PgConnection)
                  -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let mut query = origin_packages::table
            .select(origin_packages::ident)
            .filter(to_tsquery(sp.query).matches(origin_packages::ident_vector))
            .order(origin_packages::ident.asc())
            .into_boxed();

        if let Some(session_id) = sp.account_id {
            let origins = origin_members::table.select(origin_members::origin)
                                               .filter(origin_members::account_id.eq(session_id));
            query = query.filter(
                origin_packages::visibility
                    .eq(any(PackageVisibility::private()))
                    .and(origin_packages::origin.eq_any(origins))
                    .or(origin_packages::visibility.eq(PackageVisibility::Public)),
            );
        } else {
            query = query.filter(origin_packages::visibility.eq(PackageVisibility::Public));
        }

        let result = query.paginate(sp.page)
                          .per_page(sp.limit)
                          .load_and_count_records(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::search time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageSearchCallTime.set(duration_millis as f64);
        result
    }

    // This is me giving up on fighting the typechecker and just duplicating a bunch of code
    pub fn search_distinct(sp: SearchPackages,
                           conn: &PgConnection)
                           -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let mut query = origin_packages::table
            .inner_join(origins::table)
            .select(sql("concat_ws('/', origins.name, origin_packages.name)"))
            .filter(to_tsquery(sp.query).matches(origin_packages::ident_vector))
            .order(origin_packages::name.asc())
            .into_boxed();

        if let Some(session_id) = sp.account_id {
            query = query.filter(
                origin_packages::visibility
                    .eq(any(PackageVisibility::private()))
                    .and(origins::owner_id.eq(session_id))
                    .or(origin_packages::visibility.eq(PackageVisibility::Public)),
            );
        } else {
            query = query.filter(origin_packages::visibility.eq(PackageVisibility::Public));
        }

        // Because of the filter hack it is very important that this be the last filter
        query = query.filter(sql("TRUE GROUP BY origin_packages.name, origins.name"));

        let result = query.paginate(sp.page)
                          .per_page(sp.limit)
                          .load_and_count_records(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::search time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageSearchDistinctCallTime.set(duration_millis as f64);
        result
    }

    pub fn all() -> All { origin_packages::table.select(ALL_COLUMNS) }

    pub fn list_package_platforms(ident: &BuilderPackageIdent,
                                  visibilities: Vec<PackageVisibility>,
                                  conn: &PgConnection)
                                  -> QueryResult<Vec<BuilderPackageTarget>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_packages::table
            .select(origin_packages::target)
            .filter(origin_packages::origin.eq(&ident.origin))
            .filter(origin_packages::name.eq(&ident.name))
            .filter(origin_packages::ident_array.contains(&searchable_ident(&ident)))
            .filter(origin_packages::visibility.eq(any(visibilities)))
            .get_results(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::list_package_platforms time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageListPackagePlatformsCallTime.set(duration_millis as f64);
        result
    }

    pub fn is_a_service(&self) -> bool {
        // TODO: This is a temporary workaround until we plumb in a better solution for
        // determining whether a package is a service from the DB instead of needing
        // to crack the archive file to look for a SVC_USER file
        self.manifest.contains("pkg_exposes")
        || self.manifest.contains("pkg_binds")
        || self.manifest.contains("pkg_exports")
    }
}

impl PackageWithChannelPlatform {
    pub fn is_a_service(&self) -> bool {
        // TODO: This is a temporary workaround until we plumb in a better solution for
        // determining whether a package is a service from the DB instead of needing
        // to crack the archive file to look for a SVC_USER file
        self.manifest.contains("pkg_exposes")
        || self.manifest.contains("pkg_binds")
        || self.manifest.contains("pkg_exports")
    }
}

fn searchable_ident(ident: &BuilderPackageIdent) -> Vec<String> {
    // https://github.com/rust-lang/rust-clippy/issues/3071U
    #[allow(clippy::redundant_closure)]
    ident.to_string()
         .split('/')
         .map(|s| s.to_string())
         .filter(|s| s != "")
         .collect()
}

#[derive(Debug,
         Serialize,
         Deserialize,
         Clone,
         FromSqlRow,
         AsExpression,
         PartialEq)]
#[sql_type = "Text"]
pub struct BuilderPackageIdent(pub PackageIdent);

impl FromStr for BuilderPackageIdent {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, crate::error::Error> {
        Ok(BuilderPackageIdent(PackageIdent::from_str(s).map_err(|_| {
                                   crate::error::Error::ParseError(format!("BuilderPackageIdent \
                                                                            {}",
                                                                           s))
                               })?))
    }
}

impl FromSql<Text, Pg> for BuilderPackageIdent {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match bytes {
            Some(text) => Ok(BuilderPackageIdent(
                PackageIdent::from_str(str::from_utf8(text).unwrap()).unwrap(),
            )),
            None => Ok(BuilderPackageIdent(PackageIdent::default())),
        }
    }
}

impl ToSql<Text, Pg> for BuilderPackageIdent {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_all(self.to_string().as_bytes())
           .map(|_| IsNull::No)
           .map_err(Into::into)
    }
}

impl BuilderPackageIdent {
    pub fn parts(self) -> Vec<String> {
        #[allow(clippy::redundant_closure)]
        self.to_string()
            .split('/')
            .map(|s| s.to_string())
            // We must filter out empty strings from the vec.
            // This sometimes happens hen the origin or the package name are undefined.
            .filter(|s| s != "")
            .collect()
    }
}

impl Into<PackageIdent> for BuilderPackageIdent {
    fn into(self) -> PackageIdent { self.0 }
}

impl Deref for BuilderPackageIdent {
    type Target = PackageIdent;

    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug,
         Serialize,
         Deserialize,
         Clone,
         FromSqlRow,
         AsExpression,
         PartialEq,
         Hash,
         Eq,
         Copy)]
#[sql_type = "Text"]
pub struct BuilderPackageTarget(pub PackageTarget);

impl FromSql<Text, Pg> for BuilderPackageTarget {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match bytes {
            Some(text) => Ok(BuilderPackageTarget(
                PackageTarget::from_str(str::from_utf8(text).unwrap()).unwrap(),
            )),
            None => Ok(BuilderPackageTarget(
                PackageTarget::from_str("x86_64").unwrap(),
            )),
        }
    }
}

impl ToSql<Text, Pg> for BuilderPackageTarget {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_all(self.to_string().as_bytes())
           .map(|_| IsNull::No)
           .map_err(Into::into)
    }
}

impl Deref for BuilderPackageTarget {
    type Target = PackageTarget;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl FromArchive for NewPackage {
    type Error = hab_core::Error;

    fn from_archive(archive: &mut PackageArchive) -> hab_core::Result<Self> {
        let ident = match archive.ident() {
            Ok(value) => BuilderPackageIdent(value),
            Err(e) => return Err(e),
        };

        let config = match archive.config() {
            Some(config) => config.to_string(),
            None => String::from(""),
        };

        let exposes = archive.exposes()?
                             .into_iter()
                             .map(i32::from)
                             .collect::<Vec<i32>>();

        let deps = archive.deps()?
                          .into_iter()
                          .map(BuilderPackageIdent)
                          .collect::<Vec<BuilderPackageIdent>>();

        let tdeps = archive.tdeps()?
                           .into_iter()
                           .map(BuilderPackageIdent)
                           .collect::<Vec<BuilderPackageIdent>>();

        let build_deps = archive.build_deps()?
                                .into_iter()
                                .map(BuilderPackageIdent)
                                .collect::<Vec<BuilderPackageIdent>>();

        let build_tdeps = archive.build_tdeps()?
                                 .into_iter()
                                 .map(BuilderPackageIdent)
                                 .collect::<Vec<BuilderPackageIdent>>();

        // Some of the values here are made up because they are required in the db but not
        // necessarially requred for a valid package
        Ok(NewPackage { ident: ident.clone(),
                        ident_array: ident.clone().parts(),
                        origin: ident.origin().to_string(),
                        manifest: archive.manifest()?.to_string(),
                        target: BuilderPackageTarget(archive.target()?),
                        deps,
                        tdeps,
                        build_deps,
                        build_tdeps,
                        exposes,
                        config,
                        checksum: archive.checksum()?,
                        name: ident.name.to_string(),
                        owner_id: 999_999_999_999,
                        visibility: PackageVisibility::Public })
    }
}

// TED TODO: PROTOCLEANUP Remove everything below when the protos are gone
impl From<OriginPackageVisibility> for PackageVisibility {
    fn from(value: OriginPackageVisibility) -> PackageVisibility {
        match value {
            OriginPackageVisibility::Hidden => PackageVisibility::Hidden,
            OriginPackageVisibility::Private => PackageVisibility::Private,
            _ => PackageVisibility::Public,
        }
    }
}

impl Into<OriginPackageVisibility> for PackageVisibility {
    fn into(self) -> OriginPackageVisibility {
        match self {
            PackageVisibility::Hidden => OriginPackageVisibility::Hidden,
            PackageVisibility::Private => OriginPackageVisibility::Private,
            _ => OriginPackageVisibility::Public,
        }
    }
}

impl Into<OriginPackage> for Package {
    fn into(self) -> OriginPackage {
        let exposes = self.exposes
                          .into_iter()
                          .map(|e| e as u32)
                          .collect::<Vec<u32>>();

        let mut op = OriginPackage::new();
        let ident = &*self.ident;
        op.set_id(self.id as u64);
        op.set_ident(OriginPackageIdent::from(ident.clone()));
        op.set_manifest(self.manifest);
        op.set_target(self.target.to_string());
        op.set_deps(into_idents(self.deps));
        op.set_tdeps(into_idents(self.tdeps));
        op.set_build_deps(into_idents(self.build_deps));
        op.set_build_tdeps(into_idents(self.build_tdeps));
        op.set_exposes(exposes);
        op.set_config(self.config);
        op.set_checksum(self.checksum);
        op.set_owner_id(self.owner_id as u64);
        op.set_visibility(self.visibility.into());
        op
    }
}

impl Into<OriginPackage> for PackageWithVersionArray {
    fn into(self) -> OriginPackage {
        let exposes = self.exposes
                          .into_iter()
                          .map(|e| e as u32)
                          .collect::<Vec<u32>>();

        let mut op = OriginPackage::new();
        let ident = &*self.ident;
        op.set_id(self.id as u64);
        op.set_ident(OriginPackageIdent::from(ident.clone()));
        op.set_manifest(self.manifest);
        op.set_target(self.target.to_string());
        op.set_deps(into_idents(self.deps));
        op.set_tdeps(into_idents(self.tdeps));
        op.set_build_deps(into_idents(self.build_deps));
        op.set_build_tdeps(into_idents(self.build_tdeps));
        op.set_exposes(exposes);
        op.set_config(self.config);
        op.set_checksum(self.checksum);
        op.set_owner_id(self.owner_id as u64);
        op.set_visibility(self.visibility.into());
        op
    }
}

impl Into<Package> for PackageWithVersionArray {
    fn into(self) -> Package {
        Package { id:          self.id,
                  owner_id:    self.owner_id,
                  name:        self.name.clone(),
                  ident:       self.ident.clone(),
                  ident_array: self.ident_array.clone(),
                  checksum:    self.checksum.clone(),
                  manifest:    self.manifest.clone(),
                  config:      self.config.clone(),
                  target:      self.target,
                  deps:        self.deps.clone(),
                  tdeps:       self.tdeps.clone(),
                  build_deps:  self.build_deps.clone(),
                  build_tdeps: self.build_tdeps.clone(),
                  exposes:     self.exposes.clone(),
                  visibility:  self.visibility,
                  created_at:  self.created_at,
                  updated_at:  self.updated_at,
                  origin:      self.origin, }
    }
}

impl Into<OriginPackageIdent> for BuilderPackageIdent {
    fn into(self) -> OriginPackageIdent { self.0.into() }
}

fn into_idents(column: Vec<BuilderPackageIdent>) -> protobuf::RepeatedField<OriginPackageIdent> {
    let mut idents = protobuf::RepeatedField::new();
    for ident in column {
        idents.push(ident.into());
    }
    idents
}

impl Into<PackageIdentWithChannelPlatform> for PackageWithChannelPlatform {
    fn into(self) -> PackageIdentWithChannelPlatform {
        let mut platforms = self.platforms.clone();
        platforms.dedup();

        PackageIdentWithChannelPlatform { origin: self.ident.origin.clone(),
                                          name: self.ident.name.clone(),
                                          version: self.ident.version.clone(),
                                          release: self.ident.release.clone(),
                                          channels: self.channels,
                                          platforms }
    }
}

impl Into<PackageIdentWithChannelPlatform> for BuilderPackageIdent {
    fn into(self) -> PackageIdentWithChannelPlatform {
        PackageIdentWithChannelPlatform { origin:    self.origin.clone(),
                                          name:      self.name.clone(),
                                          version:   self.version.clone(),
                                          release:   self.release.clone(),
                                          channels:  Vec::new(),
                                          platforms: Vec::new(), }
    }
}
