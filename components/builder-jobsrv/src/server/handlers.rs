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

use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use bldr_core::rpc::RpcMessage;
use hab_net::NetError;

use db::models::package::Package;
use db::models::projects::*;

use diesel;
use protobuf::RepeatedField;
use protocol::jobsrv;
use protocol::net::{self, ErrCode};
use protocol::originsrv;

use super::AppState;

use server::scheduler::ScheduleClient;
use server::worker_manager::WorkerMgrClient;

use error::{Error, Result};
use time::PreciseTime;

pub fn job_get(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGet>()?;

    match state.datastore.get_job(&msg) {
        Ok(Some(ref job)) => RpcMessage::make(job).map_err(Error::BuilderCore),
        Ok(None) => {
            let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-get:1");
            Err(Error::NetError(err))
        }
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "jb:job-get:2");
            error!("{}, {}", err, e);
            Err(Error::NetError(err))
        }
    }
}

pub fn project_jobs_get(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::ProjectJobsGet>()?;
    match state.datastore.get_jobs_for_project(&msg) {
        Ok(ref jobs) => {
            // NOTE: Currently no difference between "project has no jobs" and "no
            // such project"
            RpcMessage::make(jobs).map_err(Error::BuilderCore)
        }
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "jb:project-jobs-get:1");
            error!("{}, {}", err, e);
            Err(Error::NetError(err))
        }
    }
}

