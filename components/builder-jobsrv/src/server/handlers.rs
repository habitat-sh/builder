// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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

//! A collection of handlers for the JobSrv dispatcher

use std::{collections::{HashMap,
                        HashSet},
          fs::OpenOptions,
          io::{BufRead,
               BufReader},
          path::PathBuf,
          str::FromStr,
          time::Instant};

use diesel::{self,
             result::Error::NotFound,
             PgConnection};

use protobuf::RepeatedField;

use futures03::executor::block_on;

use crate::{bldr_core::rpc::RpcMessage,
            db::models::{channel::{Channel,
                                   CreateChannel},
                         jobs::*,
                         package::*,
                         projects::*},
            hab_core::{package::{PackageIdent,
                                 PackageTarget},
                       ChannelIdent}};

use super::AppState;
use crate::protocol::{jobsrv,
                      net,
                      originsrv};

use crate::builder_graph::{data_store::DataStore as GraphDataStore,
                           package_build_manifest_graph::PackageBuildManifest,
                           package_ident_intern::PackageIdentIntern};

use crate::{scheduler_datastore::SchedulerDataStoreDb,
            server::{feat,
                     scheduler::ScheduleClient,
                     worker_manager::WorkerMgrClient}};

use crate::{data_store::DataStore,
            scheduler_datastore::GroupId};

use crate::error::{Error,
                   Result};

pub fn job_get(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGet>()?;

    match state.datastore.get_job(&msg) {
        Ok(Some(ref job)) => RpcMessage::make(job).map_err(Error::BuilderCore),
        Ok(None) => Err(Error::NotFound),
        Err(e) => {
            warn!("job_get error: {:?}", e);
            Err(Error::System)
        }
    }
}

pub fn job_log_get(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobLogGet>()?;
    let mut get = jobsrv::JobGet::new();
    get.set_id(msg.get_id());
    let job = match state.datastore.get_job(&get) {
        Ok(Some(job)) => job,
        Ok(None) => return Err(Error::NotFound),
        Err(e) => {
            warn!("job_log_get error: {:?}", e);
            return Err(Error::System);
        }
    };

    if job.get_is_archived() {
        match state.archiver.retrieve(job.get_id()) {
            Ok(lines) => {
                let start = msg.get_start();
                let num_lines = lines.len() as u64;
                let segment = if start > num_lines - 1 {
                    vec![]
                } else {
                    lines[start as usize..].to_vec()
                };

                let mut log = jobsrv::JobLog::new();
                let log_content = RepeatedField::from_vec(segment);

                log.set_start(start);
                log.set_stop(num_lines);
                log.set_is_complete(true); // by definition
                log.set_content(log_content);

                RpcMessage::make(&log).map_err(Error::BuilderCore)
            }
            Err(e @ Error::CaughtPanic(..)) => {
                // Generally, this happens when the archiver can't
                // reach it's S3 object store
                warn!("Error retrieving log: {}", e);

                // TODO: Need to return a different error here... it's
                // not quite ENTITY_NOT_FOUND
                Err(Error::NotFound)
            }
            Err(_) => Err(Error::NotFound),
        }
    } else {
        // retrieve fragment from on-disk file
        let start = msg.get_start();
        let file = state.log_dir.log_file_path(msg.get_id());

        match get_log_content(&file, start) {
            Some(content) => {
                let num_lines = content.len() as u64;
                let mut log = jobsrv::JobLog::new();
                log.set_start(start);
                log.set_content(RepeatedField::from_vec(content));
                log.set_stop(start + num_lines);
                log.set_is_complete(false);
                RpcMessage::make(&log).map_err(Error::BuilderCore)
            }
            None => {
                // The job exists, but there are no logs (either yet, or ever).
                // Just return an empty job log
                let log = jobsrv::JobLog::new();
                RpcMessage::make(&log).map_err(Error::BuilderCore)
            }
        }
    }
}

