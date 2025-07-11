use std::{fmt::{self,
                Debug},
          hash::{Hash,
                 Hasher},
          io::Write,
          ops::Deref,
          str::{self,
                FromStr},
          time::Instant};

use super::db_id_format;
use crate::{hab_core::{self,
                       package::{metadata::PackageType,
                                 FromArchive,
                                 Identifiable,
                                 PackageArchive,
                                 PackageIdent,
                                 PackageTarget},
                       ChannelIdent},
            models::{channel::{Channel,
                               OriginChannelPackage,
                               OriginChannelPromote},
                     settings::OriginPackageSettings},
            schema::{channel::{origin_channel_packages,
                               origin_channels},
                     member::origin_members,
                     origin::origins,
                     package::{origin_package_versions,
                               origin_packages,
                               origin_packages_with_version_array,
                               packages_with_channel_platform},
                     settings::origin_package_settings}};
use chrono::NaiveDateTime;
use diesel::{self,
             deserialize::{self,
                           FromSql},
             dsl::{count,
                   count_star,
                   sql},
             pg::{upsert::{excluded,
                           on_constraint},
                  Pg,
                  PgConnection,
                  PgValue},
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
use itertools::Itertools;

use crate::{bldr_core::metrics::{CounterMetric,
                                 HistogramMetric},
            // error::Error as CrateError,
            metrics::{Counter,
                      Histogram},
            protocol::originsrv::{OriginPackage,
                                  OriginPackageIdent,
                                  OriginPackageVisibility}};
use diesel_derive_enum::DbEnum;

#[derive(Debug,
         Serialize,
         Deserialize,
         QueryableByName,
         Queryable,
         Clone,
         Identifiable)]
#[diesel(table_name = origin_packages)]
pub struct Package {
    #[serde(with = "db_id_format")]
    pub id:           i64,
    #[serde(with = "db_id_format")]
    pub owner_id:     i64,
    pub name:         String,
    pub ident:        BuilderPackageIdent,
    pub ident_array:  Vec<String>,
    pub checksum:     String,
    pub manifest:     String,
    pub config:       String,
    pub target:       BuilderPackageTarget,
    pub deps:         Vec<BuilderPackageIdent>,
    pub tdeps:        Vec<BuilderPackageIdent>,
    pub build_deps:   Vec<BuilderPackageIdent>,
    pub build_tdeps:  Vec<BuilderPackageIdent>,
    pub exposes:      Vec<i32>,
    pub visibility:   PackageVisibility,
    pub created_at:   Option<NaiveDateTime>,
    pub updated_at:   Option<NaiveDateTime>,
    pub origin:       String,
    pub package_type: BuilderPackageType,
}

#[derive(Debug,
         Serialize,
         Deserialize,
         QueryableByName,
         Queryable,
         Clone,
         Identifiable)]
#[diesel(table_name = origin_packages_with_version_array)]
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
    pub package_type:  BuilderPackageType,
}

#[derive(Debug,
         Serialize,
         Deserialize,
         QueryableByName,
         Queryable,
         Clone,
         Identifiable,
         Eq)]
#[diesel(table_name = packages_with_channel_platform)]
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

impl Hash for PackageWithChannelPlatform {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.ident.hash(state);
        self.visibility.hash(state);
        self.origin.hash(state);
        self.channels.hash(state);
        self.platforms.hash(state);
    }
}

impl PartialEq for PackageWithChannelPlatform {
    fn eq(&self, other: &PackageWithChannelPlatform) -> bool {
        self.name == other.name
        && self.ident == other.ident
        && self.visibility == other.visibility
        && self.origin == other.origin
        && self.channels == other.channels
        && self.platforms == other.platforms
    }
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
                   origin_packages::origin,
                   origin_packages::package_type);

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
                                     origin_packages::origin,
                                     origin_packages::package_type);

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
                              origin_packages_with_version_array::version_array,
                              origin_packages_with_version_array::package_type);

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
     origin_packages_with_version_array::version_array,
     origin_packages_with_version_array::package_type);

