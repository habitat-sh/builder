// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
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

use std::str::FromStr;

use actix_web::http::StatusCode;
use actix_web::{error, FromRequest, HttpMessage, HttpRequest, HttpResponse, Path};

use bldr_core::build_config::{BuildCfg, BLDR_CFG};
use bldr_core::metrics::CounterMetric;
use constant_time_eq::constant_time_eq;
use github_api_client::types::GitHubWebhookPush;
use github_api_client::{AppToken, GitHubClient};
use hab_core::package::Plan;
use hex;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use protocol::jobsrv::{JobGroup, JobGroupSpec, JobGroupTrigger};
use protocol::originsrv::{OriginProject, OriginProjectGet};
use protocol::sessionsrv::{Account, AccountGet};
use serde_json;

use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::services::metrics::Counter;
use server::AppState;

pub enum GitHubEvent {
    Push,
    Ping,
}

impl FromStr for GitHubEvent {
    type Err = Error;

    fn from_str(event: &str) -> error::Result<Self, Error> {
        match event {
            "ping" => Ok(GitHubEvent::Ping),
            "push" => Ok(GitHubEvent::Push),
            _ => Err(Error::UnknownGitHubEvent(event.to_string())),
        }
    }
}

pub fn handle_event(req: HttpRequest<AppState>, body: String) -> Result<HttpResponse> {
    Counter::GitHubEvent.increment();

    let event = match req.headers().get(headers::XGITHUBEVENT) {
        Some(event) => match GitHubEvent::from_str(match event.to_str() {
            Ok(value) => value,
            Err(err) => {
                error!("An empty value was passed for XGitHubEvent Header!");
                return Err(Error::BadRequest(err.to_string()));
            }
        }) {
            Ok(event) => event,
            Err(err) => return Err(err.into()),
        },
        _ => {
            return Err(Error::BadRequest(
                "An unknown value was passed in for XGitHubEvent header!".to_string(),
            ))
        }
    };

    // Authenticate the hook
    let github = &req.state().github;
    let gh_signature = match req.headers().get(headers::XHUBSIGNATURE) {
        Some(ref sig) => sig.clone(),
        None => {
            warn!("Received a GitHub hook with no signature");
            return Err(Error::BadRequest(
                "Recieved a GitHub hook with no signature".to_string(),
            ));
        }
    };

    trace!("handle-notify, {}", body);

    let key = PKey::hmac(github.webhook_secret.as_bytes()).unwrap();
    let mut signer = Signer::new(MessageDigest::sha1(), &key).unwrap();
    signer.update(body.as_bytes()).unwrap();
    let hmac = signer.sign_to_vec().unwrap();
    let computed_signature = format!("sha1={}", &hex::encode(hmac));

    if !constant_time_eq(gh_signature.as_bytes(), computed_signature.as_bytes()) {
        warn!(
            "Web hook signatures don't match. GH = {:?}, Our = {:?}",
            gh_signature, computed_signature
        );
        return Err(Error::BadRequest(
            "Webhook signatures don't match!".to_string(),
        ));
    }

    match event {
        GitHubEvent::Ping => Ok(HttpResponse::new(StatusCode::OK)),
        GitHubEvent::Push => handle_push(&req, &body),
    }
}

pub fn repo_file_content(req: HttpRequest<AppState>) -> Result<HttpResponse> {
    let github = &req.state().github;
    let (path, install_id, repo_id) = Path::<(String, u32, u32)>::extract(&req)
        .unwrap()
        .into_inner();

    let token = {
        debug!(
            "GITHUB-CALL builder_api::github::repo_file_content: Getting app_installation_token; repo_id={} installation_id={} path={}",
            repo_id,
            install_id,
            path
        );
        match github.app_installation_token(install_id) {
            Ok(token) => token,
            Err(err) => {
                warn!("unable to generate github app token, {}", err);
                return Err(err.into());
            }
        }
    };

    match github.contents(&token, repo_id, &path) {
        Ok(None) => Ok(HttpResponse::new(StatusCode::NOT_FOUND)),
        Ok(search) => Ok(HttpResponse::Ok().json(&search)),
        Err(err) => {
            warn!("unable to fetch github contents, {}", err);
            Err(err.into())
        }
    }
}