/// Returns the lines of the log file past `offset`.
///
/// If the file does not exist, `None` is returned; this could be
/// because there is not yet any log information for the job, or the
/// job never had any log information (e.g., it predates this
/// feature).
fn get_log_content(log_file: &PathBuf, offset: u64) -> Option<Vec<String>> {
    match OpenOptions::new().read(true).open(log_file) {
        Ok(file) => {
            let lines = BufReader::new(file).lines()
                                            .skip(offset as usize)
                                            .map(|l| l.expect("Could not parse line"))
                                            .collect();
            Some(lines)
        }
        Err(e) => {
            warn!("Couldn't open log file {:?}: {:?}", log_file, e);
            None
        }
    }
}

pub fn job_group_cancel(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGroupCancel>()?;
    debug!("job_group_cancel message: {:?}", msg);

    // Get the job group
    let mut jgc = jobsrv::JobGroupGet::new();
    jgc.set_group_id(msg.get_group_id());
    jgc.set_include_projects(true);

    let group = match state.datastore.get_job_group(&jgc) {
        Ok(group_opt) => {
            match group_opt {
                Some(group) => group,
                None => return Err(Error::NotFound),
            }
        }
        Err(err) => {
            warn!("Failed to get group {} from datastore: {:?}",
                  msg.get_group_id(),
                  err);
            return Err(Error::System);
        }
    };

    // Set the Group and NotStarted projects to Cancelled
    // TODO (SA): Make the state change code below a single DB call

    state.datastore.cancel_job_group(group.get_id())?;

    // Set all the InProgress projects jobs to CancelPending
    for project in group.get_projects()
                        .iter()
                        .filter(|p| p.get_state() == jobsrv::JobGroupProjectState::InProgress)
    {
        let job_id = project.get_job_id();
        let mut req = jobsrv::JobGet::new();
        req.set_id(job_id);

        match state.datastore.get_job(&req)? {
            Some(mut job) => {
                debug!("Canceling job {:?}", job_id);
                job.set_state(jobsrv::JobState::CancelPending);
                state.datastore.update_job(&job)?;
            }
            None => {
                warn!("Unable to cancel job {:?} (not found)", job_id,);
            }
        }
    }

    // Add audit entry
    let mut jga = jobsrv::JobGroupAudit::new();
    jga.set_group_id(group.get_id());
    jga.set_operation(jobsrv::JobGroupOperation::JobGroupOpCancel);
    jga.set_trigger(msg.get_trigger());
    jga.set_requester_id(msg.get_requester_id());
    jga.set_requester_name(msg.get_requester_name().to_string());

    match state.datastore.create_audit_entry(&jga) {
        Ok(_) => (),
        Err(err) => {
            warn!("Failed to create audit entry, err={:?}", err);
        }
    };

    WorkerMgrClient::default().notify_work()?;
    RpcMessage::make(&net::NetOk::new()).map_err(Error::BuilderCore)
}

fn is_project_buildable(state: &AppState, project_name: &str, target: &str) -> bool {
    let conn = match state.db.get_conn().map_err(Error::Db) {
        Ok(conn_ref) => conn_ref,
        Err(_) => return false,
    };

    let target = if feat::is_enabled(feat::LegacyProject) {
        "x86_64-linux"
    } else {
        target
    };

    match Project::get(project_name, &target, &*conn) {
        Ok(project) => project.auto_build,
        Err(diesel::result::Error::NotFound) => false,
        Err(err) => {
            warn!("Unable to retrieve project: {:?}, error: {:?}",
                  project_name, err);
            false
        }
    }
}