type AllWithVersion =
    diesel::dsl::Select<origin_packages_with_version_array::table, AllColumnsWithVersion>;

#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
#[diesel(table_name = origin_packages)]
pub struct NewPackage {
    pub origin:       String,
    #[serde(with = "db_id_format")]
    pub owner_id:     i64,
    pub name:         String,
    pub ident:        BuilderPackageIdent,
    pub ident_array:  Vec<String>,
    pub checksum:     String,
    pub manifest:     String,
    pub config:       String,
    pub target:       BuilderPackageTarget,
    pub deps:         Vec<BuilderPackageIdent>,
    pub tdeps:        Vec<BuilderPackageIdent>,
    pub build_deps:   Vec<BuilderPackageIdent>,
    pub build_tdeps:  Vec<BuilderPackageIdent>,
    pub exposes:      Vec<i32>,
    pub visibility:   PackageVisibility,
    pub package_type: BuilderPackageType,
    pub hidden:       bool,
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

#[derive(DbEnum, Debug, Eq, Hash, Serialize, Deserialize, PartialEq, Clone)]
#[ExistingTypePath = "crate::schema::sql_types::OriginPackageVisibility"]
#[DbValueStyle = "snake_case"]
#[serde(rename_all = "snake_case")]
pub enum PackageVisibility {
    Public,
    Private,
    Hidden,
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
                              conn: &mut PgConnection)
                              -> QueryResult<Package> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = Self::all().filter(origin_packages::ident.eq(ident))
                                .filter(origin_packages::visibility.eq_any(visibility))
                                .filter(origin_packages::hidden.eq(false))
                                .get_result(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get_without_target time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetWithoutTargetCallTime.set(duration_millis as f64);
        result
    }

    pub fn get(req: GetPackage, conn: &mut PgConnection) -> QueryResult<Package> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = Self::all().filter(origin_packages::ident.eq(req.ident))
                                .filter(origin_packages::visibility.eq_any(req.visibility))
                                .filter(origin_packages::target.eq(req.target))
                                .filter(origin_packages::hidden.eq(false))
                                .get_result(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetCallTime.set(duration_millis as f64);
        result
    }

