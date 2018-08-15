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

pub mod error;
pub mod framework;
pub mod helpers;
pub mod resources;
pub mod services;

use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::Arc;
use std::thread;

use actix_web::http::{self, StatusCode};
use actix_web::middleware::Logger;
use actix_web::{server, App, HttpRequest, HttpResponse, Result};

use github_api_client::GitHubClient;
use hab_net::socket;

use oauth_client::client::OAuth2Client;
use segment_api_client::SegmentClient;

use self::error::Error;
use self::framework::middleware::{Authenticated, XRouteClient};
use self::services::route_broker::RouteBroker;
use self::services::s3::S3Handler;
// TODO: use services::upstream::{UpstreamClient, UpstreamMgr};

use self::resources::authenticate::*;
use self::resources::origins::*;
use self::resources::pkgs::*;
use self::resources::profile::*;
use self::resources::user::*;

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
    // TODO: upstream: UpstreamClient
}

impl AppState {
    pub fn new(config: &Config) -> AppState {
        AppState {
            config: config.clone(),
            packages: S3Handler::new(config.s3.clone()),
            github: GitHubClient::new(config.github.clone()),
            oauth: OAuth2Client::new(config.oauth.clone()),
            segment: SegmentClient::new(config.segment.clone()),
            // TODO: upstream: UpstreamClient::default()
        }
    }
}

