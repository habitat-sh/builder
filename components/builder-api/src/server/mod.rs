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
use self::framework::middleware::{Authenticated, Optional, XRouteClient};
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

    // TODO: Move registration of paths into each resource module

    server::new(move || {
        let app_state = AppState::new(&config);

        App::with_state(app_state)
            .middleware(Logger::default())
            .middleware(XRouteClient)
            .prefix("/v1")
            //
            // Unauthenticated resources
            //
            .resource("/status", |r| { r.get().f(status); r.head().f(status)})
            .resource("/authenticate/{code}", |r| r.get().f(Authenticate::authenticate))
            .resource("/depot/pkgs/origins/{origin}/stats", |r| r.get().f(Packages::get_stats))
            .resource("/depot/origins/{origin}", |r| r.get().f(Origins::get_origin))
            //
            // Authenticated resources
            //
            //
            // Pkgs resource
            //
            .resource("/depot/pkgs/{origin}", |r| {
                r.middleware(Optional);
                r.method(http::Method::GET).with(Packages::get_packages);
            })
            //
            // Profile resource
            //
            .resource("/profile", |r| {
                r.middleware(Authenticated);
                r.get().f(Profile::get_profile);
                r.method(http::Method::PATCH).with(Profile::update_profile);
            })
            .resource("/profile/access-tokens", |r| {
                r.middleware(Authenticated);
                r.get().f(Profile::get_access_tokens);
                r.post().f(Profile::generate_access_token);
            })
            .resource("/profile/access-tokens/{id}", |r| {
                r.middleware(Authenticated);
                r.delete().f(Profile::revoke_access_token);
            })
            //
            //  User resource
            //
            .resource("/user/invitations", |r| {
                r.middleware(Authenticated);
                r.get().f(User::get_invitations);
            })
            .resource("/user/origins", |r| {
                r.middleware(Authenticated);
                r.get().f(User::get_origins);
            })
            //
            //  Origins resource
            //
            .resource("/depot/origins", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).with(Origins::create_origin);
            })
            .resource("/depot/origins/{origin}/keys", |r| {
                r.middleware(Authenticated);
                r.method(http::Method::POST).f(Origins::create_keys);
            })
    }).workers(cfg.handler_count())
        .bind(cfg.http.clone())
        .unwrap()
        .run();

    Ok(())
}

// TODO:

// ORIGIN HANDLERS: "/depot/origins/..."

/*

   r.post(
        "/origins",
        XHandler::new(origin_create).before(basic.clone()),
        "origin_create",
    );
    r.put(
        "/origins/:name",
        XHandler::new(origin_update).before(basic.clone()),
        "origin_update",
    );
    r.get("/origins/:origin", origin_show, "origin");
    r.get("/origins/:origin/keys", list_origin_keys, "origin_keys");
    r.get(
        "/origins/:origin/keys/latest",
        download_latest_origin_key,
        "origin_key_latest",
    );
    r.get(
        "/origins/:origin/keys/:revision",
        download_origin_key,
        "origin_key",
    );
    r.get(
        "/origins/:origin/encryption_key",
        XHandler::new(download_latest_origin_encryption_key).before(basic.clone()),
        "origin_encryption_key_download",
    );
    r.post(
        "/origins/:origin/keys",
        XHandler::new(generate_origin_keys).before(basic.clone()),
        "origin_key_generate",
    );
    r.post(
        "/origins/:origin/keys/:revision",
        XHandler::new(upload_origin_key).before(basic.clone()),
        "origin_key_create",
    );
    r.post(
        "/origins/:origin/secret_keys/:revision",
        XHandler::new(upload_origin_secret_key).before(basic.clone()),
        "origin_secret_key_create",
    );
    r.post(
        "/origins/:origin/secret",
        XHandler::new(create_origin_secret).before(basic.clone()),
        "origin_secret_create",
    );
    r.get(
        "/origins/:origin/secret",
        XHandler::new(list_origin_secrets).before(basic.clone()),
        "origin_secret_list",
    );
    r.delete(
        "/origins/:origin/secret/:secret",
        XHandler::new(delete_origin_secret).before(basic.clone()),
        "origin_secret_delete",
    );
    r.get(
        "/origins/:origin/secret_keys/latest",
        XHandler::new(download_latest_origin_secret_key).before(basic.clone()),
        "origin_secret_key_latest",
    );
    r.get(
        "/origins/:origin/integrations/:integration/names",
        XHandler::new(handlers::integrations::fetch_origin_integration_names).before(basic.clone()),
        "origin_integration_get_names",
    );
    r.put(
        "/origins/:origin/integrations/:integration/:name",
        XHandler::new(handlers::integrations::create_origin_integration).before(basic.clone()),
        "origin_integration_put",
    );
    r.delete(
        "/origins/:origin/integrations/:integration/:name",
        XHandler::new(handlers::integrations::delete_origin_integration).before(basic.clone()),
        "origin_integration_delete",
    );
    r.get(
        "/origins/:origin/integrations/:integration/:name",
        XHandler::new(handlers::integrations::get_origin_integration).before(basic.clone()),
        "origin_integration_get",
    );
    r.get(
        "/origins/:origin/integrations",
        XHandler::new(handlers::integrations::fetch_origin_integrations).before(basic.clone()),
        "origin_integrations",
    );
    r.post(
        "/origins/:origin/users/:username/invitations",
        XHandler::new(invite_to_origin).before(basic.clone()),
        "origin_invitation_create",
    );
    r.put(
        "/origins/:origin/invitations/:invitation_id",
        XHandler::new(accept_invitation).before(basic.clone()),
        "origin_invitation_accept",
    );
    r.put(
        "/origins/:origin/invitations/:invitation_id/ignore",
        XHandler::new(ignore_invitation).before(basic.clone()),
        "origin_invitation_ignore",
    );
    r.delete(
        "/origins/:origin/invitations/:invitation_id",
        XHandler::new(rescind_invitation).before(basic.clone()),
        "origin_invitation_rescind",
    );
    r.get(
        "/origins/:origin/invitations",
        XHandler::new(list_origin_invitations).before(basic.clone()),
        "origin_invitations",
    );
    r.get(
        "/origins/:origin/users",
        XHandler::new(list_origin_members).before(basic.clone()),
        "origin_users",
    );
    r.delete(
        "/origins/:origin/users/:username",
        XHandler::new(origin_member_delete).before(basic.clone()),
        "origin_member_delete",
    );
}
*/

