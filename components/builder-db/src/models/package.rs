use std::fmt;
use std::io::Write;
use std::ops::Deref;
use std::str::{self, FromStr};

use protobuf;
use protocol::originsrv::{OriginPackage, OriginPackageIdent, OriginPackageVisibility};

use chrono::NaiveDateTime;
use diesel;
use diesel::deserialize::{self, FromSql};
use diesel::dsl::sql;
use diesel::pg::expression::dsl::any;
use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use diesel::result::QueryResult;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::PgArrayExpressionMethods;
use diesel::RunQueryDsl;
use diesel_full_text_search::{to_tsquery, TsQueryExtensions};

use super::db_id_format;
use hab_core;
use hab_core::package::{FromArchive, Identifiable, PackageArchive, PackageIdent, PackageTarget};
use models::channel::{Channel, OriginChannelPackage, OriginChannelPromote};
use models::pagination::*;

use schema::channel::{origin_channel_packages, origin_channels};
use schema::origin::origins;
use schema::package::{origin_package_versions, origin_packages, packages_with_channel_platform};

use bldr_core::metrics::CounterMetric;
use metrics::Counter;

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable, Clone, Identifiable)]
#[table_name = "origin_packages"]
pub struct Package {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub ident: BuilderPackageIdent,
    pub ident_array: Vec<String>,
    pub checksum: String,
    pub manifest: String,
    pub config: String,
    pub target: BuilderPackageTarget,
    pub deps: Vec<BuilderPackageIdent>,
    pub tdeps: Vec<BuilderPackageIdent>,
    pub exposes: Vec<i32>,
    pub visibility: PackageVisibility,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub origin: String,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable, Clone, Identifiable)]
#[table_name = "packages_with_channel_platform"]
pub struct PackageWithChannelPlatform {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub ident: BuilderPackageIdent,
    pub ident_array: Vec<String>,
    pub checksum: String,
    pub manifest: String,
    pub config: String,
    pub target: BuilderPackageTarget,
    pub deps: Vec<BuilderPackageIdent>,
    pub tdeps: Vec<BuilderPackageIdent>,
    pub exposes: Vec<i32>,
    pub visibility: PackageVisibility,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub origin: String,
    pub channels: Vec<String>,
    pub platforms: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackageIdentWithChannelPlatform {
    pub origin: String,
    pub name: String,
    pub version: Option<String>,
    pub release: Option<String>,
    pub channels: Vec<String>,
    pub platforms: Vec<String>,
}

/// We literally never want to select `ident_vector`
/// so we provide this type and constant to pass to `.select`

type AllColumns = (
    origin_packages::id,
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
    origin_packages::exposes,
    origin_packages::visibility,
    origin_packages::created_at,
    origin_packages::updated_at,
    origin_packages::origin,
);

pub const ALL_COLUMNS: AllColumns = (
    origin_packages::id,
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
    origin_packages::exposes,
    origin_packages::visibility,
    origin_packages::created_at,
    origin_packages::updated_at,
    origin_packages::origin,
);

type All = diesel::dsl::Select<origin_packages::table, AllColumns>;

#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
#[table_name = "origin_packages"]
pub struct NewPackage {
    pub origin: String,
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub ident: BuilderPackageIdent,
    pub ident_array: Vec<String>,
    pub checksum: String,
    pub manifest: String,
    pub config: String,
    pub target: BuilderPackageTarget,
    pub deps: Vec<BuilderPackageIdent>,
    pub tdeps: Vec<BuilderPackageIdent>,
    pub exposes: Vec<i32>,
    pub visibility: PackageVisibility,
}

#[derive(Debug)]
pub struct GetLatestPackage {
    pub ident: BuilderPackageIdent,
    pub target: BuilderPackageTarget,
    pub visibility: Vec<PackageVisibility>,
}

#[derive(Debug)]
pub struct GetPackage {
    pub ident: BuilderPackageIdent,
    pub visibility: Vec<PackageVisibility>,
    pub target: BuilderPackageTarget,
}

#[derive(Debug)]
pub struct UpdatePackageVisibility {
    pub visibility: PackageVisibility,
    pub ids: Vec<i64>,
}

pub struct ListPackages {
    pub ident: BuilderPackageIdent,
    pub visibility: Vec<PackageVisibility>,
    pub page: i64,
    pub limit: i64,
}

pub struct SearchPackages {
    pub query: String,
    pub account_id: Option<i64>,
    pub page: i64,
    pub limit: i64,
}
#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct OriginPackageVersions {
    pub origin: String,
    pub name: String,
    pub version: String,
    #[serde(with = "db_id_format")]
    pub release_count: i64,
    pub latest: String,
    pub platforms: Vec<String>,
    pub visibility: PackageVisibility,
}