pub fn job_log_get(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobLogGet>()?;
    let mut get = jobsrv::JobGet::new();
    get.set_id(msg.get_id());
    let job = match state.datastore.get_job(&get) {
        Ok(Some(job)) => job,
        Ok(None) => {
            let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-log-get:1");
            return Err(Error::NetError(err));
        }
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "jb:job-log-get:2");
            error!("{}, {}", err, e);
            return Err(Error::NetError(err));
        }
    };

    if job.get_is_archived() {
        match state.archiver.retrieve(job.get_id()) {
            Ok(lines) => {
                let start = msg.get_start();
                let num_lines = lines.len() as u64;
                let segment;

                if start > num_lines - 1 {
                    segment = vec![];
                } else {
                    segment = lines[start as usize..].to_vec();
                }

                let mut log = jobsrv::JobLog::new();
                let log_content = RepeatedField::from_vec(segment);

                log.set_start(start);
                log.set_stop(num_lines);
                log.set_is_complete(true); // by definition
                log.set_content(log_content);

                RpcMessage::make(&log).map_err(Error::BuilderCore)
            }
            Err(e @ Error::CaughtPanic(_, _)) => {
                // Generally, this happens when the archiver can't
                // reach it's S3 object store
                warn!("Error retrieving log: {}", e);

                // TODO: Need to return a different error here... it's
                // not quite ENTITY_NOT_FOUND
                let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-log-get:5");
                Err(Error::NetError(err))
            }
            Err(_) => {
                let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-log-get:4");
                Err(Error::NetError(err))
            }
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
            let lines = BufReader::new(file)
                .lines()
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
        Ok(group_opt) => match group_opt {
            Some(group) => group,
            None => {
                let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-group-cancel:1");
                return Err(Error::NetError(err));
            }
        },
        Err(err) => {
            warn!(
                "Failed to get group {} from datastore: {:?}",
                msg.get_group_id(),
                err
            );
            let err = NetError::new(ErrCode::DATA_STORE, "jb:job-group-cancel:2");
            return Err(Error::NetError(err));
        }
    };

    // Set the Group and NotStarted projects to Cancelled
    // TODO (SA): Make the state change code below a single DB call

    state.datastore.cancel_job_group(group.get_id())?;

    // Set all the InProgress projects jobs to CancelPending
    for project in group
        .get_projects()
        .iter()
        .filter(|&ref p| p.get_state() == jobsrv::JobGroupProjectState::InProgress)
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

fn is_project_buildable(state: &AppState, project_name: &str) -> bool {
    let conn = match state.db.get_conn().map_err(Error::Db) {
        Ok(conn_ref) => conn_ref,
        Err(_) => return false,
    };

    match Project::get(project_name, &*conn) {
        Ok(project) => project.auto_build,
        Err(diesel::result::Error::NotFound) => false,
        Err(err) => {
            warn!(
                "Unable to retrieve project: {:?}, error: {:?}",
                project_name, err
            );
            false
        }
    }
}

fn populate_build_projects(
    msg: &jobsrv::JobGroupSpec,
    state: &AppState,
    rdeps: &Vec<(String, String)>,
    projects: &mut Vec<(String, String)>,
) {
    let mut excluded = HashSet::new();
    let mut start_time;
    let mut end_time;

    for s in rdeps {
        // Skip immediately if black-listed
        if excluded.contains(&s.0) {
            continue;
        };

        // If the project is not linked to Builder, or is not auto-buildable
        // then we will skip it, as well as any later projects that depend on it
        // TODO (SA): Move the project list creation/vetting to background thread
        if !is_project_buildable(state, &s.0) {
            debug!(
                "Project is not linked to Builder or not auto-buildable - not adding: {}",
                &s.0
            );
            excluded.insert(s.0.clone());

            let rdeps_opt = {
                let target_graph = state.graph.read().unwrap();
                let graph = target_graph.graph(msg.get_target()).unwrap(); // Unwrap OK
                start_time = PreciseTime::now();
                let ret = graph.rdeps(&s.0);
                end_time = PreciseTime::now();
                ret
            };

            match rdeps_opt {
                Some(rdeps) => {
                    debug!(
                        "Graph rdeps: {} items ({} sec)\n",
                        rdeps.len(),
                        start_time.to(end_time)
                    );

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

        let origin = s.0.split("/").nth(0).unwrap();

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

    // Check that the target is supported - currently only x86_64-linux buildable
    if msg.get_target() != "x86_64-linux" {
        let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-group-create:1");
        return Err(Error::NetError(err));
    }

    let project_name = format!("{}/{}", msg.get_origin(), msg.get_package());
    let mut projects = Vec::new();

    // Get the ident for the root package
    let mut start_time;
    let mut end_time;

    let project_ident = {
        let mut target_graph = state.graph.write().unwrap();
        let graph = match target_graph.graph_mut(msg.get_target()) {
            Some(g) => g,
            None => {
                warn!(
                    "JobGroupSpec, no graph found for target {}",
                    msg.get_target()
                );
                let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-group-create:2");
                return Err(Error::NetError(err));
            }
        };

        start_time = PreciseTime::now();
        let ret = match graph.resolve(&project_name) {
            Some(s) => s,
            None => {
                warn!("JobGroupSpec, project ident not found for {}", project_name);
                // If a package has never been uploaded, we won't see it in the graph
                // Carry on with stiff upper lip
                String::from("")
            }
        };
        end_time = PreciseTime::now();
        ret
    };
    debug!("Resolved project name: {} sec\n", start_time.to(end_time));

    // Bail if auto-build is false, and the project has not been manually kicked off
    if !is_project_buildable(state, &project_name) {
        match msg.get_trigger() {
            jobsrv::JobGroupTrigger::HabClient | jobsrv::JobGroupTrigger::BuilderUI => (),
            _ => {
                let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-group-create:3");
                return Err(Error::NetError(err));
            }
        }
    }

    // Add the root package if needed
    if !msg.get_deps_only() || msg.get_package_only() {
        projects.push((project_name.clone(), project_ident.clone()));
    }

    // Search the packages graph to find the reverse dependencies
    if !msg.get_package_only() {
        let rdeps_opt = {
            let target_graph = state.graph.read().unwrap();
            let graph = target_graph.graph(msg.get_target()).unwrap(); // Unwrap OK
            start_time = PreciseTime::now();
            let ret = graph.rdeps(&project_name);
            end_time = PreciseTime::now();
            ret
        };

        match rdeps_opt {
            Some(rdeps) => {
                debug!(
                    "Graph rdeps: {} items ({} sec)\n",
                    rdeps.len(),
                    start_time.to(end_time)
                );

                populate_build_projects(&msg, state, &rdeps, &mut projects);
            }
            None => {
                debug!("Graph rdeps: no entries found");
            }
        }
    }

    let group = if projects.is_empty() {
        debug!("No projects need building - group is complete");

        let mut new_group = jobsrv::JobGroup::new();
        let projects = RepeatedField::new();
        new_group.set_id(0);
        new_group.set_state(jobsrv::JobGroupState::GroupComplete);
        new_group.set_projects(projects);
        new_group
    } else {
        // If already have a queued job group (queue length: 1 per project),
        // then return that group, else create a new job group
        // TODO (SA) - update the group's projects instead of just returning the group
        let new_group = match state.datastore.get_queued_job_group(&project_name)? {
            Some(group) => {
                debug!("JobGroupSpec, project {} is already queued", project_name);
                group
            }
            None => state.datastore.create_job_group(&msg, projects)?,
        };
        ScheduleClient::default().notify()?;

        // Add audit entry
        let mut jga = jobsrv::JobGroupAudit::new();
        jga.set_group_id(new_group.get_id());
        jga.set_operation(jobsrv::JobGroupOperation::JobGroupOpCreate);
        jga.set_trigger(msg.get_trigger());
        jga.set_requester_id(msg.get_requester_id());
        jga.set_requester_name(msg.get_requester_name().to_string());

        match state.datastore.create_audit_entry(&jga) {
            Ok(_) => (),
            Err(err) => {
                warn!("Failed to create audit entry, err={:?}", err);
            }
        };

        new_group
    };

    RpcMessage::make(&group).map_err(Error::BuilderCore)
}

pub fn job_graph_package_reverse_dependencies_get(
    req: &RpcMessage,
    state: &AppState,
) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGraphPackageReverseDependenciesGet>()?;
    debug!("reverse_dependencies_get message: {:?}", msg);

    let ident = format!("{}/{}", msg.get_origin(), msg.get_name());
    let target_graph = state.graph.read().expect("Graph lock is poisoned");
    let graph = match target_graph.graph(msg.get_target()) {
        Some(g) => g,
        None => {
            warn!(
                "JobGraphPackageReverseDependenciesGet, no graph found for target {}",
                msg.get_target()
            );
            let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:reverse-dependencies-get:1");
            return Err(Error::NetError(err));
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

pub fn job_group_origin_get(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGroupOriginGet>()?;

    match state.datastore.get_job_group_origin(&msg) {
        Ok(ref jgor) => RpcMessage::make(jgor).map_err(Error::BuilderCore),
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "jb:job-group-origin-get:1");
            error!("{}, {}", err, e);
            Err(Error::NetError(err))
        }
    }
}

pub fn job_group_get(req: &RpcMessage, state: &AppState) -> Result<RpcMessage> {
    let msg = req.parse::<jobsrv::JobGroupGet>()?;
    debug!("group_get message: {:?}", msg);

    let group_opt = match state.datastore.get_job_group(&msg) {
        Ok(group_opt) => group_opt,
        Err(err) => {
            warn!(
                "Unable to retrieve group {}, err: {:?}",
                msg.get_group_id(),
                err
            );
            None
        }
    };

    match group_opt {
        Some(group) => RpcMessage::make(&group).map_err(Error::BuilderCore),
        None => {
            let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-group-get:1");
            Err(Error::NetError(err))
        }
    }
}

pub fn job_graph_package_create(req: &Package, state: &AppState) -> Result<Package> {
    let msg = req.parse::<jobsrv::JobGraphPackageCreate>()?;
    let package = msg.get_package();
    // Extend the graph with new package
    let mut target_graph = state.graph.write().unwrap();
    let graph = match target_graph.graph_mut(package.get_target()) {
        Some(g) => g,
        None => {
            warn!(
                "JobGraphPackageCreate, no graph found for target {}",
                package.get_target()
            );
            let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-graph-package-create:1");
            return Err(Error::NetError(err));
        }
    };
    let start_time = PreciseTime::now();
    let (ncount, ecount) = graph.extend(&package);
    let end_time = PreciseTime::now();
    debug!(
        "Extended graph, nodes: {}, edges: {} ({} sec)\n",
        ncount,
        ecount,
        start_time.to(end_time)
    );

    RpcMessage::make(package).map_err(Error::BuilderCore)
}

pub fn job_graph_package_precreate(req: &Package, state: &AppState) -> Result<Package> {
    let msg = req.parse::<jobsrv::JobGraphPackagePreCreate>()?;
    debug!("package_precreate message: {:?}", msg);
    let package: originsrv::OriginPackage = msg.into();

    // Check that we can safely extend the graph with new package
    let can_extend = {
        let mut target_graph = state.graph.write().unwrap();
        let graph = match target_graph.graph_mut(package.get_target()) {
            Some(g) => g,
            None => {
                warn!(
                    "JobGraphPackagePreCreate, no graph found for target {}",
                    package.get_target()
                );
                let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "jb:job-graph-package-pc:1");
                return Err(Error::NetError(err));
            }
        };

        let start_time = PreciseTime::now();
        let ret = graph.check_extend(&package);
        let end_time = PreciseTime::now();

        debug!(
            "Graph pre-check: {} ({} sec)\n",
            ret,
            start_time.to(end_time)
        );

        ret
    };

    if can_extend {
        RpcMessage::make(&net::NetOk::new()).map_err(Error::BuilderCore)
    } else {
        let err = NetError::new(ErrCode::ENTITY_CONFLICT, "jb:job-graph-package-pc:2");
        Err(Error::NetError(err))
    }
}