// PACKAGES HANLDERS "/depot/pkgs/..."

/*
    r.get(
        "/pkgs/search/:query",
        XHandler::new(search_packages).before(opt.clone()),
        "package_search",
    );
    r.get(
        "/pkgs/:origin",
        XHandler::new(list_packages).before(opt.clone()),
        "packages",
    );
    r.get(
        "/:origin/pkgs",
        XHandler::new(list_unique_packages).before(opt.clone()),
        "packages_unique",
    );
    r.get(
        "/pkgs/:origin/:pkg",
        XHandler::new(list_packages).before(opt.clone()),
        "packages_pkg",
    );
    r.get(
        "/pkgs/:origin/:pkg/versions",
        XHandler::new(list_package_versions).before(opt.clone()),
        "package_pkg_versions",
    );
    r.get(
        "/pkgs/:origin/:pkg/latest",
        XHandler::new(show_package).before(opt.clone()),
        "package_pkg_latest",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version",
        XHandler::new(list_packages).before(opt.clone()),
        "packages_version",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version/latest",
        XHandler::new(show_package).before(opt.clone()),
        "package_version_latest",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version/:release",
        XHandler::new(show_package).before(opt.clone()),
        "package",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version/:release/channels",
        XHandler::new(package_channels).before(opt.clone()),
        "package_channels",
    );
    r.get(
        "/pkgs/:origin/:pkg/:version/:release/download",
        XHandler::new(download_package).before(opt.clone()),
        "package_download",
    );
    r.post(
        "/pkgs/:origin/:pkg/:version/:release",
        XHandler::new(upload_package).before(basic.clone()),
        "package_upload",
    );
    r.patch(
        "/pkgs/:origin/:pkg/:version/:release/:visibility",
        XHandler::new(package_privacy_toggle).before(basic.clone()),
        "package_privacy_toggle",
    );

    if feat::is_enabled(feat::Jobsrv) {
        r.get(
            "/pkgs/origins/:origin/stats",
            package_stats,
            "package_stats",
        );
        r.post(
            "/pkgs/schedule/:origin/:pkg",
            XHandler::new(schedule).before(basic.clone()),
            "schedule",
        );
        r.get("/pkgs/schedule/:groupid", get_schedule, "schedule_get");
        r.get(
            "/pkgs/schedule/:origin/status",
            get_origin_schedule_status,
            "schedule_get_global",
        );

    }    
*/

// CHANNELS HANDLERS "/depot/channels/..."

/*

    r.get("/channels/:origin", list_channels, "channels");
    r.get(
        "/channels/:origin/:channel/pkgs",
        XHandler::new(list_packages).before(opt.clone()),
        "channel_packages",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg",
        XHandler::new(list_packages).before(opt.clone()),
        "channel_packages_pkg",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg/latest",
        XHandler::new(show_package).before(opt.clone()),
        "channel_package_latest",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg/:version",
        XHandler::new(list_packages).before(opt.clone()),
        "channel_packages_version",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg/:version/latest",
        XHandler::new(show_package).before(opt.clone()),
        "channel_packages_version_latest",
    );
    r.get(
        "/channels/:origin/:channel/pkgs/:pkg/:version/:release",
        XHandler::new(show_package).before(opt.clone()),
        "channel_package_release",
    );
    r.put(
        "/channels/:origin/:channel/pkgs/:pkg/:version/:release/promote",
        XHandler::new(promote_package).before(basic.clone()),
        "channel_package_promote",
    );
    r.put(
        "/channels/:origin/:channel/pkgs/:pkg/:version/:release/demote",
        XHandler::new(demote_package).before(basic.clone()),
        "channel_package_demote",
    );
    r.post(
        "/channels/:origin/:channel",
        XHandler::new(create_channel).before(basic.clone()),
        "channel_create",
    );
    r.delete(
        "/channels/:origin/:channel",
        XHandler::new(delete_channel).before(basic.clone()),
        "channel_delete",
    );
*/

// PROJECTS HANLDERS - "/projects/..."

/*

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
            
*/

// JOBS HANDLERS - "/v1/jobs/..."

/*

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
*/

// OTHER HANDLERS

/*
        r.post("/notify", notify, "notify");

-            r.get(
-                "/ext/installations/:install_id/repos/:repo_id/contents/:path",
-                XHandler::new(github::repo_file_content).before(basic.clone()),
-                "ext_repo_content",
-            );

        r.post(
            "/ext/integrations/:registry_type/credentials/validate",
            XHandler::new(validate_registry_credentials).before(basic.clone()),
            "ext_credentials_registry",
        );
*/