#[derive(DbEnum, Debug, Eq, Hash, Serialize, Deserialize, PartialEq, Clone, ToSql, FromSql)]
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
        vec![
            PackageVisibility::Public,
            PackageVisibility::Private,
            PackageVisibility::Hidden,
        ]
    }

    pub fn private() -> Vec<Self> {
        vec![PackageVisibility::Private, PackageVisibility::Hidden]
    }
}

impl Package {
    pub fn get_without_target(
        ident: BuilderPackageIdent,
        visibility: Vec<PackageVisibility>,
        conn: &PgConnection,
    ) -> QueryResult<Package> {
        Counter::DBCall.increment();
        Self::all()
            .filter(origin_packages::ident.eq(ident))
            .filter(origin_packages::visibility.eq(any(visibility)))
            .get_result(conn)
    }

    pub fn get(req: GetPackage, conn: &PgConnection) -> QueryResult<Package> {
        Counter::DBCall.increment();
        Self::all()
            .filter(origin_packages::ident.eq(req.ident))
            .filter(origin_packages::visibility.eq(any(req.visibility)))
            .filter(origin_packages::target.eq(req.target))
            .get_result(conn)
    }

    pub fn get_all(
        req_ident: BuilderPackageIdent,
        conn: &PgConnection,
    ) -> QueryResult<Vec<Package>> {
        Counter::DBCall.increment();
        Self::all()
            .filter(origin_packages::ident_array.contains(req_ident.parts()))
            .get_results(conn)
    }

    pub fn get_latest(req: GetLatestPackage, conn: &PgConnection) -> QueryResult<Package> {
        Counter::DBCall.increment();
        Self::all()
            .filter(origin_packages::ident_array.contains(req.ident.parts()))
            .filter(origin_packages::target.eq(req.target))
            .filter(origin_packages::visibility.eq(any(req.visibility)))
            .order(sql::<Package>(
                "to_semver(ident_array[3]) desc, ident_array[4] desc",
            )).limit(1)
            .get_result(conn)
    }

    pub fn create(package: NewPackage, conn: &PgConnection) -> QueryResult<Package> {
        Counter::DBCall.increment();
        let package = diesel::insert_into(origin_packages::table)
            .values(&package)
            .returning(ALL_COLUMNS)
            .on_conflict_do_nothing()
            .get_result::<Package>(conn)?;

        OriginChannelPackage::promote(
            OriginChannelPromote {
                ident: package.ident.clone(),
                origin: package.origin.clone(),
                channel: String::from("unstable"),
            },
            conn,
        )?;
        Ok(package)
    }