    pub fn delete(req: DeletePackage, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_packages::table
                .filter(origin_packages::ident.eq(req.ident))
                .filter(origin_packages::target.eq(req.target)),
        )
        .execute(conn)
    }

    pub fn get_group(req: GetPackageGroup, conn: &mut PgConnection) -> QueryResult<Vec<Package>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = Self::all().filter(origin_packages::ident.eq_any(req.pkgs))
                                .filter(origin_packages::visibility.eq_any(req.visibility))
                                .filter(origin_packages::hidden.eq(false))
                                .get_results(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get_group time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetGroupCallTime.set(duration_millis as f64);
        result
    }

    pub fn get_all(req_ident: &BuilderPackageIdent,
                   conn: &mut PgConnection)
                   -> QueryResult<Vec<Package>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result =
            Self::all().filter(origin_packages::origin.eq(&req_ident.origin))
                       .filter(origin_packages::name.eq(&req_ident.name))
                       .filter(origin_packages::ident_array.contains(req_ident.clone().parts()))
                       .filter(origin_packages::hidden.eq(false))
                       .get_results(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get_all time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetAllCallTime.set(duration_millis as f64);
        result
    }

    pub fn get_latest(req: GetLatestPackage,
                      conn: &mut PgConnection)
                      -> QueryResult<PackageWithVersionArray> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_packages_with_version_array::table
            .filter(origin_packages_with_version_array::origin.eq(&req.ident.origin.clone()))
            .filter(origin_packages_with_version_array::name.eq(&req.ident.name.clone()))
            .filter(origin_packages_with_version_array::ident_array.contains(req.ident.parts()))
            .filter(origin_packages_with_version_array::target.eq(req.target))
            .filter(origin_packages_with_version_array::visibility.eq_any(req.visibility))
            .order(sql::<Text>(
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

    pub fn get_all_latest(conn: &mut PgConnection) -> QueryResult<Vec<PackageWithVersionArray>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();
        let result = origin_packages_with_version_array::table
            .distinct_on((
                origin_packages_with_version_array::origin,
                origin_packages_with_version_array::name,
                origin_packages_with_version_array::target,
            ))
            .order((
                origin_packages_with_version_array::origin,
                origin_packages_with_version_array::name,
                origin_packages_with_version_array::target,
                sql::<Text>(
                "origin, name, target, string_to_array(version_array[1],'.')::\
                numeric[] desc, ident_array[4] desc",
                ),
            ))
            .get_results(conn);
        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::get_all_latest time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageGetAllLatestCallTime.set(duration_millis as f64);
        result
    }

    pub fn create(package: &NewPackage, conn: &mut PgConnection) -> QueryResult<Package> {
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
                origin_packages::package_type.eq(excluded(origin_packages::package_type)),
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
                             conn: &mut PgConnection)
                             -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_packages::table.filter(origin_packages::ident.eq(idt)))
            .set(origin_packages::visibility.eq(vis))
            .execute(conn)
    }

    pub fn update_visibility_bulk(req: UpdatePackageVisibility,
                                  conn: &mut PgConnection)
                                  -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_packages::table.filter(origin_packages::id.eq_any(req.ids)))
            .set(origin_packages::visibility.eq(req.visibility))
            .execute(conn)
    }

    pub fn list(pl: &ListPackages,
                conn: &mut PgConnection)
                -> QueryResult<(Vec<PackageWithChannelPlatform>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        // Extract cloned copies out of pl
        let origin_str = pl.ident.origin.clone();
        let name_str = pl.ident.name.clone();
        let parts = pl.ident.clone().parts();
        let visibility = pl.visibility.clone();
        let page = pl.page;
        let limit = pl.limit;

        let mut query = packages_with_channel_platform::table
        .filter(packages_with_channel_platform::origin.eq(origin_str))
        .into_boxed();
        // We need the into_boxed above to be able to conditionally filter and not break the
        // typesystem.
        if !pl.ident.name.is_empty() {
            query = query.filter(packages_with_channel_platform::name.eq(name_str))
        };

        let mut pkgs = if pl.limit < 0 {
            let query =
                query.filter(packages_with_channel_platform::ident_array.contains(parts.clone()))
                     .filter(packages_with_channel_platform::visibility.eq_any(visibility.clone()))
                     .order(packages_with_channel_platform::ident.desc());
            let pkgs: std::vec::Vec<PackageWithChannelPlatform> = query.get_results(conn)?;
            pkgs
        } else {
            let all_rows: Vec<PackageWithChannelPlatform> =
                query.filter(packages_with_channel_platform::ident_array.contains(parts.clone()))
                     .filter(packages_with_channel_platform::visibility.eq_any(visibility.clone()))
                     .order(packages_with_channel_platform::ident.desc())
                     .load(conn)?;
            let unique_rows: Vec<PackageWithChannelPlatform> =
                all_rows.into_iter().unique().collect();
            let start = ((page.saturating_sub(1)) * limit) as usize;
            let end = (start + limit as usize).min(unique_rows.len());
            unique_rows[start..end].to_vec()
        };

        // helpful trick when debugging queries, this has Debug trait:
        // diesel::query_builder::debug_query::<diesel::pg::Pg, _>(&query)

        let duration_millis = start_time.elapsed().as_millis();
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageListCallTime.set(duration_millis as f64);

        // Package list for a whole origin is still not very
        // performant, and we want to track that
        if !pl.ident.name.is_empty() {
            Histogram::PackageListOriginOnlyCallTime.set(duration_millis as f64);
        } else {
            Histogram::PackageListOriginNameCallTime.set(duration_millis as f64);
        }

        trace!(target: "habitat_builder_api::server::resources::pkgs::versions", "Package::list for {:?}, returned {} items", pl.ident, pkgs.len());

        // TODO: Look for a performant Postgresql fix
        // and possibly rethink the channels design
        pkgs = pkgs.into_iter().unique().collect();
        trace!(target: "habitat_builder_api::server::resources::pkgs::versions", "Package::list for {:?} after de-dup has {} items", pl.ident, pkgs.len());

        let new_count = pkgs.len() as i64;
        Ok((pkgs, new_count))
    }

    pub fn list_distinct(pl: &ListPackages,
                         conn: &mut PgConnection)
                         -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        // Extract cloned copies out of pl
        let origin_str = pl.ident.origin.clone();
        let name_str = pl.ident.name.clone();
        let parts = pl.ident.clone().parts();
        let visibility = pl.visibility.clone();
        let page = pl.page;
        let limit = pl.limit;

        let mut count_query =
            origin_packages::table.filter(origin_packages::origin.eq(origin_str.clone()))
                                  .into_boxed();
        // We need the into_boxed above to be able to conditionally filter and not break the
        // typesystem.
        if !name_str.is_empty() {
            count_query = count_query.filter(origin_packages::name.eq(name_str.clone()));
        }
        let total_count: i64 =
            count_query.filter(origin_packages::ident_array.contains(parts.clone()))
                       .filter(origin_packages::visibility.eq_any(visibility.clone()))
                       .filter(origin_packages::hidden.eq(false))
                       .select(sql::<diesel::sql_types::BigInt>("COUNT(DISTINCT \
                                                                 (origin_packages.origin, \
                                                                 origin_packages.name))"))
                       .first(conn)?;

        let mut page_query =
            origin_packages::table.filter(origin_packages::origin.eq(origin_str.clone()))
                                  .filter(origin_packages::ident_array.contains(parts))
                                  .filter(origin_packages::visibility.eq_any(visibility))
                                  .filter(origin_packages::hidden.eq(false))
                                  .select((origin_packages::origin, origin_packages::name))
                                  .distinct()
                                  .order((origin_packages::name.asc(),
                                          origin_packages::origin.asc()))
                                  .into_boxed();
        if !pl.ident.name.is_empty() {
            page_query = page_query.filter(origin_packages::name.eq(pl.ident.name.clone()));
        }

        let limit_i64 = limit;
        let offset_i64 = (page.saturating_sub(1)) * limit;

        let rows: Vec<(String, String)> =
            page_query.limit(limit_i64).offset(offset_i64).load(conn)?;

        let pkgs: Vec<BuilderPackageIdent> = rows.into_iter()
                                                 .map(|(origin, name)| {
                                                     BuilderPackageIdent(PackageIdent { origin,
                                                       name,
                                                       version: None,
                                                       release: None })
                                                 })
                                                 .collect();

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::list_distinct time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageListDistinctCallTime.set(duration_millis as f64);
        // Package list for a whole origin is still not very
        // performant, and we want to track that
        if !pl.ident.name.is_empty() {
            Histogram::PackageListDistinctOriginOnlyCallTime.set(duration_millis as f64);
        } else {
            Histogram::PackageListDistinctOriginNameCallTime.set(duration_millis as f64);
        }

        Ok((pkgs, total_count))
    }

    pub fn distinct_for_origin(pl: &ListPackages,
                               conn: &mut PgConnection)
                               -> QueryResult<(Vec<OriginPackageSettings>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        // Extract cloned copies out of pl
        let origin_str = pl.ident.origin.clone();
        let visibility = pl.visibility.clone();
        let page = pl.page;
        let limit = pl.limit;

        let  base_query = origin_package_settings::table
            .filter(origin_package_settings::origin.eq(origin_str))
            .filter(origin_package_settings::visibility.eq_any(visibility))
            .filter(origin_package_settings::hidden.eq(false))
            .order(origin_package_settings::origin.asc())
            .order(origin_package_settings::name.asc())
            .into_boxed();

        // helpful trick when debugging queries, this has Debug trait:
        // diesel::query_builder::debug_query::<diesel::pg::Pg, _>(&base_query)

        let all_rows: Vec<OriginPackageSettings> = base_query.load(conn)?;
        let unique_by_name: Vec<OriginPackageSettings> = all_rows.into_iter()
                                                                 .unique_by(|pkg| pkg.name.clone())
                                                                 .collect();

        let total_count = unique_by_name.len() as i64;
        let start = ((page.saturating_sub(1)) * limit) as usize;
        let end = (start + limit as usize).min(unique_by_name.len());
        let results = if limit < 0 {
            unique_by_name.clone()
        } else if start >= unique_by_name.len() {
            Vec::new()
        } else {
            unique_by_name[start..end].to_vec()
        };

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::list_distinct_for_origin time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageListDistinctForOriginCallTime.set(duration_millis as f64);
        Ok((results, total_count))
    }

    pub fn list_package_channels(ident: &BuilderPackageIdent,
                                 target: PackageTarget,
                                 visibility: Vec<PackageVisibility>,
                                 conn: &mut PgConnection)
                                 -> QueryResult<Vec<Channel>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_packages::table
            .inner_join(origin_channel_packages::table.inner_join(origin_channels::table))
            .select((
                origin_channels::id,
                origin_channels::owner_id,
                origin_channels::name,
                origin_channel_packages::created_at,
                origin_channel_packages::updated_at,
                origin_channels::origin,
            ))
            .filter(origin_packages::ident.eq(ident))
            .filter(origin_packages::target.eq(target.to_string()))
            .filter(origin_packages::visibility.eq_any(visibility))
            .filter(origin_packages::hidden.eq(false))
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
                                 conn: &mut PgConnection)
                                 -> QueryResult<Vec<OriginPackageVersions>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_package_versions::table
            .filter(origin_package_versions::origin.eq(ident.origin()))
            .filter(origin_package_versions::name.eq(ident.name()))
            .filter(origin_package_versions::visibility.eq_any(visibility))
            .order(sql::<Text>(
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

    pub fn count_origin_packages(origin: &str, conn: &mut PgConnection) -> QueryResult<i64> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_packages::table.select(count(origin_packages::id))
                                           .filter(origin_packages::origin.eq(&origin))
                                           .filter(origin_packages::hidden.eq(false))
                                           .first(conn);

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::count_origin_packages time: {} ms",
               duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageCountOriginPackages.set(duration_millis as f64);
        result
    }

    pub fn search(sp: &SearchPackages,
                  conn: &mut PgConnection)
                  -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let mut count_query = origin_packages::table.into_boxed();
        count_query = count_query
        .filter(to_tsquery(format!("{}:*", sp.query)).matches(origin_packages::ident_vector))
        .filter(origin_packages::hidden.eq(false));

        if let Some(session_id) = sp.account_id {
            let owned_origins =
                origin_members::table.select(origin_members::origin)
                                     .filter(origin_members::account_id.eq(session_id));

            count_query = count_query.filter(
            origin_packages::visibility
                .eq_any(PackageVisibility::private())
                .and(origin_packages::origin.eq_any(owned_origins))
                .or(origin_packages::visibility.eq(PackageVisibility::Public)),
        );
        } else {
            count_query =
                count_query.filter(origin_packages::visibility.eq(PackageVisibility::Public));
        }

        let total_count: i64 = count_query.select(count_star()).first(conn)?;

        let mut page_query = origin_packages::table.into_boxed();
        page_query = page_query
        .filter(to_tsquery(format!("{}:*", sp.query)).matches(origin_packages::ident_vector))
        .filter(origin_packages::hidden.eq(false));

        if let Some(session_id) = sp.account_id {
            let owned_origins =
                origin_members::table.select(origin_members::origin)
                                     .filter(origin_members::account_id.eq(session_id));

            page_query = page_query.filter(
            origin_packages::visibility
                .eq_any(PackageVisibility::private())
                .and(origin_packages::origin.eq_any(owned_origins))
                .or(origin_packages::visibility.eq(PackageVisibility::Public)),
        );
        } else {
            page_query =
                page_query.filter(origin_packages::visibility.eq(PackageVisibility::Public));
        }

        let limit = sp.limit;
        let offset = (sp.page.saturating_sub(1)) * sp.limit;

        let packages: Vec<BuilderPackageIdent> = page_query.select(origin_packages::ident)
                                                           .order(origin_packages::ident.asc())
                                                           .limit(limit)
                                                           .offset(offset)
                                                           .load(conn)?;

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::search time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageSearchCallTime.set(duration_millis as f64);

        Ok((packages, total_count))
    }

    // This is me giving up on fighting the typechecker and just duplicating a bunch of code
    pub fn search_distinct(sp: &SearchPackages,
                           conn: &mut PgConnection)
                           -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let mut count_query = origin_packages::table
        .inner_join(origins::table)
        .filter(to_tsquery(format!("{}:*", sp.query)).matches(origin_packages::ident_vector))
        .filter(origin_packages::hidden.eq(false))
        .into_boxed();

        if let Some(session_id) = sp.account_id {
            count_query = count_query.filter(
            origin_packages::visibility
                .eq_any(PackageVisibility::private())
                .and(origins::owner_id.eq(session_id))
                .or(origin_packages::visibility.eq(PackageVisibility::Public)),
        );
        } else {
            count_query =
                count_query.filter(origin_packages::visibility.eq(PackageVisibility::Public));
        }

        let total_count: i64 =
            count_query.select(sql::<diesel::sql_types::BigInt>("COUNT(DISTINCT concat_ws('/', \
                                                                 origins.name, \
                                                                 origin_packages.name))"))
                       .first(conn)?;

        let mut page_query = origin_packages::table
        .inner_join(origins::table)
        .select(sql::<diesel::sql_types::Text>(
            "concat_ws('/', origins.name, origin_packages.name)",
        ))
        .distinct_on((origin_packages::name, origins::name))
        .order((origin_packages::name.asc(), origins::name.asc()))
        .filter(to_tsquery(format!("{}:*", sp.query)).matches(origin_packages::ident_vector))
        .filter(origin_packages::hidden.eq(false))
        .into_boxed();

        if let Some(session_id) = sp.account_id {
            page_query = page_query.filter(
            origin_packages::visibility
                .eq_any(PackageVisibility::private())
                .and(origins::owner_id.eq(session_id))
                .or(origin_packages::visibility.eq(PackageVisibility::Public)),
        );
        } else {
            page_query =
                page_query.filter(origin_packages::visibility.eq(PackageVisibility::Public));
        }

        let limit = sp.limit;
        let offset = (sp.page.saturating_sub(1)) * sp.limit;

        let packages: Vec<BuilderPackageIdent> =
            page_query.limit(limit).offset(offset).load(conn)?;

        let duration_millis = start_time.elapsed().as_millis();
        trace!("DBCall package::search time: {} ms", duration_millis);
        Histogram::DbCallTime.set(duration_millis as f64);
        Histogram::PackageSearchDistinctCallTime.set(duration_millis as f64);
        Ok((packages, total_count))
    }

    pub fn all() -> All { origin_packages::table.select(ALL_COLUMNS) }

    pub fn list_package_platforms(ident: &BuilderPackageIdent,
                                  visibilities: Vec<PackageVisibility>,
                                  conn: &mut PgConnection)
                                  -> QueryResult<Vec<BuilderPackageTarget>> {
        Counter::DBCall.increment();
        let start_time = Instant::now();

        let result = origin_packages::table
            .select(origin_packages::target)
            .filter(origin_packages::origin.eq(&ident.origin))
            .filter(origin_packages::name.eq(&ident.name))
            .filter(origin_packages::ident_array.contains(&searchable_ident(ident)))
            .filter(origin_packages::visibility.eq_any(visibilities))
            .filter(origin_packages::hidden.eq(false))
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
         .filter(|s| !s.is_empty())
         .collect()
}

