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

// mod handlers;

use std::collections::HashMap;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

//use router::Router;
//use staticfile::Static;
use actix_web::error;
use actix_web::http;
use actix_web::middleware::{Middleware, Response, Started};
use actix_web::{server, App, HttpRequest, HttpResponse, Result};

//use backend::{s3, s3::S3Cli};
use github_api_client::GitHubClient;
use hab_net::privilege::FeatureFlags;
use hab_net::socket;
//use middleware::{Authenticated, Cors, GitHubCli, OAuthCli, SegmentCli, XHandler, XRouteClient};
use oauth_client::client::OAuth2Client;
use segment_api_client::SegmentClient;
//use upstream::{UpstreamCli, UpstreamClient, UpstreamMgr};

//use self::handlers::*;
use super::config::GatewayCfg;
use super::conn::RouteBroker;
//use super::depot;
//use super::error::{Error, Result};
//use super::github;
use config::Config;
use feat;

// Application state
struct AppState {
    config: Config,
}

struct ApiSrv;

impl ApiSrv {
    /*
    fn add_middleware(config: Arc<Config>, chain: &mut iron::Chain) {
        chain.link(persistent::Read::<Config>::both(config.clone()));

        chain.link(persistent::Read::<OAuthCli>::both(OAuth2Client::new(
            config.oauth.clone(),
        )));

        chain.link(persistent::Read::<GitHubCli>::both(GitHubClient::new(
            config.github.clone(),
        )));

        chain.link(persistent::Read::<SegmentCli>::both(SegmentClient::new(
            config.segment.clone(),
        )));

        chain.link(persistent::Read::<S3Cli>::both(s3::S3Handler::new(
            config.s3.to_owned(),
        )));

        chain.link(persistent::Read::<UpstreamCli>::both(
            UpstreamClient::default(),
        ));

        chain.link_before(XRouteClient);
        chain.link_after(Cors);
    }
*/

    /*
    fn mount(config: Arc<Config>, chain: iron::Chain) -> Mount {
        let mut mount = Mount::new();

        if let Some(ref path) = config.ui.root {
            debug!("Mounting UI at filepath {}", path);
            mount.mount("/", Static::new(path));
        }
        mount.mount("/v1", chain);

        // TBD: Deprecate legacy depot API path
        let mut depot_chain = iron::Chain::new(depot::server::router(config.clone()));
        Self::add_middleware(config, &mut depot_chain);
        mount.mount("/v1/depot", depot_chain);
        mount
    }
*/

    /*
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

        depot::server::add_routes(&mut r, basic, worker);

        r
    }
*/
}

fn enable_features_from_config(config: &Config) {
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

fn hello(req: &HttpRequest<AppState>) -> String {
    "hello world!".to_string()
}

pub fn run(config: Config) -> Result<()> {
    enable_features_from_config(&config);

    //let cfg = Arc::new(config);

    server::new(move || {
        App::with_state(AppState {
            config: config.clone(),
        }).prefix("/v1")
            .resource("/hello", |r| r.f(hello))
    }).bind("127.0.0.1:9636")
        .unwrap()
        .run();

    /*
    let mut chain = Chain::new(ApiSrv::router(cfg.clone()));
    ApiSrv::add_middleware(cfg.clone(), &mut chain);
    chain.link_before(XRouteClient);
    chain.link_after(Cors);

    let mount = ApiSrv::mount(cfg.clone(), chain);
    let mut server = iron::Iron::new(mount);
    server.threads = cfg.handler_count();
    let http_listen_addr = (cfg.listen_addr().clone(), cfg.listen_port());

    thread::Builder::new()
        .name("http-handler".to_string())
        .spawn(move || server.http(http_listen_addr))
        .unwrap();
    info!(
        "APISrv listening on {}:{}",
        cfg.listen_addr(),
        cfg.listen_port()
    );

    UpstreamMgr::start(&cfg, s3::S3Handler::new(cfg.s3.to_owned()))?;
    RouteBroker::start(socket::srv_ident(), cfg.route_addrs()).map_err(Error::Connection)?;
*/

    info!("builder-api is ready to go.");

    Ok(())
}