/*
    TODO: Migrate these routes to the new framework...

    fn router(config: Arc<Config>) -> Router {
        let basic = Authenticated::new(config.api.key_path.clone());
        let worker =
            Authenticated::new(config.api.key_path.clone()).require(FeatureFlags::BUILD_WORKER);
        let admin = Authenticated::new(PathBuf::new()).require(FeatureFlags::ADMIN);

        let mut r = Router::new();

        if feat::is_enabled(feat::Jobsrv) {
            r.post(
                "/jobs/group/:id/promote/:channel",
                XHandler::new(job_group_promote).before(basic.clone()),
                "job_group_promote",
            );
            r.post(
                "/jobs/group/:id/demote/:channel",
                XHandler::new(job_group_demote).before(basic.clone()),
                "job_group_demote",
            );
            r.post(
                "/jobs/group/:id/cancel",
                XHandler::new(job_group_cancel).before(basic.clone()),
                "job_group_cancel",
            );
            r.get("/rdeps/:origin/:name", rdeps_show, "rdeps");
            r.get(
                "/jobs/:id",
                XHandler::new(job_show).before(basic.clone()),
                "job",
            );
            r.get(
                "/jobs/:id/log",
                XHandler::new(job_log).before(basic.clone()),
                "job_log",
            );
            r.post(
                "/projects",
                XHandler::new(project_create).before(basic.clone()),
                "projects",
            );
            r.get(
                "/projects/:origin/:name",
                XHandler::new(project_show).before(basic.clone()),
                "project",
            );
            r.get(
                "/projects/:origin",
                XHandler::new(project_list).before(basic.clone()),
                "project_list",
            );
            r.get(
                "/projects/:origin/:name/jobs",
                XHandler::new(project_jobs).before(basic.clone()),
                "project_jobs",
            );
            r.put(
                "/projects/:origin/:name",
                XHandler::new(project_update).before(basic.clone()),
                "edit_project",
            );
            r.delete(
                "/projects/:origin/:name",
                XHandler::new(project_delete).before(basic.clone()),
                "delete_project",
            );
            r.patch(
                "/projects/:origin/:name/:visibility",
                XHandler::new(project_privacy_toggle).before(basic.clone()),
                "project_privacy_toggle",
            );
            r.get(
                "/projects/:origin/:name/integrations/:integration/default",
                XHandler::new(get_project_integration).before(basic.clone()),
                "project_integration_get",
            );
            r.put(
                "/projects/:origin/:name/integrations/:integration/default",
                XHandler::new(create_project_integration).before(basic.clone()),
                "project_integration_put",
            );
            r.delete(
                "/projects/:origin/:name/integrations/:integration/default",
                XHandler::new(delete_project_integration).before(basic.clone()),
                "project_integration_delete",
            );
            r.get(
                "/ext/installations/:install_id/repos/:repo_id/contents/:path",
                XHandler::new(github::repo_file_content).before(basic.clone()),
                "ext_repo_content",
            );
        }

        r.get("/status", status, "status");
        r.get("/authenticate/:code", authenticate, "authenticate");
        r.post("/notify", notify, "notify");
        r.patch(
            "/profile",
            XHandler::new(update_profile).before(basic.clone()),
            "update_profile",
        );
        r.get(
            "/profile",
            XHandler::new(get_profile).before(basic.clone()),
            "get_profile",
        );

        r.get(
            "/profile/access-tokens",
            XHandler::new(get_access_tokens).before(basic.clone()),
            "get_access_tokens",
        );
        r.post(
            "/profile/access-tokens",
            XHandler::new(generate_access_token).before(basic.clone()),
            "generate_access_token",
        );
        r.delete(
            "/profile/access-tokens/:id",
            XHandler::new(revoke_access_token).before(basic.clone()),
            "revoke_access_token",
        );

        r.get(
            "/user/invitations",
            XHandler::new(list_account_invitations).before(basic.clone()),
            "user_invitations",
        );
        r.get(
            "/user/origins",
            XHandler::new(list_user_origins).before(basic.clone()),
            "user_origins",
        );
        r.post(
            "/ext/integrations/:registry_type/credentials/validate",
            XHandler::new(validate_registry_credentials).before(basic.clone()),
            "ext_credentials_registry",
        );

        r.post(
            "/admin/search",
            XHandler::new(search).before(admin.clone()),
            "admin_search",
        );
        r.get(
            "/admin/accounts/:id",
            XHandler::new(account_show).before(admin.clone()),
            "admin_account",
        );

        // TODO : Don't forget about the depot routes :)
        // Mount these in both the "v1" and "v1/depot" namespace..

        depot::server::add_routes(&mut r, basic, worker);

        r
    }
*/

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
        })
        .unwrap();

    // TODO: UpstreamMgr::start(&cfg, s3::S3Handler::new(cfg.s3.to_owned()))?;
    // TODO: chain.link_after(Cors);

    /* TODO ? Do we need this ?:
        if let Some(ref path) = config.ui.root {
            debug!("Mounting UI at filepath {}", path);
            mount.mount("/", Static::new(path));
        }
    */

    let cfg = Arc::new(config.clone());

    info!(
        "builder-api listening on {}:{}",
        cfg.listen_addr(),
        cfg.listen_port()
    );

    server::new(move || {
        let app_state = AppState::new(&config);

        App::with_state(app_state)
            .middleware(Logger::default())
            .middleware(XRouteClient)
            .prefix("/v1")
            //
            // Unauthenticated resources
            //
            .resource("/status", |r| r.head().f(status))
            .resource("/authenticate/{code}", |r| r.get().f(authenticate))
            .resource("/depot/pkgs/origins/{origin}/stats", |r| r.get().f(package_stats))
            .resource("/depot/origins/{origin}", |r| r.get().f(origin_show))
            //
            // Authenticated resources
            //
            //
            // Profile resource
            //
            .resource("/profile", |r| {
                r.middleware(Authenticated);
                r.get().f(get_profile);
            })
            //
            //  User resource
            //
            .resource("/user/invitations", |r| {
                r.middleware(Authenticated);
                r.get().f(list_account_invitations); 
            })
            .resource("/user/origins", |r| {
                r.middleware(Authenticated);
                r.get().f(user_origins);
            })
            //
            //  Origins resource
            //
            .resource("/depot/origins", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).with(origin_create);
            })
            .resource("/depot/origins/{origin}/keys", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).f(generate_origin_keys);
            })
    }).workers(cfg.handler_count())
        .bind(cfg.http.clone())
        .unwrap()
        .run();

    Ok(())
}