#[derive(Debug,
         Serialize,
         Deserialize,
         Clone,
         FromSqlRow,
         AsExpression,
         PartialEq,
         Eq,
         Hash)]
#[diesel(sql_type = Text)]
pub struct BuilderPackageType(pub PackageType);

impl FromStr for BuilderPackageType {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, crate::error::Error> {
        Ok(BuilderPackageType(PackageType::from_str(s).map_err(|_| {
                                  crate::error::Error::ParseError(format!("BuilderPackageType {}",
                                                                          s))
                              })?))
    }
}

impl ToSql<Text, Pg> for BuilderPackageType {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, Pg>) -> serialize::Result {
        out.write_all(self.to_string().as_bytes())
           .map(|_| IsNull::No)
           .map_err(Into::into)
    }
}

impl FromSql<Text, Pg> for BuilderPackageType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let s =
            std::str::from_utf8(bytes.as_bytes()).map_err(|e| {
                                                     std::io::Error::new(std::io::ErrorKind::Other,
                                                                         e)
                                                 })?;
        Ok(BuilderPackageType::from_str(s).map_err(|_| {
                                              std::io::Error::new(std::io::ErrorKind::Other,
                                                                  format!("Invalid ident: {}", s))
                                          })?)
    }
}

impl Deref for BuilderPackageType {
    type Target = PackageType;

    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug,
         Serialize,
         Deserialize,
         Clone,
         FromSqlRow,
         AsExpression,
         PartialEq,
         Eq,
         Hash)]
