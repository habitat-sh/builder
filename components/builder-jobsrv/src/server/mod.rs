// Copyright (c) 2016-2020 Chef Software Inc. and/or applicable contributors
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
mod scheduler2;
mod worker_manager;

use self::{log_archiver::LogArchiver,
           log_directory::LogDirectory,
           log_ingester::LogIngester,
           scheduler::ScheduleMgr,
           scheduler2::Scheduler,
           worker_manager::WorkerMgr};
use crate::{bldr_core::rpc::RpcMessage,
            builder_graph::target_graph::TargetGraph,
            config::{Config,
                     GatewayCfg},
            data_store::DataStore,
            db::{models::package::*,
                 DbPool},
            error::Result,
            hab_core::package::PackageTarget,
            protocol::originsrv::OriginPackage,
            scheduler_datastore::SchedulerDataStoreDb,
            Error};
use actix_web::{dev::Body,
                http::StatusCode,
                middleware::Logger,
                web::{self,
                      Data,
                      Json,
                      JsonConfig,
                      Query},
                App,
                HttpResponse,
                HttpServer};

use std::{collections::{HashMap,
                        HashSet},
          iter::{FromIterator,
                 Iterator},
          panic,
          str::FromStr,
          sync::{Arc,
                 RwLock},
          time::Instant};

// Set a max size for JsonConfig payload. Default is 32Kb
const MAX_JSON_PAYLOAD: usize = 262_144;

features! {
    pub mod feat {
        const BuildDeps = 0b0000_0001,
        const LegacyProject = 0b0000_0010,
        const UseCyclicGraph = 0b0000_0100,
        const NewScheduler = 0b0000_1000
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
    scheduler:     Option<Scheduler>,
}

impl AppState {
    pub fn new(cfg: &Config,
               datastore: &DataStore,
               db: DbPool,
               graph: &Arc<RwLock<TargetGraph>>,
               scheduler: Option<&Scheduler>)
               -> Self {
        AppState { archiver: log_archiver::from_config(&cfg.archive).unwrap(),
                   datastore: datastore.clone(),
                   db,
                   graph: graph.clone(),
                   log_dir: LogDirectory::new(&cfg.log_dir),
                   build_targets: cfg.build_targets.clone(),
                   scheduler: scheduler.map(|s| s.clone()) }
    }
}

// Patterned after helpers in the api
#[derive(Deserialize)]
pub struct OriginTarget {
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
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

#[allow(clippy::needless_pass_by_value)]
fn handle_graph(state: Data<AppState>, query: Query<OriginTarget>) -> HttpResponse {
    let origin_filter = query.origin.as_deref(); //
    let target = query.target.as_deref().unwrap_or("x86_64-linux");

    match fetch_graph_for_target(state, &target, origin_filter) {
        Ok(body) => HttpResponse::with_body(StatusCode::OK, Body::from(body)),
        Err(err) => {
            HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR,
                                    Body::from_message(err.to_string()))
        } // maybe we do 401 ill formed instead?
    }
}

#[tracing::instrument(skip(state))]
fn fetch_graph_for_target(state: Data<AppState>,
                          target_string: &str,
                          origin_filter: Option<&str>)
                          -> Result<String> {
    let target = PackageTarget::from_str(target_string).unwrap(); // fix when we no longer hardcode this value above
    let target_graph = state.graph.read().map_err(|_| Error::System)?; // Should rethink this error
    let graph = target_graph.graph_for_target(target).ok_or(Error::System)?;
    let body = graph.as_json(origin_filter);
    Ok(body)
}

fn enable_features_from_config(cfg: &Config) {
    let features: HashMap<_, _> = HashMap::from_iter(vec![("BUILDDEPS", feat::BuildDeps),
                                                          ("LEGACYPROJECT", feat::LegacyProject),
                                                          ("USECYCLICGRAPH",
                                                           feat::UseCyclicGraph),
                                                          ("NEWSCHEDULER", feat::NewScheduler)]);
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
    let mut graph = TargetGraph::new(feat::is_enabled(feat::UseCyclicGraph));
    let pkg_conn = &db_pool.get_conn()?;
    let packages = Package::get_all_latest(&pkg_conn)?;
    let origin_packages: Vec<OriginPackage> = packages.iter().map(|p| p.clone().into()).collect();
    let start_time = Instant::now();

    let res = graph.build(&origin_packages, feat::is_enabled(feat::BuildDeps));

    info!("Graph build stats ({} sec):",
          start_time.elapsed().as_secs_f64());

    for stat in res {
        info!("Target {}: {} nodes, {} edges",
              stat.target, stat.node_count, stat.edge_count,);
    }

    info!("builder-jobsrv listening on {}:{}",
          cfg.listen_addr(),
          cfg.listen_port());

    let graph_arc = Arc::new(RwLock::new(graph));
    LogDirectory::validate(&config.log_dir)?;
    let log_dir = LogDirectory::new(&config.log_dir);
    LogIngester::start(&config, log_dir, datastore.clone())?;

    if feat::is_enabled(feat::NewScheduler) {
        let scheduler_datastore = SchedulerDataStoreDb::new(datastore.clone());
        let (scheduler, scheduler_handle) = Scheduler::start(Box::new(scheduler_datastore), 1);
        let scheduler_for_http = scheduler.clone();

        WorkerMgr::start(&config, &datastore, db_pool.clone(), Some(scheduler))?;

        let http_serv = HttpServer::new(move || {
                            let app_state = AppState::new(&config,
                                                          &datastore,
                                                          db_pool.clone(),
                                                          &graph_arc,
                                                          Some(&scheduler_for_http));

                            App::new().data(JsonConfig::default().limit(MAX_JSON_PAYLOAD))
                    .data(app_state)
                    .wrap(Logger::default().exclude("/status"))
                    .service(web::resource("/status").route(web::get().to(status))
                                                    .route(web::head().to(status)))
                    .route("/rpc", web::post().to(handle_rpc))
                    .route("/graph", web::get().to(handle_graph))
                        }).workers(cfg.handler_count())
                          .keep_alive(cfg.http.keep_alive)
                          .bind(cfg.http.clone())
                          .unwrap()
                          .run();

        // This is not what we want.  try_join! would be more appropriate so that we shut down if
        // any of the things we're joining returns an error. However, the http_serv and
        // scheduler_handle have different error types and I can't figure out how to resolve
        // that scenario. We also need to handle any errors on the scheduler
        let (http_res, _sched_res) = tokio::join!(http_serv, scheduler_handle);

        http_res.map_err(Error::from)
    } else {
        WorkerMgr::start(&config, &datastore, db_pool.clone(), None)?;
        ScheduleMgr::start(&config, &datastore, db_pool.clone())?;
        HttpServer::new(move || {
            let app_state = AppState::new(&config, &datastore, db_pool.clone(), &graph_arc, None);

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
}

pub fn migrate(config: &Config) -> Result<()> {
    let ds = DataStore::new(&config.datastore);
    ds.setup()
}