    pub fn update_visibility(
        vis: PackageVisibility,
        idt: BuilderPackageIdent,
        conn: &PgConnection,
    ) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_packages::table.filter(origin_packages::ident.eq(idt)))
            .set(origin_packages::visibility.eq(vis))
            .execute(conn)
    }

    pub fn update_visibility_bulk(
        req: UpdatePackageVisibility,
        conn: &PgConnection,
    ) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_packages::table.filter(origin_packages::id.eq_any(req.ids)))
            .set(origin_packages::visibility.eq(req.visibility))
            .execute(conn)
    }

    pub fn list(
        pl: ListPackages,
        conn: &PgConnection,
    ) -> QueryResult<(Vec<PackageWithChannelPlatform>, i64)> {
        Counter::DBCall.increment();

        packages_with_channel_platform::table
            .filter(packages_with_channel_platform::ident_array.contains(pl.ident.parts()))
            .filter(packages_with_channel_platform::visibility.eq(any(pl.visibility)))
            .order(packages_with_channel_platform::ident.desc())
            .paginate(pl.page)
            .per_page(pl.limit)
            .load_and_count_records(conn)
    }

    pub fn list_distinct(
        pl: ListPackages,
        conn: &PgConnection,
    ) -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        origin_packages::table
            .select(sql(
                "concat_ws('/', ident_array[1], ident_array[2]) as ident",
            )).filter(origin_packages::ident_array.contains(pl.ident.parts()))
            .filter(origin_packages::visibility.eq(any(pl.visibility)))
            // This is because diesel doesn't yet support group_by
            // see: https://github.com/diesel-rs/diesel/issues/210
            .filter(sql("TRUE GROUP BY ident_array[2], ident_array[1]"))
            .order(sql::<BuilderPackageIdent>("ident ASC"))
            .paginate(pl.page)
            .per_page(pl.limit)
            .load_and_count_records(conn)
    }

    pub fn distinct_for_origin(
        pl: ListPackages,
        conn: &PgConnection,
    ) -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        origin_packages::table
            .inner_join(origins::table)
            .select(sql("concat_ws('/', origins.name, origin_packages.name)"))
            .filter(origins::name.eq(&pl.ident.origin))
            .filter(origin_packages::visibility.eq(any(pl.visibility)))
            .filter(sql("TRUE GROUP BY origin_packages.name, origins.name"))
            .order(origins::name.asc())
            .paginate(pl.page)
            .per_page(pl.limit)
            .load_and_count_records(conn)
    }

    pub fn list_package_channels(
        ident: &BuilderPackageIdent,
        visibility: Vec<PackageVisibility>,
        conn: &PgConnection,
    ) -> QueryResult<Vec<Channel>> {
        Counter::DBCall.increment();
        origin_packages::table
            .inner_join(origin_channel_packages::table.inner_join(origin_channels::table))
            .select(origin_channels::table::all_columns())
            .filter(origin_packages::ident.eq(ident))
            .filter(origin_packages::visibility.eq(any(visibility)))
            .order(origin_channels::name.desc())
            .get_results(conn)
    }

    pub fn list_package_versions(
        ident: BuilderPackageIdent,
        visibility: Vec<PackageVisibility>,
        conn: &PgConnection,
    ) -> QueryResult<Vec<OriginPackageVersions>> {
        Counter::DBCall.increment();

        origin_package_versions::table
            .filter(origin_package_versions::origin.eq(ident.origin()))
            .filter(origin_package_versions::name.eq(ident.name()))
            .filter(origin_package_versions::visibility.eq(any(visibility)))
            .order(origin_package_versions::version.desc())
            .get_results(conn)
    }

    pub fn search(
        sp: SearchPackages,
        conn: &PgConnection,
    ) -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();
        let mut query = origin_packages::table
            .inner_join(origins::table)
            .select(origin_packages::ident)
            .filter(to_tsquery(sp.query).matches(origin_packages::ident_vector))
            .order(origin_packages::ident.asc())
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

        query
            .paginate(sp.page)
            .per_page(sp.limit)
            .load_and_count_records(conn)
    }

    // This is me giving up on fighting the typechecker and just duplicating a bunch of code
    pub fn search_distinct(
        sp: SearchPackages,
        conn: &PgConnection,
    ) -> QueryResult<(Vec<BuilderPackageIdent>, i64)> {
        Counter::DBCall.increment();

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
        query
            .paginate(sp.page)
            .per_page(sp.limit)
            .load_and_count_records(conn)
    }

    pub fn all() -> All {
        origin_packages::table.select(ALL_COLUMNS)
    }
    pub fn list_package_platforms(
        ident: BuilderPackageIdent,
        visibilities: Vec<PackageVisibility>,
        conn: &PgConnection,
    ) -> QueryResult<Vec<BuilderPackageTarget>> {
        origin_packages::table
            .select(origin_packages::target)
            .filter(origin_packages::ident_array.contains(&searchable_ident(&ident)))
            .filter(origin_packages::visibility.eq(any(visibilities)))
            .get_results(conn)
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
    ident
        .to_string()
        .split("/")
        .map(|s| s.to_string())
        .filter(|s| s != "")
        .collect()
}