fn populate_build_projects(msg: &jobsrv::JobGroupSpec,
                           state: &AppState,
                           rdeps: &[(String, String)],
                           projects: &mut Vec<(String, String)>) {
    let mut excluded = HashSet::new();
    let mut start_time;

    for s in rdeps {
        // Skip immediately if black-listed
        if excluded.contains(&s.0) {
            continue;
        };

        // If the project is not linked to Builder, or is not auto-buildable
        // then we will skip it, as well as any later projects that depend on it
        // TODO (SA): Move the project list creation/vetting to background thread
        if !is_project_buildable(state, &s.0, &msg.get_target()) {
            debug!("Project is not linked to Builder or not auto-buildable - not adding: {}",
                   &s.0);
            excluded.insert(s.0.clone());

            let rdeps_opt = {
                let target_graph = state.graph.read().unwrap();
                let graph = target_graph.graph(msg.get_target()).unwrap(); // Unwrap OK
                start_time = Instant::now();
                graph.rdeps(&s.0)
            };

            match rdeps_opt {
                Some(rdeps) => {
                    debug!("Graph rdeps: {} items ({} sec)\n",
                           rdeps.len(),
                           start_time.elapsed().as_secs_f64());
                    for dep in rdeps {
                        excluded.insert(dep.0.clone());
                    }
                }
                None => {
                    debug!("Graph rdeps: no entries found");
                }
            }

            continue;
        };

        let origin = s.0.split('/').next().unwrap();

        // If the origin_only flag is true, make sure the origin matches
        if !msg.get_origin_only() || origin == msg.get_origin() {
            debug!("Adding to projects: {} ({})", s.0, s.1);
            projects.push(s.clone());
        } else {
            debug!("Skipping non-origin project: {} ({})", s.0, s.1);
        }
    }
}

pub fn job_group_create(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGroupSpec>()?;
    debug!("job_group_create message: {:?}", msg);

    // Check that the target is supported
    let target = match PackageTarget::from_str(msg.get_target()) {
        Ok(t) => t,
        Err(_) => {
            debug!("Invalid package target: {:?}", msg.get_target());
            return Err(Error::NotFound);
        }
    };

    if !state.build_targets.contains(&target) {
        debug!("Rejecting build request with target: {:?}", target);
        return Err(Error::NotFound);
    }

    let group = if feat::is_enabled(feat::NewScheduler) {
        job_group_create_new(&msg, target, &state)?
    } else {
        job_group_create_old(&msg, target, &state)?
    };
    RpcMessage::make(&group).map_err(Error::BuilderCore)
}

fn job_group_create_old(msg: &jobsrv::JobGroupSpec,
                        target: PackageTarget,
                        state: &AppState)
                        -> Result<jobsrv::JobGroup> {
    let project_name = format!("{}/{}", msg.get_origin(), msg.get_package());
    let mut projects = Vec::new();

    // Get the ident for the root package
    let mut start_time;

    let project_ident = {
        let mut target_graph = state.graph.write().unwrap();
        let graph = match target_graph.graph_mut(msg.get_target()) {
            Some(g) => g,
            None => {
                warn!("JobGroupSpec, no graph found for target {}",
                      msg.get_target());
                return Err(Error::NotFound);
            }
        };

        start_time = Instant::now();
        let ret = match graph.resolve(&project_name) {
            Some(s) => s,
            None => {
                warn!("JobGroupSpec, project ident not found for {}", project_name);
                // If a package has never been uploaded, we won't see it in the graph
                // Carry on with stiff upper lip
                String::from("")
            }
        };
        debug!("Resolved project name: {} sec\n",
               start_time.elapsed().as_secs_f64());
        ret
    };

    // Bail if auto-build is false, and the project has not been manually kicked off
    if !is_project_buildable(state, &project_name, &target) {
        match msg.get_trigger() {
            jobsrv::JobGroupTrigger::HabClient | jobsrv::JobGroupTrigger::BuilderUI => (),
            _ => {
                return Err(Error::NotFound);
            }
        }
    }

    // Add the root package if needed
    if !msg.get_deps_only() || msg.get_package_only() {
        projects.push((project_name.clone(), project_ident));
    }
    // Search the packages graph to find the reverse dependencies
    if !msg.get_package_only() {
        let rdeps_opt = {
            let target_graph = state.graph.read().unwrap();
            let graph = target_graph.graph(msg.get_target()).unwrap(); // Unwrap OK
            start_time = Instant::now();
            graph.rdeps(&project_name)
        };

        match rdeps_opt {
            Some(rdeps) => {
                debug!("Graph rdeps: {} items ({} sec)\n",
                       rdeps.len(),
                       start_time.elapsed().as_secs_f64());
                populate_build_projects(&msg, state, &rdeps, &mut projects);
            }
            None => {
                debug!("Graph rdeps: no entries found");
            }
        }
    }

    if projects.is_empty() {
        debug!("No projects need building - group is complete");

        let mut new_group = jobsrv::JobGroup::new();
        let projects = RepeatedField::new();
        new_group.set_id(0);
        new_group.set_state(jobsrv::JobGroupState::GroupComplete);
        new_group.set_projects(projects);
        new_group.set_target(msg.get_target().to_string());
        Ok(new_group)
    } else {
        // If already have a queued job group (queue length: 1 per project and target),
        // then return that group, else create a new job group
        // TODO (SA) - update the group's projects instead of just returning the group
        let conn = state.db.get_conn().map_err(Error::Db)?;

        let new_group = match Group::get_queued(&project_name, &msg.get_target(), &*conn) {
            Ok(group) => {
                debug!("JobGroupSpec, project {} is already queued", project_name);
                group.into()
            }
            Err(NotFound) => state.datastore.create_job_group(&msg, projects)?,
            Err(err) => {
                debug!("Failed to retrieve queued groups, err = {}", err);
                return Err(Error::DieselError(err));
            }
        };
        ScheduleClient::default().notify()?;

        add_job_group_audit_entry(new_group.get_id(), &msg, &state.datastore);

        Ok(new_group)
    }
}

