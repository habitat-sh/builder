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

pub mod authorize;
pub mod error;
pub mod framework;
pub mod helpers;
pub mod resources;
pub mod services;

use std::{cell::RefCell,
          collections::HashMap,
          iter::FromIterator,
          sync::Arc};

use actix_web::{http::StatusCode,
                middleware::Logger,
                web,
                App,
                Error,
                HttpResponse,
                HttpServer,
                Result};

use crate::{bldr_core::rpc::RpcClient,
            db::{migration,
                 DbPool}};
use github_api_client::GitHubClient;

use artifactory_client::client::ArtifactoryClient;
use oauth_client::client::OAuth2Client;

use self::framework::middleware::authentication_middleware;

use self::services::{memcache::MemcacheClient,
                     s3::S3Handler};

use self::resources::{authenticate::Authenticate,
                      channels::Channels,
                      ext::Ext,
                      jobs::Jobs,
                      notify::Notify,
                      origins::Origins,
                      pkgs::Packages,
                      profile::Profile,
                      projects::Projects,
                      settings::Settings,
                      user::User};

use crate::config::{Config,
                    GatewayCfg};

features! {
    pub mod feat {
        const List = 0b0000_0001,
        const Jobsrv = 0b0000_0010,
        const Artifactory = 0b0000_0100,
        const BuildDeps = 0b0000_1000
    }
}

// Application state
pub struct AppState {
    config:      Config,
    packages:    S3Handler,
    github:      GitHubClient,
    jobsrv:      RpcClient,
    oauth:       OAuth2Client,
    memcache:    RefCell<MemcacheClient>,
    artifactory: ArtifactoryClient,
    db:          DbPool,
}

impl AppState {
    pub fn new(config: &Config, db: DbPool) -> error::Result<AppState> {
        Ok(AppState { config: config.clone(),
                      packages: S3Handler::new(config.s3.clone()),
                      github: GitHubClient::new(config.github.clone())?,
                      jobsrv: RpcClient::new(&format!("{}", config.jobsrv)),
                      oauth: OAuth2Client::new(config.oauth.clone())?,
                      memcache: RefCell::new(MemcacheClient::new(&config.memcache.clone())),
                      artifactory: ArtifactoryClient::new(config.artifactory.clone())?,
                      db })
    }
}

fn enable_features(config: &Config) {
    let features: HashMap<_, _> = HashMap::from_iter(vec![("LIST", feat::List),
                                                          ("JOBSRV", feat::Jobsrv),
                                                          ("ARTIFACTORY", feat::Artifactory),
                                                          ("BUILDDEPS", feat::BuildDeps)]);
    let features_enabled = config.api
                                 .features_enabled
                                 .split(',')
                                 .map(|f| f.trim().to_uppercase());
    for key in features_enabled {
        if features.contains_key(key.as_str()) {
            info!("Enabling feature: {}", key);
            feat::enable(features[key.as_str()]);
        }
    }

    if feat::is_enabled(feat::List) {
        println!("Listing possible feature flags: {:?}", features.keys());
        println!("Enable features by populating 'features_enabled' in config");
    }
}

/// Endpoint for determining availability of builder-api components.
///
/// Returns a status 200 on success. Any non-200 responses are an outage or a partial outage.
pub fn status() -> HttpResponse { HttpResponse::new(StatusCode::OK) }

pub fn run(config: Config) -> Result<()> {
    enable_features(&config);

    let cfg = Arc::new(config.clone());

    info!("builder-api listening on {}:{}",
          cfg.listen_addr(),
          cfg.listen_port());

    // TED TODO: When originsrv gets removed we need to do the migrations here

    let db_pool = DbPool::new(&config.datastore.clone());

    migration::setup(&db_pool.get_conn().unwrap()).unwrap();

    HttpServer::new(move || {
        let app_state = match AppState::new(&config, db_pool.clone()) {
            Ok(state) => state,
            Err(err) => {
                error!("Unable to create application state, err = {}", err);
                panic!("Cannot start without valid application state");
            }
        };

        App::new().data(app_state)
                  .wrap_fn(authentication_middleware)
                  .wrap(Logger::default().exclude("/v1/status"))
                  .service(web::scope("/v1")
                      .configure(Authenticate::register)
                      .configure(Channels::register)
                      .configure(Ext::register)
                      .configure(Jobs::register)
                      .configure(Notify::register)
                      .configure(Origins::register)
                      .configure(Packages::register)
                      .configure(Profile::register)
                      .configure(Projects::register)
                      .configure(Settings::register)
                      .configure(User::register)
                      .service(web::resource("/status")
                          .route(web::get().to(status))
                          .route(web::head().to(status))))
    }).workers(cfg.handler_count())
      .keep_alive(cfg.http.keep_alive)
      .bind(cfg.http.clone())
      .unwrap()
      .run()
      .map_err(Error::from)
}
