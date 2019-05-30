// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

mod handlers;
pub mod log_archiver;
mod log_directory;
mod log_ingester;
mod metrics;
mod scheduler;
mod worker_manager;

use std::{collections::{HashMap,
                        HashSet},
          iter::{FromIterator,
                 Iterator},
          sync::{Arc,
                 RwLock}};
use time::PreciseTime;

use actix;
use actix_web::{http::{Method,
                       StatusCode},
                middleware::Logger,
                server::{self,
                         KeepAlive},
                App,
                HttpRequest,
                HttpResponse,
                Json};

use crate::{bldr_core::{rpc::RpcMessage,
                        target_graph::TargetGraph},
            db::{models::package::*,
                 DbPool},
            hab_core::package::PackageTarget,
            protocol::originsrv::OriginPackage};

use self::{log_archiver::LogArchiver,
           log_directory::LogDirectory,
           log_ingester::LogIngester,
           scheduler::ScheduleMgr,
           worker_manager::WorkerMgr};

use crate::{config::{Config,
                     GatewayCfg},
            data_store::DataStore,
            error::Result};

features! {
    pub mod feat {
        const BuildDeps = 0b0000_0001
    }
}

// Application state
pub struct AppState {
    archiver:      Box<LogArchiver>,
    datastore:     DataStore,
    db:            DbPool,
    graph:         Arc<RwLock<TargetGraph>>,
    log_dir:       LogDirectory,
    build_targets: HashSet<PackageTarget>,
}

impl AppState {
    pub fn new(cfg: &Config,
               datastore: &DataStore,
               db: DbPool,
               graph: &Arc<RwLock<TargetGraph>>)
               -> Self {
        AppState { archiver: log_archiver::from_config(&cfg.archive).unwrap(),
                   datastore: datastore.clone(),
                   db,
                   graph: graph.clone(),
                   log_dir: LogDirectory::new(&cfg.log_dir),
                   build_targets: cfg.build_targets.clone() }
    }
}

/// Endpoint for determining availability of builder-jobsrv components.
///
/// Returns a status 200 on success. Any non-200 responses are an outage or a partial outage.
fn status(_req: &HttpRequest<AppState>) -> HttpResponse { HttpResponse::new(StatusCode::OK) }

fn handle_rpc((req, msg): (HttpRequest<AppState>, Json<RpcMessage>)) -> HttpResponse {
    debug!("Got RPC message, body =\n{:?}", msg);

    let result = match msg.id.as_str() {
        "JobGet" => handlers::job_get(&msg, req.state()),
        "JobLogGet" => handlers::job_log_get(&msg, req.state()),
        "JobGroupSpec" => handlers::job_group_create(&msg, req.state()),
        "JobGroupCancel" => handlers::job_group_cancel(&msg, req.state()),
        "JobGroupGet" => handlers::job_group_get(&msg, req.state()),
        "JobGroupOriginGet" => handlers::job_group_origin_get(&msg, req.state()),
        "JobGraphPackageCreate" => handlers::job_graph_package_create(&msg, req.state()),
        "JobGraphPackagePreCreate" => handlers::job_graph_package_precreate(&msg, req.state()),
        "JobGraphPackageReverseDependenciesGet" => {
            handlers::job_graph_package_reverse_dependencies_get(&msg, req.state())
        }
        "JobGraphPackageReverseDependenciesGroupedGet" => {
            handlers::job_graph_package_reverse_dependencies_grouped_get(&msg, req.state())
        }

        _ => {
            let err = format!("Unknown RPC message received: {}", msg.id);
            error!("{}", err);
            return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR, err);
        }
    };

    match result {
        Ok(m) => HttpResponse::Ok().json(m),
        Err(e) => e.into(),
    }
}

fn enable_features_from_config(cfg: &Config) {
    let features: HashMap<_, _> = HashMap::from_iter(vec![("BUILDDEPS", feat::BuildDeps)]);
    let features_enabled = cfg.features_enabled
                              .split(',')
                              .map(|f| f.trim().to_uppercase());

    for key in features_enabled {
        if features.contains_key(key.as_str()) {
            info!("Enabling feature: {}", key);
            feat::enable(features[key.as_str()]);
        }
    }

    if feat::is_enabled(feat::BuildDeps) {
        println!("Listing possible feature flags: {:?}", features.keys());
        println!("Enable features by populating 'features_enabled' in config");
    }
}

pub fn run(config: Config) -> Result<()> {
    let sys = actix::System::new("builder-jobsrv");
    let cfg = Arc::new(config.clone());

    enable_features_from_config(&config);

    let datastore = DataStore::new(&config.datastore);
    let db_pool = DbPool::new(&config.datastore.clone());
    let mut graph = TargetGraph::new();
    let pkg_conn = &db_pool.get_conn()?;
    let packages = Package::get_all_latest(&pkg_conn)?;
    let origin_packages: Vec<OriginPackage> = packages.iter().map(|p| p.clone().into()).collect();
    let start_time = PreciseTime::now();
    let res = graph.build(origin_packages.into_iter(),
                          feat::is_enabled(feat::BuildDeps));

    let end_time = PreciseTime::now();
    info!("Graph build stats ({} sec):", start_time.to(end_time));

    for stat in res {
        info!("Target {}: {} nodes, {} edges",
              stat.target, stat.node_count, stat.edge_count,);
    }

    let graph_arc = Arc::new(RwLock::new(graph));
    LogDirectory::validate(&config.log_dir)?;
    let log_dir = LogDirectory::new(&config.log_dir);
    LogIngester::start(&config, log_dir, datastore.clone())?;

    WorkerMgr::start(&config, &datastore, db_pool.clone())?;
    ScheduleMgr::start(&config, &datastore, db_pool.clone())?;

    info!("builder-jobsrv listening on {}:{}",
          cfg.listen_addr(),
          cfg.listen_port());

    server::new(move || {
        let app_state = AppState::new(&config, &datastore, db_pool.clone(), &graph_arc);

        App::with_state(app_state).middleware(Logger::default().exclude("/status"))
                                  .resource("/status", |r| {
                                      r.get().f(status);
                                      r.head().f(status)
                                  })
                                  .route("/rpc", Method::POST, handle_rpc)
    }).workers(cfg.handler_count())
      .keep_alive(KeepAlive::Timeout(cfg.http.keep_alive))
      .bind(cfg.http.clone())
      .unwrap()
      .start();

    let _ = sys.run();
    Ok(())
}

pub fn migrate(config: &Config) -> Result<()> {
    let ds = DataStore::new(&config.datastore);
    ds.setup()
}
