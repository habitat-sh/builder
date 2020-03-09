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

use self::{log_archiver::LogArchiver,
           log_directory::LogDirectory,
           log_ingester::LogIngester,
           scheduler::ScheduleMgr,
           worker_manager::WorkerMgr};
use crate::{bldr_core::{rpc::RpcMessage,
                        target_graph::TargetGraph},
            config::{Config,
                     GatewayCfg},
            data_store::DataStore,
            db::{models::package::*,
                 DbPool},
            error::Result,
            hab_core::package::PackageTarget,
            protocol::originsrv::OriginPackage,
            Error};
use actix_web::{dev::Body,
                http::StatusCode,
                middleware::Logger,
                web::{self,
                      Data,
                      Json,
                      JsonConfig},
                App,
                HttpResponse,
                HttpServer};
use std::{collections::{HashMap,
                        HashSet},
          iter::{FromIterator,
                 Iterator},
          panic,
          sync::{Arc,
                 RwLock},
          time::Instant};

// Set a max size for JsonConfig payload. Default is 32Kb
const MAX_JSON_PAYLOAD: usize = 262_144;

features! {
    pub mod feat {
        const BuildDeps = 0b0000_0001,
        const LegacyProject = 0b0000_0010
    }
}

// Application state
pub struct AppState {
    archiver:      Box<dyn LogArchiver>,
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
fn status() -> HttpResponse { HttpResponse::new(StatusCode::OK) }

#[allow(clippy::needless_pass_by_value)]
fn handle_rpc(msg: Json<RpcMessage>, state: Data<AppState>) -> HttpResponse {
    debug!("Got RPC message, body =\n{:?}", msg);

    let result = match msg.id.as_str() {
        "JobGet" => handlers::job_get(&msg, &state),
        "JobLogGet" => handlers::job_log_get(&msg, &state),
        "JobGroupSpec" => handlers::job_group_create(&msg, &state),
        "JobGroupCancel" => handlers::job_group_cancel(&msg, &state),
        "JobGroupGet" => handlers::job_group_get(&msg, &state),
        "JobGroupOriginGet" => handlers::job_group_origin_get(&msg, &state),
        "JobGraphPackageCreate" => handlers::job_graph_package_create(&msg, &state),
        "JobGraphPackagePreCreate" => handlers::job_graph_package_precreate(&msg, &state),
        "JobGraphPackageReverseDependenciesGet" => {
            handlers::job_graph_package_reverse_dependencies_get(&msg, &state)
        }
        "JobGraphPackageReverseDependenciesGroupedGet" => {
            handlers::job_graph_package_reverse_dependencies_grouped_get(&msg, &state)
        }

        _ => {
            let err = format!("Unknown RPC message received: {}", msg.id);
            error!("{}", err);
            return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR,
                                           Body::from_message(err));
        }
    };

    match result {
        Ok(m) => HttpResponse::Ok().json(m),
        Err(e) => e.into(),
    }
}

fn enable_features_from_config(cfg: &Config) {
    let features: HashMap<_, _> = HashMap::from_iter(vec![("BUILDDEPS", feat::BuildDeps),
                                                          ("LEGACYPROJECT", feat::LegacyProject)]);
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

pub async fn run(config: Config) -> Result<()> {
    // Set custom panic hook - a panic on the scheduler thread will
    // cause the builder-jobsrv process to exit (and be re-started
    // by the supervisor when running under hab)
    panic::set_hook(Box::new(|panic_info| {
                        let backtrace = backtrace::Backtrace::new();
                        println!("panic info: {:?}", panic_info);
                        println!("{:?}", backtrace);
                        println!("Exiting builder-jobsrv process");
                        std::process::exit(1)
                    }));

    let cfg = Arc::new(config.clone());

    enable_features_from_config(&config);

    let datastore = DataStore::new(&config.datastore);
    let db_pool = DbPool::new(&config.datastore.clone());
    let mut graph = TargetGraph::new();
    let pkg_conn = &db_pool.get_conn()?;
    let packages = Package::get_all_latest(&pkg_conn)?;
    let origin_packages: Vec<OriginPackage> = packages.iter().map(|p| p.clone().into()).collect();
    let start_time = Instant::now();

    let res = graph.build(origin_packages.into_iter(),
                          feat::is_enabled(feat::BuildDeps));

    info!("Graph build stats ({} sec):",
          start_time.elapsed().as_secs_f64());

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

    HttpServer::new(move || {
        let app_state = AppState::new(&config, &datastore, db_pool.clone(), &graph_arc);

        App::new().data(JsonConfig::default().limit(MAX_JSON_PAYLOAD))
                  .data(app_state)
                  .wrap(Logger::default().exclude("/status"))
                  .service(web::resource("/status").route(web::get().to(status))
                                                   .route(web::head().to(status)))
                  .route("/rpc", web::post().to(handle_rpc))
    }).workers(cfg.handler_count())
      .keep_alive(cfg.http.keep_alive)
      .bind(cfg.http.clone())
      .unwrap()
      .run()
      .await
      .map_err(Error::from)
}

pub fn migrate(config: &Config) -> Result<()> {
    let ds = DataStore::new(&config.datastore);
    ds.setup()
}