#[diesel(sql_type = Text)]
pub struct BuilderPackageIdent(pub PackageIdent);

impl FromStr for BuilderPackageIdent {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, crate::error::Error> {
        Ok(BuilderPackageIdent(PackageIdent::from_str(s).map_err(|_| {
                                                            crate::error::Error::ParseError(format!(
                    "BuilderPackageIdent \
                                                                            {}",
                    s
                ))
                                                        })?))
    }
}

impl ToSql<Text, Pg> for BuilderPackageIdent {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, Pg>) -> serialize::Result {
        out.write_all(self.to_string().as_bytes())
           .map(|_| IsNull::No)
           .map_err(Into::into)
    }
}

impl FromSql<Text, Pg> for BuilderPackageIdent {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let s =
            std::str::from_utf8(bytes.as_bytes()).map_err(|e| {
                                                     std::io::Error::new(std::io::ErrorKind::Other,
                                                                         e)
                                                 })?;
        Ok(BuilderPackageIdent::from_str(s).map_err(|_| {
                                               std::io::Error::new(std::io::ErrorKind::Other,
                                                                   format!("Invalid ident: {}", s))
                                           })?)
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
            .filter(|s| !s.is_empty())
            .collect()
    }
}

impl From<BuilderPackageIdent> for PackageIdent {
    fn from(value: BuilderPackageIdent) -> PackageIdent { value.0 }
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
#[diesel(sql_type = Text)]
pub struct BuilderPackageTarget(pub PackageTarget);

impl FromStr for BuilderPackageTarget {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, crate::error::Error> {
        Ok(BuilderPackageTarget(PackageTarget::from_str(s).map_err(|_| {
                                    crate::error::Error::ParseError(format!(
                "BuilderPackageTarget {}",
                s
            ))
                                })?))
    }
}

impl ToSql<Text, Pg> for BuilderPackageTarget {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, Pg>) -> serialize::Result {
        out.write_all(self.to_string().as_bytes())
           .map(|_| IsNull::No)
           .map_err(Into::into)
    }
}