fn job_group_create_new(msg: &jobsrv::JobGroupSpec,
                        target: PackageTarget,
                        state: &AppState)
                        -> Result<jobsrv::JobGroup> {
    let project_name = format!("{}/{}", msg.get_origin(), msg.get_package());

    // This may be slightly redundant with the work done building the manifest, but
    // leaving this for now.
    // Bail if auto-build is false, and the project has not been manually kicked off
    if !is_project_buildable(state, &project_name, &target) {
        match msg.get_trigger() {
            jobsrv::JobGroupTrigger::HabClient | jobsrv::JobGroupTrigger::BuilderUI => (),
            _ => {
                return Err(Error::NotFound);
            }
        }
    }

    // Find/create the group
    // There are several options around what we do if there is already a group for this package
    // 1) just return the existing queued build group (previous behavior)
    // 2) cancel the old group and replace it with a new one
    // 3) do nothing and notify the user to cancel if they want to
    // 4) create a new group and have possibly redundant builds
    //
    // This doesn't really take into account possible changes in the deps_only and package_only
    // flags but probably should.
    // For now we will do 4) and create the group no matter what.

    let conn = state.db.get_conn().map_err(Error::Db)?;

    let new_group = NewGroup { group_state:  "Queued",
                               project_name: &project_name,
                               target:       &target.to_string(), };
    let group = Group::create(&new_group, &conn)?;

    {
        // the code in this block might be best moved to some sort of asynchronous task, maybe
        // even another thread.
        info!("Generating Manifest");
        let mut manifest = if msg.get_package_only() {
            // we only build the package itself.
            info!("Empty Manifest");
            PackageBuildManifest::new()
        } else {
            info!("Including deps in Manifest");
            // !(!msg.get_deps_only() || msg.get_package_only())
            let _exclude_root = msg.get_deps_only() && !msg.get_package_only();

            // The exclude root feature is currently unimplemented, because we don't believe it's
            // used.
            let target_graph =
                state.graph
                     .read()
                     .expect("Graph lock could not be acquired because it was poisoned");
            let graph = target_graph.graph_for_target(target)
                                    .expect("No graph for target");
            let package =
                PackageIdentIntern::from_str(&project_name).expect("Could not parse \
                                                                    project_name, which is odd \
                                                                    because we control the format");
            let graph_datastore = GraphDataStore::from_pool(state.db.clone())?;
            // NOTE: We only use the  Unbuildable trait from graph_datastore. This partitioning of
            // the trait is an artifact of how we managed the transition from the old code to new.
            // Once we get to the point where we can get rid of the old scheduler and
            // clean up the datastore layer, we can eliminate this.
            graph.compute_build(&[package], &graph_datastore)?
        };

        // This can be removed once we get a worker API that lets us exactly specify the
        // dependencies. Without that the worker takes whatever is latest in the channel,
        // which under a loose ordering might be newer than what we want.
        manifest.constrain_package_cycles();

        // We would like to have dbg!(&manifest) here but it is very verbose for normal operation.
        // Perhaps we should make a separate API/path to allow this to be discovered
        insert_job_graph_entries(&manifest,
                                 group.id as i64,
                                 BuilderPackageTarget(target),
                                 &conn)?;

        // Notify the scheduler of new work available
        let mut scheduler = state.scheduler
                                 .clone()
                                 .expect("Unable to get valid scheduler to talk to");
        block_on(scheduler.job_group_added(GroupId(group.id), BuilderPackageTarget(target)));
    }

    add_job_group_audit_entry(group.id as u64, &msg, &state.datastore);
    Ok(group.into())
}

