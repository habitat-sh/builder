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

use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::Arc;
use std::thread;

use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::server::{self, KeepAlive};
use actix_web::{App, HttpRequest, HttpResponse, Result};

use github_api_client::GitHubClient;
use hab_net::socket;

use oauth_client::client::OAuth2Client;
use segment_api_client::SegmentClient;

use self::error::Error;
use self::framework::middleware::{Authentication, XRouteClient};
use self::services::route_broker::RouteBroker;
use self::services::s3::S3Handler;
use self::services::upstream::{UpstreamClient, UpstreamMgr};

use self::resources::authenticate::Authenticate;
use self::resources::channels::Channels;
use self::resources::ext::Ext;
use self::resources::jobs::Jobs;
use self::resources::notify::Notify;
use self::resources::origins::Origins;
use self::resources::pkgs::Packages;
use self::resources::profile::Profile;
use self::resources::projects::Projects;
use self::resources::user::User;

use config::{Config, GatewayCfg};

features! {
    pub mod feat {
        const List = 0b00000001,
        const Jobsrv = 0b00000010,
        const Upstream = 0b00000100
    }
}

// Application state
pub struct AppState {
    config: Config,
    packages: S3Handler,
    github: GitHubClient,
    oauth: OAuth2Client,
    segment: SegmentClient,
    upstream: UpstreamClient,
}

impl AppState {
    pub fn new(config: &Config) -> AppState {
        AppState {
            config: config.clone(),
            packages: S3Handler::new(config.s3.clone()),
            github: GitHubClient::new(config.github.clone()),
            oauth: OAuth2Client::new(config.oauth.clone()),
            segment: SegmentClient::new(config.segment.clone()),
            upstream: UpstreamClient::default(),
        }
    }
}

fn enable_features(config: &Config) {
    let features: HashMap<_, _> = HashMap::from_iter(vec![
        ("LIST", feat::List),
        ("JOBSRV", feat::Jobsrv),
        ("UPSTREAM", feat::Upstream),
    ]);
    let features_enabled = config
        .api
        .features_enabled
        .split(",")
        .map(|f| f.trim().to_uppercase());
    for key in features_enabled {
        if features.contains_key(key.as_str()) {
            info!("Enabling feature: {}", key);
            feat::enable(features.get(key.as_str()).unwrap().clone());
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
pub fn status(_req: &HttpRequest<AppState>) -> HttpResponse {
    HttpResponse::new(StatusCode::OK)
}

pub fn run(config: Config) -> Result<()> {
    enable_features(&config);

    let c = config.clone();
    thread::Builder::new()
        .name("route-broker".to_string())
        .spawn(move || {
            RouteBroker::start(socket::srv_ident(), c.route_addrs())
                .map_err(Error::Connection)
                .unwrap();
        }).unwrap();

    UpstreamMgr::start(&config, S3Handler::new(config.s3.to_owned()))?;

    let cfg = Arc::new(config.clone());

    info!(
        "builder-api listening on {}:{}",
        cfg.listen_addr(),
        cfg.listen_port()
    );

    server::new(move || {
        let app_state = AppState::new(&config);

        App::with_state(app_state)
            .middleware(Logger::default().exclude("/v1/status"))
            .middleware(XRouteClient)
            .middleware(Authentication)
            .prefix("/v1")
            .configure(Authenticate::register)
            .configure(Channels::register)
            .configure(Ext::register)
            .configure(Jobs::register)
            .configure(Notify::register)
            .configure(Origins::register)
            .configure(Packages::register)
            .configure(Profile::register)
            .configure(Projects::register)
            .configure(User::register)
            .resource("/status", |r| {
                r.get().f(status);
                r.head().f(status)
            })
    }).workers(cfg.handler_count())
    .keep_alive(KeepAlive::Timeout(cfg.http.keep_alive))
    .bind(cfg.http.clone())
    .unwrap()
    .run();

    Ok(())
}