impl FromSql<Text, Pg> for BuilderPackageTarget {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let s =
            std::str::from_utf8(bytes.as_bytes()).map_err(|e| {
                                                     std::io::Error::new(std::io::ErrorKind::Other,
                                                                         e)
                                                 })?;
        Ok(BuilderPackageTarget::from_str(s).map_err(|_| {
                                                std::io::Error::new(std::io::ErrorKind::Other,
                                                                    format!("Invalid ident: {}", s))
                                            })?)
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
                        visibility: PackageVisibility::Public,
                        package_type: BuilderPackageType(archive.package_type()?),
                        hidden: false })
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

impl From<PackageVisibility> for OriginPackageVisibility {
    fn from(value: PackageVisibility) -> OriginPackageVisibility {
        match value {
            PackageVisibility::Hidden => OriginPackageVisibility::Hidden,
            PackageVisibility::Private => OriginPackageVisibility::Private,
            _ => OriginPackageVisibility::Public,
        }
    }
}

impl From<Package> for OriginPackage {
    fn from(value: Package) -> OriginPackage {
        let exposes = value.exposes
                           .into_iter()
                           .map(|e| e as u32)
                           .collect::<Vec<u32>>();

        let mut op = OriginPackage::new();
        let ident = &*value.ident;
        op.set_id(value.id as u64);
        op.set_ident(OriginPackageIdent::from(ident.clone()));
        op.set_manifest(value.manifest);
        op.set_target(value.target.to_string());
        op.set_deps(into_idents(value.deps));
        op.set_tdeps(into_idents(value.tdeps));
        op.set_build_deps(into_idents(value.build_deps));
        op.set_build_tdeps(into_idents(value.build_tdeps));
        op.set_exposes(exposes);
        op.set_config(value.config);
        op.set_checksum(value.checksum);
        op.set_owner_id(value.owner_id as u64);
        op.set_visibility(value.visibility.into());
        op
    }
}