/// Start a build based on the results of a previous build group.
/// The message specifies:
/// * the id of a previous build group
/// * a list of plans to use as a seed (can be empty)
/// * if add_failed_packages is true includes any failed packages from the previous build
///
/// Takes information on origin, target, and build scope from previous build.
pub fn job_group_rebuild(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGroupRebuildFromSpec>()?;
    debug!("job_group_rebuild message: {:?}", msg);

    let group = if feat::is_enabled(feat::NewScheduler) {
        job_group_rebuild_new(&msg, state)?
    } else {
        Err(Error::UnsupportedFeature("Rebuild not supported on \
                                       legacy scheduler"
                                                        .to_owned()))?
    };

    RpcMessage::make(&group).map_err(Error::BuilderCore)
}

fn job_group_rebuild_new(msg: &jobsrv::JobGroupRebuildFromSpec,
                         state: &AppState)
                         -> Result<jobsrv::JobGroup> {
    let conn = state.db.get_conn().map_err(Error::Db)?;
    // Get previous group
    let old_group_data = Group::get(msg.get_job_group_id() as i64, &conn)?;

    // TODO safety checks; job should be complete/canceled etc

    let target = PackageTarget::from_str(&old_group_data.target)?;
    // Create new group from it
    let new_group = NewGroup { group_state:  "Queued",
                               project_name: &old_group_data.project_name.clone(), /* should this be
                                                                                    * somehow different
                                                                                    * or carry more
                                                                                    * info? */
                               target:       &old_group_data.target, };
    let new_group_data = Group::create(&new_group, &conn)?;

    let old_channel = Channel::channel_for_group(old_group_data.id as u64);
    let old_channel_data =
        Channel::get(&msg.get_origin(), &ChannelIdent::from(old_channel), &conn)?;

    let new_channel = Channel::channel_for_group(new_group_data.id as u64);
    let new_channel_data = Channel::create(&CreateChannel { name:     &new_channel.to_string(),
                                                            owner_id: msg.get_requester_id() as i64,
                                                            origin:   msg.get_origin(), },
                                           &conn)?;
    Channel::do_promote_or_demote_packages_cross_channels(old_channel_data.id,
                                                          new_channel_data.id,
                                                          true,
                                                          &conn)?;

    // Expand any provided plans
    let plans: std::result::Result<Vec<PackageIdentIntern>, habitat_core::error::Error> =
        msg.get_packages()
           .iter()
           .map(|plan| PackageIdentIntern::from_str(plan))
           .collect();

    let mut plans: Vec<PackageIdentIntern> = plans?;

    // Fetch failed plans from previous group
    // Maybe we also want to include cancelled jobs here. DependencyFailed will be found by
    // transitive property during manifest expansion.
    let entries =
        JobGraphEntry::list_group_by_state(old_group_data.id, JobExecState::JobFailed, &conn)?;
    let mut failed_plans: Vec<PackageIdentIntern> =
        entries.iter()
               .map(|e| PackageIdentIntern::from_str(&e.project_name))
               .collect::<std::result::Result<Vec<PackageIdentIntern>, habitat_core::error::Error>>()?;

    plans.append(&mut failed_plans);

    let manifest = make_manifest_from_plans(&plans, target, state)?;

    // We would like to have dbg!(&manifest) here but it is very verbose for normal operation.
    // Perhaps we should make a separate API/path to allow this to be discovered
    insert_job_graph_entries(&manifest,
                             new_group_data.id as i64,
                             BuilderPackageTarget(target),
                             &conn)?;

    // Notify the scheduler of new work available
    let mut scheduler = state.scheduler
                             .clone()
                             .expect("Unable to get valid scheduler to talk to");
    block_on(scheduler.job_group_added(GroupId(new_group_data.id), BuilderPackageTarget(target)));

    Ok(new_group_data.into())
}