fn handle_push(req: &HttpRequest<AppState>, body: &str) -> Result<HttpResponse> {
    let hook = match serde_json::from_str::<GitHubWebhookPush>(&body) {
        Ok(hook) => hook,
        Err(err) => {
            return Err(err.into());
        }
    };
    debug!(
        "GITHUB-WEBHOOK builder_api::github::handle_push: received hook; repository={} repository_id={} ref={} installation_id={}",
        hook.repository.full_name,
        hook.repository.id,
        hook.git_ref,
        hook.installation.id
    );

    if hook.commits.is_empty() {
        debug!("GITHUB-WEBHOOK builder_api::github::handle_push: hook commits is empty!");
        return Ok(HttpResponse::new(StatusCode::OK));
    }

    let github = &req.state().github;

    debug!(
        "GITHUB-CALL builder_api::github::handle_push: Getting app_installation_token; installation_id={}",
        hook.installation.id
    );
    let token = match github.app_installation_token(hook.installation.id) {
        Ok(token) => token,
        Err(err) => {
            warn!("unable to generate github app token, {}", err);
            return Err(err.into());
        }
    };

    let mut account_get = AccountGet::new();
    account_get.set_name(hook.pusher.name.clone());
    let account_id = match route_message::<AccountGet, Account>(&req, &account_get) {
        Ok(account) => Some(account.get_id()),
        Err(_) => None,
    };

    let config = read_bldr_config(&github, &token, &hook);
    debug!("Config, {:?}", config);
    let plans = read_plans(&github, &token, &hook, &config);
    debug!("Triggered Plans, {:?}", plans);
    build_plans(
        &req,
        &hook.repository.clone_url,
        &hook.pusher.name,
        account_id,
        plans,
    )
}

fn build_plans(
    req: &HttpRequest<AppState>,
    repo_url: &str,
    pusher: &str,
    account_id: Option<u64>,
    plans: Vec<Plan>,
) -> Result<HttpResponse> {
    let mut request = JobGroupSpec::new();

    for plan in plans.iter() {
        let mut project_get = OriginProjectGet::new();
        project_get.set_name(format!("{}/{}", &plan.origin, &plan.name));

        match route_message::<OriginProjectGet, OriginProject>(&req, &project_get) {
            Ok(project) => {
                if repo_url != project.get_vcs_data() {
                    warn!(
                        "Repo URL ({}) doesn't match project vcs data ({}). Aborting.",
                        repo_url,
                        project.get_vcs_data()
                    );
                    continue;
                }
            }
            Err(err) => {
                warn!("Failed to fetch project, {}", err);
                continue;
            }
        }

        debug!("Scheduling, {:?}", plan);
        request.set_origin(plan.origin.clone());
        request.set_package(plan.name.clone());
        // JW TODO: We need to be able to determine which platform this build is for based on
        // the directory structure the plan is found in or metadata inside the plan. We will need
        // to have this done before we support building additional targets with Builder.
        request.set_target("x86_64-linux".to_string());
        request.set_trigger(JobGroupTrigger::Webhook);
        request.set_requester_name(pusher.to_string());
        if account_id.is_some() {
            request.set_requester_id(account_id.unwrap());
        }

        match route_message::<JobGroupSpec, JobGroup>(&req, &request) {
            Ok(group) => debug!("JobGroup created, {:?}", group),
            Err(err) => debug!("Failed to create group, {:?}", err),
        }
    }
    Ok(HttpResponse::Ok().json(&plans))
}

fn read_bldr_config(github: &GitHubClient, token: &AppToken, hook: &GitHubWebhookPush) -> BuildCfg {
    match github.contents(&token, hook.repository.id, BLDR_CFG) {
        Ok(Some(contents)) => match contents.decode() {
            Ok(ref bytes) => match BuildCfg::from_slice(bytes) {
                Ok(cfg) => cfg,
                Err(err) => {
                    debug!("unable to parse bldr.toml, {}", err);
                    BuildCfg::default()
                }
            },
            Err(err) => {
                debug!("unable to read bldr.toml, {}", err);
                BuildCfg::default()
            }
        },
        Ok(None) => BuildCfg::default(),
        Err(err) => {
            warn!("unable to retrieve bldr.toml, {}", err);
            BuildCfg::default()
        }
    }
}

fn read_plans(
    github: &GitHubClient,
    token: &AppToken,
    hook: &GitHubWebhookPush,
    config: &BuildCfg,
) -> Vec<Plan> {
    let mut plans = Vec::with_capacity(config.projects().len());
    for project in config.triggered_by(hook.branch(), hook.changed().as_slice()) {
        if let Some(plan) = read_plan(github, &token, hook, &project.plan_file().to_string_lossy())
        {
            plans.push(plan)
        }
    }
    plans
}

fn read_plan(
    github: &GitHubClient,
    token: &AppToken,
    hook: &GitHubWebhookPush,
    path: &str,
) -> Option<Plan> {
    match github.contents(&token, hook.repository.id, path) {
        Ok(Some(contents)) => match contents.decode() {
            Ok(bytes) => match Plan::from_bytes(bytes.as_slice()) {
                Ok(plan) => Some(plan),
                Err(err) => {
                    debug!("unable to read plan, {}, {}", path, err);
                    None
                }
            },
            Err(err) => {
                debug!("unable to read plan, {}, {}", path, err);
                None
            }
        },
        Ok(None) => None,
        Err(err) => {
            warn!("unable to retrieve plan, {}, {}", path, err);
            None
        }
    }
}