impl From<PackageWithVersionArray> for Package {
    fn from(value: PackageWithVersionArray) -> Package {
        Package { id:           value.id,
                  owner_id:     value.owner_id,
                  name:         value.name.clone(),
                  ident:        value.ident.clone(),
                  ident_array:  value.ident_array.clone(),
                  checksum:     value.checksum.clone(),
                  manifest:     value.manifest.clone(),
                  config:       value.config.clone(),
                  target:       value.target,
                  deps:         value.deps.clone(),
                  tdeps:        value.tdeps.clone(),
                  build_deps:   value.build_deps.clone(),
                  build_tdeps:  value.build_tdeps.clone(),
                  exposes:      value.exposes.clone(),
                  visibility:   value.visibility,
                  created_at:   value.created_at,
                  updated_at:   value.updated_at,
                  origin:       value.origin,
                  package_type: value.package_type, }
    }
}

impl From<BuilderPackageIdent> for OriginPackageIdent {
    fn from(value: BuilderPackageIdent) -> OriginPackageIdent { value.0.into() }
}

fn into_idents(column: Vec<BuilderPackageIdent>) -> protobuf::RepeatedField<OriginPackageIdent> {
    let mut idents = protobuf::RepeatedField::new();
    for ident in column {
        idents.push(ident.into());
    }
    idents
}

impl From<PackageWithChannelPlatform> for PackageIdentWithChannelPlatform {
    fn from(value: PackageWithChannelPlatform) -> PackageIdentWithChannelPlatform {
        let mut platforms = value.platforms.clone();
        platforms.dedup();

        PackageIdentWithChannelPlatform { origin: value.ident.origin.clone(),
                                          name: value.ident.name.clone(),
                                          version: value.ident.version.clone(),
                                          release: value.ident.release.clone(),
                                          channels: value.channels,
                                          platforms }
    }
}

impl From<BuilderPackageIdent> for PackageIdentWithChannelPlatform {
    fn from(value: BuilderPackageIdent) -> PackageIdentWithChannelPlatform {
        PackageIdentWithChannelPlatform { origin:    value.origin.clone(),
                                          name:      value.name.clone(),
                                          version:   value.version.clone(),
                                          release:   value.release.clone(),
                                          channels:  Vec::new(),
                                          platforms: Vec::new(), }
    }
}