/// Make a full manifest of plans to build
fn make_manifest_from_plans(plans: &Vec<PackageIdentIntern>,
                            target: PackageTarget,
                            state: &AppState)
                            -> Result<PackageBuildManifest> {
    // Compute plans to build using this set
    // This might be a slow process and need to be async/threaded

    let target_graph = state.graph
                            .read()
                            .expect("Graph lock could not be acquired because it was poisoned");
    let graph = target_graph.graph_for_target(target)
                            .expect("No graph for target");

    let graph_datastore = GraphDataStore::from_pool(state.db.clone())?;
    // NOTE: We only use the  Unbuildable trait from graph_datastore. This partitioning of
    // the trait is an artifact of how we managed the transition from the old code to new.
    // Once we get to the point where we can get rid of the old scheduler and
    // clean up the datastore layer, we can eliminate this.
    let mut manifest = graph.compute_build(&plans, &graph_datastore)?;

    // This can be removed once we get a worker API that lets us exactly specify the
    // dependencies. Without that the worker takes whatever is latest in the channel,
    // which under a loose ordering might be newer than what we want.
    manifest.constrain_package_cycles();

    Ok(manifest)
}

fn insert_job_graph_entries(manifest: &PackageBuildManifest,
                            group_id: i64,
                            target: BuilderPackageTarget,
                            conn: &PgConnection)
                            -> Result<()> {
    info!("Inserting entries");
    let order = manifest.build_order();
    info!("Got {} entries", order.len());
    let mut lookup = HashMap::new();

    for package in order {
        let project_name = package.name.ident();
        let manifest_ident = package.name.to_unbuilt_ident();

        info!("Rendered job_graph_entry for project {} package {} for group {}",
              project_name, manifest_ident, group_id);

        let mut dependency_ids: Vec<i64> = Vec::new();
        // TODO: figure out if we should be using natural_deps or all_deps
        for dependency in package.natural_deps() {
            if dependency.is_internal_node() {
                let dep_id = lookup[dependency];
                dependency_ids.push(dep_id);
            }
        }

        let project_name = project_name.to_string();
        let manifest_name = manifest_ident.to_string();
        let entry = NewJobGraphEntry::new(group_id,
                                          &project_name,
                                          &manifest_name,
                                          JobExecState::Pending,
                                          &dependency_ids,
                                          target);
        let entry = JobGraphEntry::create(&entry, &conn)?;

        lookup.insert(package.name, entry.id);
        // TODO: Should we error if we get Some(id) back?
    }
    Ok(())
}

fn add_job_group_audit_entry(group_id: u64, msg: &jobsrv::JobGroupSpec, datastore: &DataStore) {
    // Add audit entry
    let mut jga = jobsrv::JobGroupAudit::new();
    jga.set_group_id(group_id);
    jga.set_operation(jobsrv::JobGroupOperation::JobGroupOpCreate);
    jga.set_trigger(msg.get_trigger());
    jga.set_requester_id(msg.get_requester_id());
    jga.set_requester_name(msg.get_requester_name().to_string());

    match datastore.create_audit_entry(&jga) {
        Ok(_) => (),
        Err(err) => {
            warn!("Failed to create audit entry, err={:?}", err);
        }
    };
}

