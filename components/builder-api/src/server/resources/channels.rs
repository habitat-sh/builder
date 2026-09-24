// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{collections::{HashMap,
                        HashSet},
          str::FromStr};

use actix_web::{body::BoxBody,
                http::{self,
                       StatusCode},
                web::{self,
                      Data,
                      Path,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};
use bytes::Bytes;
use chrono::Utc;
use diesel::{pg::PgConnection,
             result::{DatabaseErrorKind,
                      Error::{DatabaseError,
                              NotFound}},
             Connection,
             QueryResult};
use rand::{self,
           RngExt};

use crate::{bldr_core::metrics::CounterMetric,
            hab_core::{package::{PackageIdent,
                                 PackageTarget},
                       ChannelIdent}};

use crate::db::models::{channel::*,
                        origin::*,
                        package::{BuilderPackageIdent,
                                  GetPackageGroup,
                                  Package,
                                  PackageVisibility}};

use crate::server::{authorize::authorize_session,
                    error::{Error,
                            Result},
                    framework::headers,
                    helpers::{self,
                              req_state,
                              visibility_for_optional_session,
                              Pagination,
                              PromoteChannelQuery,
                              Target,
                              ToChannel},
                    services::metrics::Counter,
                    AppState};

// Query param containers
#[derive(Debug, Default, Clone, Deserialize)]
struct SandboxBool {
    #[serde(default)]
    sandbox: bool,
}

pub struct Channels;

impl Channels {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/depot/channels/{origin}", web::get().to(get_channels))
           .route("/depot/channels/{origin}/{channel}",
                  web::post().to(create_channel))
           .route("/depot/channels/{origin}/{channel}",
                  web::delete().to(delete_channel))
           .route("/depot/channels/{origin}/{channel}/pkgs",
                  web::get().to(get_packages_for_origin_channel))
           .route("/depot/channels/{origin}/{channel}/pkgs/_latest",
                  web::get().to(get_latest_packages_for_origin_channel))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}",
                  web::get().to(get_packages_for_origin_channel_package))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/latest",
                  web::get().to(get_latest_package_for_origin_channel_package))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}",
                  web::get().to(get_packages_for_origin_channel_package_version))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/latest",
                  web::get().to(get_latest_package_for_origin_channel_package_version))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}",
                  web::get().to(get_package_fully_qualified))
           .route("/depot/channels/{origin}/{channel}/pkgs/promote",
                  web::put().to(promote_channel_packages))
           .route("/depot/channels/{origin}/{channel}/pkgs/demote",
                  web::put().to(demote_channel_packages))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}/promote",
                  web::put().to(promote_package))
           .route("/depot/channels/{origin}/{channel}/pkgs/{pkg}/{version}/{release}/demote",
                  web::put().to(demote_package));
    }
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
async fn get_channels(path: Path<String>,
                      sandbox: Query<SandboxBool>,
                      state: Data<AppState>)
                      -> HttpResponse {
    let origin = path.into_inner();

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Channel::list(&origin, sandbox.sandbox, &mut conn).map_err(Error::DieselError) {
        Ok(list) => {
            // TED: This is to maintain backwards API compat while killing some proto definitions
            // currently the output looks like [{"name": "foo"}] when it probably should be ["foo"]
            #[derive(Serialize)]
            struct Temp {
                name: String,
            }
            let ident_list: Vec<Temp> = list.iter()
                                            .map(|channel| Temp { name: channel.name.clone(), })
                                            .collect();
            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .json(ident_list)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get channels, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn create_channel(req: HttpRequest,
                        path: Path<(String, String)>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let (origin, channel) = path.into_inner();

    let session_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.id(),
            Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
        };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Channel::create(&CreateChannel { name:     &channel,
                                           origin:   &origin,
                                           owner_id: session_id as i64, },
                          &mut conn)
    {
        Ok(channel) => HttpResponse::Created().json(channel),
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            HttpResponse::Conflict().into()
        }
        Err(err) => {
            debug!("Failed to create channel, err={}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn delete_channel(req: HttpRequest,
                        path: Path<(String, String)>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let (origin, channel) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    if let Err(_err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return HttpResponse::new(StatusCode::UNAUTHORIZED);
    }

    if channel == ChannelIdent::stable() || channel == ChannelIdent::unstable() {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    state.memcache
         .borrow_mut()
         .clear_cache_for_channel(&origin, &channel);

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Channel::delete(&origin, &channel, &mut conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => {
            debug!("Failed to delete channel, err={}", err);
            err.into()
        }
    }
}

// Response structs for the snapshot=true path of promote_channel_packages
#[derive(Serialize)]
struct SnapshotResponse {
    snapshot_channel: String,
    // Keyed by target (e.g. "x86_64-linux") first, since a channel can hold
    // multiple "latest" head packages for the same origin/name -- one per
    // target platform -- and each target's closure must be reported
    // independently.
    packages:         HashMap<String, HashMap<String, HashMap<String, Vec<PackageFqiEntry>>>>,
}

#[derive(Serialize)]
struct PackageFqiEntry {
    ident:   String,
    origin:  String,
    name:    String,
    version: String,
    release: String,
}

// Response body for the check=true compatibility-check failure path of
// promote_channel_packages. `conflicts` is keyed by target platform first
// (mirroring SnapshotResponse.packages), since idents are only compared for
// conflicts within the same target.
#[derive(Serialize)]
struct CompatibilityError {
    error:     String,
    conflicts: HashMap<String, HashMap<String, Vec<String>>>,
}

// Parses an ident string of the form "origin/name/version/release" (release may be
// absent) and appends it to the target -> origin -> name -> entries map, unless an
// identical ident is already present for that target/origin/name. Multiple distinct
// idents can legitimately share a target/origin/name here: a head package's own
// ident may differ from an older version of the same name pinned as a tdep of some
// *other* head package, and both need to be reported rather than one silently
// clobbering the other. `target` is taken from the head package doing the
// referencing (its own target, for itself, or the parent's target for its tdeps --
// runtime tdeps must match their parent's target), since ident strings alone don't
// carry target information.
fn insert_ident(set: &mut HashMap<String,
                             HashMap<String, HashMap<String, Vec<PackageFqiEntry>>>>,
                target: &str,
                ident_str: &str) {
    let mut parts = ident_str.splitn(4, '/');
    let origin = parts.next().unwrap_or_default().to_string();
    let name = parts.next().unwrap_or_default().to_string();
    let version = parts.next().unwrap_or_default().to_string();
    let release = parts.next().unwrap_or_default().to_string();

    let entries = set.entry(target.to_string())
                     .or_default()
                     .entry(origin.clone())
                     .or_default()
                     .entry(name.clone())
                     .or_default();
    if !entries.iter().any(|e| e.ident == ident_str) {
        entries.push(PackageFqiEntry { ident: ident_str.to_string(),
                                       origin,
                                       name,
                                       version,
                                       release });
    }
}

// Merges a target channel's closure with an incoming source channel's
// closure for the purposes of the check=true compatibility check, into a
// target -> "origin/name" -> distinct idents map (consumed by
// find_conflicts).
//
// Promotion only ever inserts channel-package rows (Channel::promote_packages
// uses ON CONFLICT DO NOTHING) -- it never removes anything the target
// channel already has. Post-promotion, Channel::list_head_packages picks
// whichever ident is actually the highest version/release for a given
// origin/name/target as the new head. So for any group_key (a head package's
// target + origin/name) present on both sides, the side whose head ident is
// >= the other's head ident is the one that will actually be the
// post-promotion head, and only *that* side's whole group (head + its own
// recorded tdeps) describes the post-promotion state -- the losing side's
// group (including its tdeps) is dropped entirely, since it won't be a head
// afterward. A group_key present on only one side contributes in full,
// unconditionally.
fn merge_closures_for_conflict_check(target_closure: &[helpers::ClosureEntry],
                                     source_closure: &[helpers::ClosureEntry])
                                     -> HashMap<String, HashMap<String, HashSet<String>>> {
    let mut target_groups: HashMap<&(String, String), Vec<&helpers::ClosureEntry>> = HashMap::new();
    for entry in target_closure {
        target_groups.entry(&entry.group_key)
                     .or_default()
                     .push(entry);
    }
    let mut source_groups: HashMap<&(String, String), Vec<&helpers::ClosureEntry>> = HashMap::new();
    for entry in source_closure {
        source_groups.entry(&entry.group_key)
                     .or_default()
                     .push(entry);
    }

    let all_group_keys: HashSet<&(String, String)> = target_groups.keys()
                                                                  .chain(source_groups.keys())
                                                                  .copied()
                                                                  .collect();

    let mut by_target: HashMap<String, HashMap<String, HashSet<String>>> = HashMap::new();
    for group_key in all_group_keys {
        let winning_group = match (target_groups.get(group_key), source_groups.get(group_key)) {
            (Some(target_group), Some(source_group)) => {
                let target_head = target_group.iter().find(|e| e.is_head).map(|e| &e.ident.0);
                let source_head = source_group.iter().find(|e| e.is_head).map(|e| &e.ident.0);
                match (target_head, source_head) {
                    (Some(th), Some(sh)) if sh >= th => source_group,
                    (Some(_), Some(_)) => target_group,
                    // Shouldn't normally happen (every group carries a head
                    // entry), but fall back to whichever side is present.
                    (None, Some(_)) => source_group,
                    _ => target_group,
                }
            }
            (Some(target_group), None) => target_group,
            (None, Some(source_group)) => source_group,
            (None, None) => continue,
        };

        for entry in winning_group {
            by_target.entry(entry.target.clone())
                     .or_default()
                     .entry(format!("{}/{}", entry.ident.origin, entry.ident.name))
                     .or_default()
                     .insert(entry.ident.to_string());
        }
    }
    by_target
}

// Returns, for every target -> "origin/name" key with more than one distinct
// ident, the sorted list of conflicting idents. An empty map means the merged
// closure (target channel's existing packages plus the source channel's
// incoming head+tdeps closure) is internally consistent -- at most one
// distinct ident per origin/name within each target platform. Idents for the
// same origin/name under *different* target platforms (e.g. an
// x86_64-linux and an x86_64-windows build of the same package) are
// legitimate and never considered conflicting with each other.
fn find_conflicts(by_target: &HashMap<String, HashMap<String, HashSet<String>>>)
                  -> HashMap<String, HashMap<String, Vec<String>>> {
    by_target.iter()
             .filter_map(|(target, by_name)| {
                 let conflicts: HashMap<String, Vec<String>> =
                     by_name.iter()
                            .filter(|(_, versions)| versions.len() > 1)
                            .map(|(name, versions)| {
                                let mut v: Vec<String> = versions.iter().cloned().collect();
                                v.sort();
                                (name.clone(), v)
                            })
                            .collect();
                 if conflicts.is_empty() {
                     None
                 } else {
                     Some((target.clone(), conflicts))
                 }
             })
             .collect()
}

// Error type produced by the promotion transaction closure in
// promote_channel_packages. A `Conflict` causes the transaction to roll back
// (nothing gets written to the target channel); `Diesel` wraps any
// underlying database error.
enum PromoteTxnError {
    Diesel(diesel::result::Error),
    Conflict(HashMap<String, HashMap<String, Vec<String>>>),
}

impl From<diesel::result::Error> for PromoteTxnError {
    fn from(e: diesel::result::Error) -> Self { PromoteTxnError::Diesel(e) }
}

struct PromoteTxnSuccess {
    pkg_ids:  Vec<i64>,
    snapshot: Option<SnapshotResponse>,
}

// Acquires the per-channel advisory lock (see Channel::lock_channel) for both
// `a` and `b`, always in the same lexicographic order regardless of which one
// is logically the source or target for a given request. This is required so
// that two concurrent requests locking the same pair of channels in opposite
// roles (e.g. a promotion from X to Y racing a promotion from Y to X) cannot
// deadlock by acquiring the two locks in opposite orders. Promotion/demotion
// paths in this module that read a channel's packages and then perform a
// dependent write (single- or bulk-) should acquire this pair of locks first
// (or Channel::lock_channel directly for a single channel), so that
// another such path cannot commit a package into source or target between the
// read and the dependent write.
fn lock_channels(origin: &str, a: &str, b: &str, conn: &mut PgConnection) -> QueryResult<()> {
    let (first, second) = if a <= b { (a, b) } else { (b, a) };
    Channel::lock_channel(origin, first, conn)?;
    if first != second {
        Channel::lock_channel(origin, second, conn)?;
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
async fn promote_channel_packages(req: HttpRequest,
                                  path: Path<(String, String)>,
                                  state: Data<AppState>,
                                  query: Query<PromoteChannelQuery>)
                                  -> HttpResponse {
    Counter::AtomicChannelRequests.increment();
    let (origin, channel) = path.into_inner();

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let ch_source = ChannelIdent::from(channel);
    let ch_target = ChannelIdent::from(query.channel.as_ref());

    // Simple guards to protect users from bad decisioning (mirrors the
    // equivalent guards used by the demote path's
    // do_promote_or_demote_channel_packages).
    if ch_target.as_str().is_empty()
       || ch_target == ch_source
       || ch_target == ChannelIdent::unstable()
    {
        let body = Bytes::from("Invalid target channel: must be non-empty, different from the \
                                source channel, and not 'unstable'"
                                                                   .to_string()
                                                                   .into_bytes());
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, BoxBody::new(body));
    }

    let check = query.check;
    let want_snapshot = query.snapshot;

    let txn_result = conn.transaction::<PromoteTxnSuccess, PromoteTxnError, _>(|conn| {
        // Serialize this whole read-then-write sequence (check and promote)
        // against every other path that can mutate either channel's package
        // membership -- bulk or single-package promote/demote -- so a
        // concurrent write can't land between this request's check and its
        // own write, on either the source or the target side. Held for the
        // duration of the transaction.
        lock_channels(&origin, ch_source.as_str(), ch_target.as_str(), conn)?;

        #[rustfmt::skip]
        let target_channel = match Channel::get(&origin, &ch_target, conn) {
            Ok(channel) => channel,
            Err(NotFound) => {
                if (ch_target != ChannelIdent::stable()) && (ch_target != ChannelIdent::unstable()) {
                    Channel::create(
                        &CreateChannel {
                            name:     ch_target.as_str(),
                            origin:   &origin,
                            owner_id: session.id() as i64,
                        },
                    conn)?
                } else {
                    warn!("Unable to retrieve target channel: {}", ch_target);
                    return Err(PromoteTxnError::Diesel(NotFound));
                }
            }
            Err(e) => {
                info!("Unable to retrieve channel, err: {:?}", e);
                return Err(PromoteTxnError::Diesel(e));
            }
        };

        // Resolve the source channel once and reuse its id for both the
        // compatibility check's closure computation (if any) and the
        // package listing below, instead of re-resolving it by name a
        // second time.
        let source_channel_id = Channel::get(&origin, &ch_source, conn).ok().map(|c| c.id);

        if check {
            let target_closure = helpers::channel_package_closure(Some(target_channel.id), conn)?;
            let source_closure = helpers::channel_package_closure(source_channel_id, conn)?;

            let by_target = merge_closures_for_conflict_check(&target_closure, &source_closure);

            let conflicts = find_conflicts(&by_target);
            if !conflicts.is_empty() {
                return Err(PromoteTxnError::Conflict(conflicts));
            }
        }

        let pkgs = match source_channel_id {
            Some(id) => {
                Channel::list_all_packages_by_channel_id_idents(id, &PackageVisibility::all(),
                                                                conn)?
            }
            None => Vec::new(),
        };

        let op = Package::get_group(GetPackageGroup { pkgs,
                                                       visibility: PackageVisibility::all(), },
                                    conn)?;

        let pkg_ids: Vec<i64> = op.iter().map(|x| x.id).collect();

        debug!("Bulk promoting Pkg IDs: {:?}", pkg_ids);
        Channel::promote_packages(target_channel.id, &pkg_ids, conn)?;

        let snapshot = if want_snapshot {
            Some(create_snapshot_channel(&origin, &ch_target, session.id() as i64, conn)?)
        } else {
            None
        };

        Ok(PromoteTxnSuccess { pkg_ids, snapshot })
    });

    match txn_result {
        Ok(success) => {
            match PackageGroupChannelAudit::audit(
                PackageGroupChannelAudit {
                    origin: &origin,
                    channel: ch_target.as_str(),
                    package_ids: success.pkg_ids,
                    operation: PackageChannelOperation::Promote,
                    trigger: helpers::trigger_from_request_model(&req),
                    requester_id: session.id() as i64,
                    requester_name: session.name(),
                    group_id: 0_i64,
                },
                &mut conn,
            ) {
                Ok(_) => {}
                Err(e) => debug!("Failed to save rank change to audit log: {}", e),
            };

            match success.snapshot {
                Some(response) => HttpResponse::Ok().json(response),
                None => HttpResponse::new(StatusCode::OK),
            }
        }
        Err(PromoteTxnError::Conflict(conflicts)) => {
            HttpResponse::Conflict().json(CompatibilityError { error:
                                                                   "compatibility_check_failed".to_string(),
                                                               conflicts, })
        }
        Err(PromoteTxnError::Diesel(e)) => {
            debug!("Failed to promote channel packages, err={}", e);
            Error::DieselError(e).into()
        }
    }
}

fn create_snapshot_channel(origin: &str,
                           ch_target: &ChannelIdent,
                           owner_id: i64,
                           conn: &mut PgConnection)
                           -> std::result::Result<SnapshotResponse, PromoteTxnError> {
    const MAX_SNAPSHOT_NAME_ATTEMPTS: u32 = 5;

    let mut last_err = None;

    for attempt in 0..MAX_SNAPSHOT_NAME_ATTEMPTS {
        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.6fZ").to_string();
        let suffix: u32 = rand::rng().random();
        let snapshot_name = format!("{}_SS_{}_{:08x}", ch_target.as_str(), timestamp, suffix);

        let txn_result =
            conn.transaction::<SnapshotResponse, diesel::result::Error, _>(|conn| {
                    let snapshot_channel = Channel::create(&CreateChannel { name:
                                                                                &snapshot_name,
                                                                            origin,
                                                                            owner_id },
                                                           conn)?;

                    let target_channel = Channel::get(origin, ch_target, conn)?;

                    let all_target_pkg_ids =
                        Channel::list_all_packages_by_channel_id(target_channel.id,
                                                                 &PackageVisibility::all(),
                                                                 conn)?;

                    Channel::promote_packages(snapshot_channel.id, &all_target_pkg_ids, conn)?;

                    let head_packages = Channel::list_head_packages(snapshot_channel.id, conn)?;

                    let mut pkg_set: HashMap<String,
                                             HashMap<String,
                                                     HashMap<String, Vec<PackageFqiEntry>>>> =
                        HashMap::new();
                    // Insert all head packages first so each target/origin/name's
                    // list always starts with the actual head ident. Older
                    // tdep idents for the same target/origin/name (e.g. a
                    // not-yet-updated head package pinned to a previous
                    // version of a dependency that has since been promoted
                    // to head status) are appended alongside it rather than
                    // being dropped, since both idents are genuinely part of
                    // the snapshot's closure. tdeps are grouped under their
                    // parent head package's target, since runtime tdeps
                    // must match their parent's target platform.
                    for pkg in &head_packages {
                        insert_ident(&mut pkg_set,
                                     &pkg.target.to_string(),
                                     &pkg.ident.to_string());
                    }
                    for pkg in &head_packages {
                        for dep in &pkg.tdeps {
                            insert_ident(&mut pkg_set, &pkg.target.to_string(), &dep.to_string());
                        }
                    }

                    Ok(SnapshotResponse { snapshot_channel: snapshot_name.clone(),
                                          packages:         pkg_set, })
                });

        match txn_result {
            Ok(r) => return Ok(r),
            Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _))
                if attempt + 1 < MAX_SNAPSHOT_NAME_ATTEMPTS =>
            {
                debug!("Snapshot channel name {} collided, retrying with a new name",
                       snapshot_name);
                continue;
            }
            Err(e) => {
                last_err = Some(e);
                break;
            }
        }
    }

    Err(PromoteTxnError::Diesel(last_err.expect("snapshot creation failed without recording the \
                                                 last error")))
}

#[allow(clippy::needless_pass_by_value)]
async fn demote_channel_packages(req: HttpRequest,
                                 path: Path<(String, String)>,
                                 state: Data<AppState>,
                                 to_channel: Query<ToChannel>)
                                 -> HttpResponse {
    let (origin, channel) = path.into_inner();
    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let ch_source = ChannelIdent::from(channel);
    let ch_target = ChannelIdent::from(to_channel.channel.as_ref());

    match do_promote_or_demote_channel_packages(&req,
                                                &ch_source,
                                                &ch_target,
                                                &origin,
                                                false,
                                                session.id() as i64)
    {
        Ok(pkg_ids) => {
            match PackageGroupChannelAudit::audit(
                PackageGroupChannelAudit {
                    origin: &origin,
                    channel: ch_target.as_str(),
                    package_ids: pkg_ids,
                    operation: PackageChannelOperation::Demote,
                    trigger: helpers::trigger_from_request_model(&req),
                    requester_id: session.id() as i64,
                    requester_name: session.name(),
                    group_id: 0_i64,
                },
                &mut conn,
            ) {
                Ok(_) => {}
                Err(e) => debug!("Failed to save rank change to audit log: {}", e),
            };
            HttpResponse::new(StatusCode::OK)
        }
        Err(e) => {
            debug!("Failed to demote channel packages, err={}", e);
            e.into()
        }
    }
}

fn do_promote_or_demote_channel_packages(req: &HttpRequest,
                                         ch_source: &ChannelIdent,
                                         ch_target: &ChannelIdent,
                                         origin: &str,
                                         promote: bool,
                                         session_id: i64)
                                         -> Result<Vec<i64>> {
    Counter::AtomicChannelRequests.increment();
    let mut conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    // Simple guards to protect users from bad decisioning
    if ch_target.as_str().is_empty() {
        return Err(Error::BadRequest);
    }

    if !promote
       && (*ch_target == ChannelIdent::unstable() || *ch_source == ChannelIdent::unstable())
    {
        return Err(Error::BadRequest);
    }

    if *ch_target == *ch_source {
        return Err(Error::BadRequest);
    }

    if promote && *ch_target == ChannelIdent::unstable() {
        return Err(Error::BadRequest);
    }

    conn.transaction::<Vec<i64>, Error, _>(|conn| {
        // Serialize this read-then-write sequence against every other path
        // that can mutate either channel's package membership -- bulk or
        // single-package promote/demote, including the check=true bulk
        // promote path -- so a concurrent write can't land between the
        // package read below and the promote/demote write.
        lock_channels(origin, ch_source.as_str(), ch_target.as_str(), conn)?;

        let pkgs =
            Channel::list_all_packages(&ListAllChannelPackages { visibility:
                                                                     &PackageVisibility::all(),
                                                                 origin,
                                                                 channel: ch_source, },
                                       conn)?;

        #[rustfmt::skip]
        let channel = match Channel::get(origin, ch_target, conn) {
            Ok(channel) => channel,
            Err(NotFound) => {
                if (ch_target != &ChannelIdent::stable()) && (ch_target != &ChannelIdent::unstable()) {
                    Channel::create(
                        &CreateChannel {
                            name:     ch_target.as_str(),
                            origin,
                            owner_id: session_id,
                        },
                    conn)?
                } else {
                    warn!("Unable to retrieve target channel: {}", ch_target);
                    return Err(Error::DieselError(NotFound));
                }
            }
            Err(e) => {
                info!("Unable to retrieve channel, err: {:?}", e);
                return Err(Error::DieselError(e));
            }
        };

        #[rustfmt::skip]
        let op = Package::get_group(
            GetPackageGroup {
                pkgs,
                visibility: PackageVisibility::all()
            },
        conn)?;

        let pkg_ids: Vec<i64> = op.iter().map(|x| x.id).collect();

        if promote {
            debug!("Bulk promoting Pkg IDs: {:?}", pkg_ids);
            Channel::promote_packages(channel.id, &pkg_ids, conn)?;
        } else {
            debug!("Bulk demoting Pkg IDs: {:?}", pkg_ids);
            Channel::demote_packages(channel.id, &pkg_ids, conn)?;
        }
        Ok(pkg_ids)
    })
}

#[allow(clippy::needless_pass_by_value)]
async fn promote_package(req: HttpRequest,
                         path: Path<(String, String, String, String, String)>,
                         qtarget: Query<Target>,
                         state: Data<AppState>)
                         -> HttpResponse {
    let (origin, channel, pkg, version, release) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let ident = PackageIdent::new(origin.clone(), pkg, Some(version), Some(release));

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => {
                    debug!("Invalid target requested: {}, err = {:?}", t, err);
                    let body = Bytes::from(format!("Invalid package target '{}'", t).into_bytes());
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                                   BoxBody::new(body));
                }
            }
        }
        None => helpers::target_from_headers(&req),
    };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let auditevent = PackageChannelAudit { package_ident:  BuilderPackageIdent(ident.clone()),
                                           channel:        channel.as_str(),
                                           operation:      PackageChannelOperation::Promote,
                                           trigger:
                                               helpers::trigger_from_request_model(&req),
                                           requester_id:   session.id() as i64,
                                           requester_name: session.name(),
                                           origin:         &origin, };

    match conn.transaction::<usize, diesel::result::Error, _>(|conn| {
                  // Serialize against any other path (single- or
                  // bulk-, promote or demote, including the check=true
                  // bulk promote path) that reads or writes this
                  // channel's package membership.
                  Channel::lock_channel(&origin, channel.as_str(), conn)?;
                  OriginChannelPackage::promote(OriginChannelPromote { ident:
                                                                            BuilderPackageIdent(ident.clone()),
                                                                        target,
                                                                        origin: origin.clone(),
                                                                        channel: channel.clone(), },
                                                conn)
              })
              .map_err(Error::DieselError)
    {
        Ok(promoted_count) => {
            // Note: promoted_count is 0 when attempting to promote a package to a channel where it already exists
            if promoted_count != 0 {
                if let Err(e) = PackageChannelAudit::audit(&auditevent, &mut conn) {
                    debug!("Failed to save rank change to audit log: {}", e);
                };
            }

            state.memcache.borrow_mut().clear_cache_for_package(&ident);
            HttpResponse::new(StatusCode::OK)
        }
        Err(err) => {
            debug!("Failed to promote package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn demote_package(req: HttpRequest,
                        path: Path<(String, String, String, String, String)>,
                        qtarget: Query<Target>,
                        state: Data<AppState>)
                        -> HttpResponse {
    let (origin, channel, pkg, version, release) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    if channel == ChannelIdent::unstable() {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        Ok(session) => session,
        Err(_) => return HttpResponse::new(StatusCode::UNAUTHORIZED),
    };

    let ident = PackageIdent::new(origin.clone(), pkg, Some(version), Some(release));

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            match PackageTarget::from_str(t) {
                Ok(t) => t,
                Err(err) => {
                    debug!("Invalid target requested: {}, err = {:?}", t, err);
                    let body = Bytes::from(format!("Invalid package target '{}'", t).into_bytes());
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                                   BoxBody::new(body));
                }
            }
        }
        None => helpers::target_from_headers(&req),
    };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match conn.transaction::<usize, diesel::result::Error, _>(|conn| {
                  // Serialize against any other path (single- or
                  // bulk-, promote or demote, including the check=true
                  // bulk promote path) that reads or writes this
                  // channel's package membership.
                  Channel::lock_channel(&origin, channel.as_str(), conn)?;
                  OriginChannelPackage::demote(OriginChannelDemote { ident:
                                                                          BuilderPackageIdent(ident.clone()),
                                                                      target,
                                                                      origin: origin.clone(),
                                                                      channel: channel.clone() },
                                               conn)
              })
              .map_err(Error::DieselError)
    {
        Ok(0) => {
            debug!("Requested package {} for target {} not present in channel {}",
                   ident, target, channel);
            HttpResponse::new(StatusCode::BAD_REQUEST)
        }
        Ok(_) => {
            match PackageChannelAudit::audit(
                &PackageChannelAudit {
                    package_ident: BuilderPackageIdent(ident.clone()),
                    channel: channel.as_str(),
                    operation: PackageChannelOperation::Demote,
                    trigger: helpers::trigger_from_request_model(&req),
                    requester_id: session.id() as i64,
                    requester_name: session.name(),
                    origin: &origin,
                },
                &mut conn,
            ) {
                Ok(_) => {}
                Err(err) => debug!("Failed to save rank change to audit log: {}", err),
            };
            state.memcache.borrow_mut().clear_cache_for_package(&ident);
            HttpResponse::new(StatusCode::OK)
        }
        Err(err) => {
            debug!("Failed to demote package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_packages_for_origin_channel_package_version(req: HttpRequest,
                                                         path: Path<(String,
                                                               String,
                                                               String,
                                                               String)>,
                                                         pagination: Query<Pagination>)
                                                         -> HttpResponse {
    let (origin, channel, pkg, version) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, Some(version), None);

    match do_get_channel_packages(&req, &pagination, &ident, &channel) {
        Ok((packages, count)) => {
            postprocess_channel_package_list(&req, &packages, count, &pagination)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get packages, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_packages_for_origin_channel_package(req: HttpRequest,
                                                 path: Path<(String, String, String)>,
                                                 pagination: Query<Pagination>)
                                                 -> HttpResponse {
    let (origin, channel, pkg) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_channel_packages(&req, &pagination, &ident, &channel) {
        Ok((packages, count)) => {
            postprocess_channel_package_list(&req, &packages, count, &pagination)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get packages, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_packages_for_origin_channel(req: HttpRequest,
                                         path: Path<(String, String)>,
                                         pagination: Query<Pagination>)
                                         -> HttpResponse {
    let (origin, channel) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    // It feels 1000x wrong to set the package name to ""
    let ident = PackageIdent::new(origin, String::from(""), None, None);

    match do_get_channel_packages(&req, &pagination, &ident, &channel) {
        Ok((packages, count)) => {
            postprocess_channel_package_list(&req, &packages, count, &pagination)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get packages, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_latest_package_for_origin_channel_package(req: HttpRequest,
                                                       path: Path<(String, String, String)>,
                                                       qtarget: Query<Target>)
                                                       -> HttpResponse {
    let (origin, channel, pkg) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, None, None);

    match do_get_channel_package(&req, &qtarget, &ident, &channel) {
        Ok(json_body) => {
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get latest package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_latest_package_for_origin_channel_package_version(req: HttpRequest,
                                                               path: Path<(String,
                                                                     String,
                                                                     String,
                                                                     String)>,
                                                               qtarget: Query<Target>)
                                                               -> HttpResponse {
    let (origin, channel, pkg, version) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, Some(version), None);

    match do_get_channel_package(&req, &qtarget, &ident, &channel) {
        Ok(json_body) => {
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get latest package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_package_fully_qualified(req: HttpRequest,
                                     path: Path<(String, String, String, String, String)>,
                                     qtarget: Query<Target>)
                                     -> HttpResponse {
    let (origin, channel, pkg, version, release) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    let ident = PackageIdent::new(origin, pkg, Some(version), Some(release));

    match do_get_channel_package(&req, &qtarget, &ident, &channel) {
        Ok(json_body) => {
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(err) => {
            debug!("Failed to get package, err={}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_latest_packages_for_origin_channel(req: HttpRequest,
                                                path: Path<(String, String)>,
                                                qtarget: Query<Target>)
                                                -> HttpResponse {
    let (origin, channel) = path.into_inner();
    let channel = ChannelIdent::from(channel);

    match do_get_latest_channel_packages(&req, &qtarget, &origin, &channel) {
        Ok((channel, target, data)) => {
            let json_body = helpers::channel_listing_results_json(&channel, &target, &data);
            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(json_body)
        }
        Err(Error::NotFound) => HttpResponse::new(StatusCode::NOT_FOUND),
        Err(Error::BadRequest) => HttpResponse::new(StatusCode::BAD_REQUEST),
        Err(err) => {
            debug!("Failed to get package, err={}", err);
            err.into()
        }
    }
}

// Internal - these functions should return Result<..>
//

fn do_get_latest_channel_packages(req: &HttpRequest,
                                  qtarget: &Query<Target>,
                                  origin: &str,
                                  channel: &ChannelIdent)
                                  -> Result<(String, String, Vec<BuilderPackageIdent>)> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.id()),
        Err(_) => None,
    };

    // This is a new API, so we only look at the query string not the headers.
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            t
        }
        None => return Err(Error::BadRequest),
    };

    let mut conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    Channel::list_latest_packages(
        &ListAllChannelPackagesForTarget {
            visibility: &helpers::visibility_for_optional_session(req, opt_session_id, origin),
            channel,
            origin,
            target,
        },
        &mut conn,
    )
    .map_err(Error::DieselError)
}

fn do_get_channel_packages(req: &HttpRequest,
                           pagination: &Query<Pagination>,
                           ident: &PackageIdent,
                           channel: &ChannelIdent)
                           -> Result<(Vec<BuilderPackageIdent>, i64)> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.id()),
        Err(_) => None,
    };
    let (page, per_page) = helpers::extract_pagination_in_pages(pagination);

    let mut conn = req_state(req).db.get_conn().map_err(Error::DbError)?;

    Channel::list_packages(
        &ListChannelPackages {
            ident: BuilderPackageIdent(ident.clone()),
            visibility: helpers::visibility_for_optional_session(
                req,
                opt_session_id,
                &ident.origin,
            ),
            origin: ident.origin.clone(),
            channel: channel.clone(),
            page: page as i64,
            limit: per_page as i64,
        },
        &mut conn,
    )
    .map_err(Error::DieselError)
}

fn do_get_channel_package(req: &HttpRequest,
                          qtarget: &Query<Target>,
                          ident: &PackageIdent,
                          channel: &ChannelIdent)
                          -> Result<String> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.id()),
        Err(_) => None,
    };
    Counter::GetChannelPackage.increment();

    let req_ident = ident.clone();

    // TODO: Deprecate target from headers
    let target = match qtarget.target {
        Some(ref t) => {
            trace!("Query requested target = {}", t);
            PackageTarget::from_str(t)?
        }
        None => helpers::target_from_headers(req),
    };

    // Scope this memcache usage so the reference goes out of
    // scope before the visibility_for_optional_session call
    // below
    {
        let mut memcache = req_state(req).memcache.borrow_mut();
        match memcache.get_package(&req_ident, channel, &target, opt_session_id) {
            (true, Some(pkg_json)) => {
                trace!("Channel package {} {} {} {:?} - cache hit with pkg json",
                       channel,
                       ident,
                       target,
                       opt_session_id);
                // Note: the Package specifier is needed even though the variable is un-used
                let _p: Package = match serde_json::from_str(&pkg_json) {
                    Ok(p) => p,
                    Err(e) => {
                        debug!("Unable to deserialize package json, err={:?}", e);
                        return Err(Error::SerdeJson(e));
                    }
                };
                Counter::MemcacheChannelPackageHit.increment();
                return Ok(pkg_json);
            }
            (true, None) => {
                trace!("Channel package {} {} {} {:?} - cache hit with 404",
                       channel,
                       ident,
                       target,
                       opt_session_id);
                Counter::MemcacheChannelPackage404.increment();
                return Err(Error::NotFound);
            }
            (false, _) => {
                trace!("Channel package {} {} {} {:?} - cache miss",
                       channel,
                       ident,
                       target,
                       opt_session_id);
                Counter::MemcacheChannelPackageMiss.increment();
            }
        };
    }

    let mut conn = req_state(req).db.get_conn()?;

    let pkg: Package = match Channel::get_latest_package(
        &GetLatestPackage {
            ident: &BuilderPackageIdent(ident.clone()),
            channel,
            target: &target,
            visibility: &helpers::visibility_for_optional_session(
                req,
                opt_session_id,
                &ident.origin,
            ),
        },
        &mut conn,
    ) {
        Ok(pkg) => pkg.into(),
        Err(NotFound) => {
            let mut memcache = req_state(req).memcache.borrow_mut();
            memcache.set_package(&req_ident, None, channel, &target, opt_session_id);
            return Err(Error::NotFound);
        }
        Err(err) => return Err(err.into()),
    };

    let mut pkg_json = serde_json::to_value(pkg.clone()).unwrap();
    let channels = channels_for_package_ident(req, &pkg.ident, target, &mut conn)?;

    pkg_json["channels"] = json!(channels);
    pkg_json["is_a_service"] = json!(pkg.is_a_service());

    let json_body = serde_json::to_string(&pkg_json).unwrap();

    {
        let mut memcache = req_state(req).memcache.borrow_mut();
        memcache.set_package(&req_ident,
                             Some(&json_body),
                             channel,
                             &target,
                             opt_session_id);
    }

    Ok(json_body)
}

pub fn channels_for_package_ident(req: &HttpRequest,
                                  package: &BuilderPackageIdent,
                                  target: PackageTarget,
                                  conn: &mut PgConnection)
                                  -> Result<Option<Vec<String>>> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.id()),
        Err(_) => None,
    };

    match Package::list_package_channels(package,
                                         target,
                                         visibility_for_optional_session(req,
                                                                         opt_session_id,
                                                                         &package.clone().origin),
                                         &mut *conn).map_err(Error::DieselError)
    {
        Ok(channels) => {
            let list: Vec<String> = channels.iter()
                                            .map(|channel| channel.name.to_string())
                                            .collect();

            Ok(Some(list))
        }
        Err(err) => Err(err),
    }
}

// Helper

fn postprocess_channel_package_list(_req: &HttpRequest,
                                    packages: &[BuilderPackageIdent],
                                    count: i64,
                                    pagination: &Query<Pagination>)
                                    -> HttpResponse {
    let (start, _) = helpers::extract_pagination(pagination);
    let pkg_count = packages.len() as isize;
    let stop = match pkg_count {
        0 => count,
        _ => (start + pkg_count - 1) as i64,
    };

    debug!("postprocessing channel package list, start: {}, stop: {}, total_count: {}",
           start, stop, count);

    let body = helpers::package_results_json(packages, count as isize, start, stop as isize);

    let mut response = if count as isize > (stop as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response.append_header((http::header::CONTENT_TYPE, headers::APPLICATION_JSON))
            .append_header((http::header::CACHE_CONTROL, headers::Cache::NoCache.to_string()))
            .body(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reproduces the scenario reported against the snapshot=true path: a
    // head package (e.g. openssl 1.1.1w, just promoted) shares an
    // origin/name with an older ident pinned as a tdep of some other,
    // unrelated head package (e.g. libarchive, still built against openssl
    // 1.1.1l). Both idents must be preserved in the snapshot summary
    // instead of the tdep silently losing the origin/name slot.
    #[test]
    fn insert_ident_keeps_distinct_idents_for_same_origin_name() {
        let mut set: HashMap<String, HashMap<String, HashMap<String, Vec<PackageFqiEntry>>>> =
            HashMap::new();

        // Head package pass
        insert_ident(&mut set,
                     "x86_64-linux",
                     "core/openssl/1.1.1w/20240108093230");
        // Tdep pass (from an unrelated head package still pinned to the old version)
        insert_ident(&mut set,
                     "x86_64-linux",
                     "core/openssl/1.1.1l/20220425143501");

        let entries = &set["x86_64-linux"]["core"]["openssl"];
        assert_eq!(entries.len(), 2);
        assert!(entries.iter()
                       .any(|e| e.ident == "core/openssl/1.1.1w/20240108093230"));
        assert!(entries.iter()
                       .any(|e| e.ident == "core/openssl/1.1.1l/20220425143501"));
    }

    #[test]
    fn insert_ident_dedupes_identical_idents() {
        let mut set: HashMap<String, HashMap<String, HashMap<String, Vec<PackageFqiEntry>>>> =
            HashMap::new();

        // The same ident can legitimately be reached twice (e.g. as a tdep
        // of two different head packages); it should only appear once.
        insert_ident(&mut set,
                     "x86_64-linux",
                     "core/openssl/1.1.1w/20240108093230");
        insert_ident(&mut set,
                     "x86_64-linux",
                     "core/openssl/1.1.1w/20240108093230");

        assert_eq!(set["x86_64-linux"]["core"]["openssl"].len(), 1);
    }

    #[test]
    fn insert_ident_parses_origin_name_version_release() {
        let mut set: HashMap<String, HashMap<String, HashMap<String, Vec<PackageFqiEntry>>>> =
            HashMap::new();

        insert_ident(&mut set, "x86_64-linux", "core/xz/5.2.5/20220425103110");

        let entry = &set["x86_64-linux"]["core"]["xz"][0];
        assert_eq!(entry.ident, "core/xz/5.2.5/20220425103110");
        assert_eq!(entry.origin, "core");
        assert_eq!(entry.name, "xz");
        assert_eq!(entry.version, "5.2.5");
        assert_eq!(entry.release, "20220425103110");
    }

    // The same origin/name can legitimately have a distinct head package per
    // target platform (e.g. a Windows build and a Linux build of the same
    // package name); each target's entries must be kept independent rather
    // than merged/overwritten.
    #[test]
    fn insert_ident_keeps_entries_independent_per_target() {
        let mut set: HashMap<String, HashMap<String, HashMap<String, Vec<PackageFqiEntry>>>> =
            HashMap::new();

        insert_ident(&mut set,
                     "x86_64-linux",
                     "core/openssl/3.2.4/20250428090043");
        insert_ident(&mut set,
                     "x86_64-windows",
                     "core/openssl/1.1.1w/20240108093230");

        assert_eq!(set["x86_64-linux"]["core"]["openssl"].len(), 1);
        assert_eq!(set["x86_64-windows"]["core"]["openssl"].len(), 1);
        assert_eq!(set["x86_64-linux"]["core"]["openssl"][0].ident,
                   "core/openssl/3.2.4/20250428090043");
        assert_eq!(set["x86_64-windows"]["core"]["openssl"][0].ident,
                   "core/openssl/1.1.1w/20240108093230");
    }

    // Builds a ClosureEntry the way helpers::channel_package_closure would:
    // `head_ident` is a head package's own ident, and `tdep_idents` are its
    // recorded tdeps -- all sharing the head package's target and group_key.
    fn closure_group(target: &str,
                     head_ident: &str,
                     tdep_idents: &[&str])
                     -> Vec<helpers::ClosureEntry> {
        let head: BuilderPackageIdent = head_ident.parse().unwrap();
        let group_key = (target.to_string(), format!("{}/{}", head.origin, head.name));
        let mut entries = vec![helpers::ClosureEntry { target:    target.to_string(),
                                                       group_key: group_key.clone(),
                                                       ident:     head,
                                                       is_head:   true, }];
        for tdep in tdep_idents {
            entries.push(helpers::ClosureEntry { target:    target.to_string(),
                                                 group_key: group_key.clone(),
                                                 ident:     tdep.parse().unwrap(),
                                                 is_head:   false, });
        }
        entries
    }

    // Reproduces the original bug report: target's head package
    // (libarchive, untouched by this promotion) still pins an older openssl
    // as a tdep, while the source promotes a newer openssl directly. This
    // is a genuine conflict -- libarchive's own tdep pin doesn't get
    // superseded by a promotion that never touches libarchive itself.
    #[test]
    fn merge_closures_flags_conflict_for_unrelated_head_packages_tdep() {
        let target_closure = closure_group("x86_64-windows",
                                           "core/libarchive/3.5.2/20220425144748",
                                           &["core/openssl/1.1.1l/20220425143501"]);
        let source_closure =
            closure_group("x86_64-windows", "core/openssl/1.1.1w/20240108093230", &[]);

        let by_target = merge_closures_for_conflict_check(&target_closure, &source_closure);
        let conflicts = find_conflicts(&by_target);

        assert!(conflicts["x86_64-windows"].contains_key("core/openssl"));
    }

    // Reproduces the follow-up report: every head package in the target is
    // also being promoted (as a coherent "refresh") from the source, so the
    // target's old head packages -- and, crucially, their own stale tdeps --
    // are all superseded and should contribute nothing. No conflict should
    // be reported.
    #[test]
    fn merge_closures_does_not_flag_conflict_when_source_supersedes_every_target_head() {
        let mut target_closure = closure_group("x86_64-windows",
                                               "core/libarchive/3.5.2/20220425144748",
                                               &["core/openssl/1.1.1l/20220425143501",
                                                 "core/xz/5.2.5/20220425103110",
                                                 "core/zlib/1.2.12/20220425102528"]);
        target_closure.extend(closure_group("x86_64-windows",
                                            "core/openssl/1.1.1l/20220425143501",
                                            &[]));

        let mut source_closure = closure_group("x86_64-windows",
                                               "core/libarchive/3.7.2/20241008044517",
                                               &["core/openssl/1.1.1w/20240108093230",
                                                 "core/xz/5.2.5/20240108063910",
                                                 "core/zlib/1.3/20240108063610"]);
        source_closure.extend(closure_group("x86_64-windows",
                                            "core/openssl/1.1.1w/20240108093230",
                                            &[]));

        let by_target = merge_closures_for_conflict_check(&target_closure, &source_closure);
        let conflicts = find_conflicts(&by_target);

        assert!(conflicts.is_empty());
    }

    // A partial refresh: the source promotes a new libarchive whose tdeps
    // reference a new xz that the source channel itself doesn't otherwise
    // carry as a head package. The target's own (unrelated, un-superseded)
    // xz head package is still older, so this must still be flagged --
    // promoting libarchive alone doesn't make the missing xz update appear.
    #[test]
    fn merge_closures_flags_conflict_for_partial_refresh_missing_dependency_update() {
        let target_closure = closure_group("x86_64-windows", "core/xz/5.2.5/20220425103110", &[]);
        let source_closure = closure_group("x86_64-windows",
                                           "core/libarchive/3.7.2/20241008044517",
                                           &["core/xz/5.2.5/20240108063910"]);

        let by_target = merge_closures_for_conflict_check(&target_closure, &source_closure);
        let conflicts = find_conflicts(&by_target);

        assert!(conflicts["x86_64-windows"].contains_key("core/xz"));
    }

    // Regression: promotion is additive (Channel::promote_packages uses ON
    // CONFLICT DO NOTHING) and post-promotion head selection always picks
    // the highest version/release for a given origin/name/target. So if the
    // *source's* head for some name is actually older than the target's
    // existing head for that same name, the target's head does not get
    // superseded -- it remains the head after promotion, and its own tdeps
    // (not the older source head's) describe the post-promotion state.
    // Merging must reflect that: no conflict here, and the merged closure
    // should carry the target's (newer) libarchive/openssl, not the
    // source's (older) ones.
    #[test]
    fn merge_closures_keeps_newer_target_head_when_source_head_is_older() {
        let target_closure = closure_group("x86_64-windows",
                                           "core/libarchive/3.7.2/20241008044517",
                                           &["core/openssl/1.1.1w/20240108093230"]);
        let source_closure = closure_group("x86_64-windows",
                                           "core/libarchive/3.5.2/20220425144748",
                                           &["core/openssl/1.1.1l/20220425143501"]);

        let by_target = merge_closures_for_conflict_check(&target_closure, &source_closure);
        let conflicts = find_conflicts(&by_target);

        assert!(conflicts.is_empty());
        assert_eq!(by_target["x86_64-windows"]["core/libarchive"],
                   HashSet::from(["core/libarchive/3.7.2/20241008044517".to_string()]));
        assert_eq!(by_target["x86_64-windows"]["core/openssl"],
                   HashSet::from(["core/openssl/1.1.1w/20240108093230".to_string()]));
    }
}