#[derive(Debug, Serialize, Deserialize, Clone, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub struct BuilderPackageIdent(pub PackageIdent);

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
        self.to_string()
            .split("/")
            .map(|s| s.to_string())
            // We must filter out empty strings from the vec.
            // This sometimes happens hen the origin or the package name are undefined.
            .filter(|s| s != "")
            .collect()
    }
}

impl Into<PackageIdent> for BuilderPackageIdent {
    fn into(self) -> PackageIdent {
        self.0
    }
}

impl Deref for BuilderPackageIdent {
    type Target = PackageIdent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromSqlRow, AsExpression)]
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

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromArchive for NewPackage {
    type Error = hab_core::Error;

    fn from_archive(archive: &mut PackageArchive) -> hab_core::Result<Self> {
        let ident = match archive.ident() {
            Ok(value) => BuilderPackageIdent(value),
            Err(e) => return Err(hab_core::Error::from(e)),
        };

        let config = match archive.config()? {
            Some(config) => config,
            None => String::from(""),
        };

        let exposes = archive
            .exposes()?
            .into_iter()
            .map(|e| e as i32)
            .collect::<Vec<i32>>();

        let deps = archive
            .deps()?
            .into_iter()
            .map(|d| BuilderPackageIdent(d))
            .collect::<Vec<BuilderPackageIdent>>();

        let tdeps = archive
            .tdeps()?
            .into_iter()
            .map(|d| BuilderPackageIdent(d))
            .collect::<Vec<BuilderPackageIdent>>();

        // Some of the values here are made up because they are required in the db but not
        // necessarially requred for a valid package
        Ok(NewPackage {
            ident: ident.clone(),
            ident_array: ident.clone().parts(),
            origin: ident.origin().to_string(),
            manifest: archive.manifest()?,
            target: BuilderPackageTarget(archive.target()?),
            deps: deps,
            tdeps: tdeps,
            exposes: exposes,
            config: config,
            checksum: archive.checksum()?,
            name: ident.name.to_string(),
            owner_id: 999999999999,
            visibility: PackageVisibility::Public,
        })
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
        let exposes = self
            .exposes
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
        op.set_exposes(exposes);
        op.set_config(self.config);
        op.set_checksum(self.checksum);
        op.set_owner_id(self.owner_id as u64);
        op.set_visibility(self.visibility.into());
        op
    }
}

impl Into<OriginPackageIdent> for BuilderPackageIdent {
    fn into(self) -> OriginPackageIdent {
        self.0.into()
    }
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

        PackageIdentWithChannelPlatform {
            origin: self.ident.origin.clone(),
            name: self.ident.name.clone(),
            version: self.ident.version.clone(),
            release: self.ident.release.clone(),
            channels: self.channels,
            platforms: platforms,
        }
    }
}

impl Into<PackageIdentWithChannelPlatform> for BuilderPackageIdent {
    fn into(self) -> PackageIdentWithChannelPlatform {
        PackageIdentWithChannelPlatform {
            origin: self.origin.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            release: self.release.clone(),
            channels: Vec::new(),
            platforms: Vec::new(),
        }
    }
}