pub fn job_graph_package_reverse_dependencies_get(req: &RpcMessage,
                                                  state: &AppState)
                                                  -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGraphPackageReverseDependenciesGet>()?;
    debug!("reverse_dependencies_get message: {:?}", msg);

    let ident = format!("{}/{}", msg.get_origin(), msg.get_name());
    let target_graph = state.graph.read().expect("Graph lock is poisoned");
    let graph = match target_graph.graph(msg.get_target()) {
        Some(g) => g,
        None => {
            warn!("JobGraphPackageReverseDependenciesGet, no graph found for target {}",
                  msg.get_target());
            return Err(Error::NotFound);
        }
    };

    let rdeps = graph.rdeps(&ident);
    let mut rd_reply = jobsrv::JobGraphPackageReverseDependencies::new();
    rd_reply.set_origin(msg.get_origin().to_string());
    rd_reply.set_name(msg.get_name().to_string());

    match rdeps {
        Some(rd) => {
            let mut short_deps = RepeatedField::new();

            // the tuples inside rd are of the form: (core/redis, core/redis/3.2.4/20170717232232)
            // we're only interested in the short form, not the fully qualified form
            for (id, _fully_qualified_id) in rd {
                short_deps.push(id);
            }

            short_deps.sort();
            rd_reply.set_rdeps(short_deps);
        }
        None => debug!("No rdeps found for {}", ident),
    }

    RpcMessage::make(&rd_reply).map_err(Error::BuilderCore)
}

pub fn job_graph_package_reverse_dependencies_grouped_get(req: &RpcMessage,
                                                          state: &AppState)
                                                          -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGraphPackageReverseDependenciesGroupedGet>()?;
    debug!("reverse_dependencies_grouped_get message: {:?}", msg);

    let ident = format!("{}/{}", msg.get_origin(), msg.get_name());
    let target_graph = state.graph.read().expect("Graph lock is poisoned");
    let graph = match target_graph.graph(msg.get_target()) {
        Some(g) => g,
        None => {
            warn!("JobGraphPackageReverseDependenciesGroupedGet, no graph found for target {}",
                  msg.get_target());
            return Err(Error::NotFound);
        }
    };

    let rdeps = graph.rdeps(&ident);
    let mut rd_reply = jobsrv::JobGraphPackageReverseDependenciesGrouped::new();
    rd_reply.set_origin(msg.get_origin().to_string());
    rd_reply.set_name(msg.get_name().to_string());

    match rdeps {
        Some(rd) => {
            let rdeps = if rd.is_empty() {
                RepeatedField::new()
            } else {
                let rdeps = compute_rdep_build_groups(state, &ident, &msg.get_target(), &rd)?;
                RepeatedField::from_vec(rdeps)
            };
            rd_reply.set_rdeps(rdeps);
        }
        None => debug!("No rdeps found for {}", ident),
    }

    RpcMessage::make(&rd_reply).map_err(Error::BuilderCore)
}

fn compute_rdep_build_groups(state: &AppState,
                             root_ident: &str,
                             target: &str,
                             rdeps: &[(String, String)])
                             -> Result<Vec<jobsrv::JobGraphPackageReverseDependencyGroup>> {
    let mut rdep_groups = Vec::new();
    let mut in_progress = Vec::new();
    let mut satisfied_deps = HashSet::new();
    let mut group_num = 0;

    debug!("computing redep build groups for: {}", root_ident);

    let conn = state.db.get_conn().map_err(Error::Db)?;

    satisfied_deps.insert(root_ident.to_owned());
    assert!(!rdeps.is_empty());
    in_progress.push(rdeps[0].0.to_owned());
    trace!("Adding ident to in_progress: {} (group 0)", rdeps[0].0);

    for ix in 1..rdeps.len() {
        let package = Package::get(
            GetPackage {
                ident: BuilderPackageIdent(PackageIdent::from_str(&rdeps[ix].1.clone())?),
                visibility: vec![
                    PackageVisibility::Public,
                    PackageVisibility::Private,
                    PackageVisibility::Hidden,
                ],
                target: BuilderPackageTarget(PackageTarget::from_str(target)?),
            },
            &*conn,
        )?;

        let deps = package.deps;
        let mut can_dispatch = true;
        for dep in deps {
            let name = format!("{}/{}", dep.origin, dep.name);
            if (rdeps.iter().any(|s| s.0 == name)) && !satisfied_deps.contains(&name) {
                can_dispatch = false;
                break;
            }
        }

        if !can_dispatch {
            trace!("Ending group {}", group_num);
            let mut rdep_group = jobsrv::JobGraphPackageReverseDependencyGroup::new();
            rdep_group.set_group_id(group_num);
            rdep_group.set_idents(RepeatedField::from_vec(in_progress.clone()));
            rdep_groups.push(rdep_group);
            in_progress.iter().for_each(|s| {
                                  trace!("Adding to satisfied deps: {}", s);
                                  satisfied_deps.insert(s.to_owned());
                              });
            in_progress.clear();
            group_num += 1;
        }

        in_progress.push(rdeps[ix].0.to_owned());
        trace!("Pushing ident to in_progress: {} (group {})",
               rdeps[ix].0,
               group_num);
    }

    if !in_progress.is_empty() {
        let mut rdep_group = jobsrv::JobGraphPackageReverseDependencyGroup::new();
        rdep_group.set_group_id(group_num);
        rdep_group.set_idents(RepeatedField::from_vec(in_progress));
        rdep_groups.push(rdep_group);
    }

    Ok(rdep_groups)
}

pub fn job_group_origin_get(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGroupOriginGet>()?;

    match state.datastore.get_job_group_origin(&msg) {
        Ok(ref jgor) => RpcMessage::make(jgor).map_err(Error::BuilderCore),
        Err(e) => {
            warn!("job_group_origin_get error: {:?}", e);
            Err(Error::System)
        }
    }
}

pub fn job_group_get(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGroupGet>()?;
    debug!("group_get message: {:?}", msg);

    let maybe_group = if state.scheduler.is_some() {
        // We might want to fall back to the old code if we're in an env migrated from the old
        // scheduler
        let scheduler_datastore = SchedulerDataStoreDb::new(state.datastore.clone());
        scheduler_datastore.get_job_group(msg.get_group_id() as i64, msg.get_include_projects())
    } else {
        state.datastore.get_job_group(&msg)
    };

    let group_opt = match maybe_group {
        Ok(group_opt) => group_opt,
        Err(err) => {
            warn!("Unable to retrieve group {}, err: {:?}",
                  msg.get_group_id(),
                  err);
            None
        }
    };

    match group_opt {
        Some(group) => RpcMessage::make(&group).map_err(Error::BuilderCore),
        None => Err(Error::NotFound),
    }
}

pub fn job_graph_package_create(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGraphPackageCreate>()?;
    let package = msg.get_package();
    // Extend the graph with new package
    let mut target_graph = state.graph.write().unwrap();
    let graph = match target_graph.graph_mut(package.get_target()) {
        Some(g) => g,
        None => {
            warn!("JobGraphPackageCreate, no graph found for target {}",
                  package.get_target());
            return Err(Error::NotFound);
        }
    };
    let start_time = Instant::now();
    let (ncount, ecount) = graph.extend(&package, feat::is_enabled(feat::BuildDeps));
    debug!("Extended graph, nodes: {}, edges: {} ({} sec)\n",
           ncount,
           ecount,
           start_time.elapsed().as_secs_f64());

    RpcMessage::make(package).map_err(Error::BuilderCore)
}

pub fn job_graph_package_precreate(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGraphPackagePreCreate>()?;
    debug!("package_precreate message: {:?}", msg);
    let package: originsrv::OriginPackage = msg.into();

    // Check that we can safely extend the graph with new package
    let can_extend = {
        let mut target_graph = state.graph.write().unwrap();
        let graph = match target_graph.graph_mut(package.get_target()) {
            Some(g) => g,
            None => {
                warn!("JobGraphPackagePreCreate, no graph found for target {}",
                      package.get_target());
                return Err(Error::NotFound);
            }
        };

        let start_time = Instant::now();

        let ret = graph.check_extend(&package, feat::is_enabled(feat::BuildDeps));

        debug!("Graph pre-check: {} ({} sec)\n",
               ret,
               start_time.elapsed().as_secs_f64());

        ret
    };

    if can_extend {
        RpcMessage::make(&net::NetOk::new()).map_err(Error::BuilderCore)
    } else {
        Err(Error::Conflict)
    }
}
