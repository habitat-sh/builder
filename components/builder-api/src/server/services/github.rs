use crate::{
    bldr_core::{
        build_config::{BuildCfg, BLDR_CFG},
        metrics::CounterMetric,
    },
    db::models::{account::Account, projects::Project},
    hab_core::{
        crypto,
        package::{
            target::{self, PackageTarget},
            Plan,
        },
    },
    protocol::jobsrv::{JobGroup, JobGroupSpec, JobGroupTrigger},
    server::{
        authorize::authorize_session,
        error::{Error, Result},
        feat,
        framework::{headers, middleware::route_message},
        helpers::req_state,
        services::metrics::Counter,
        AppState,
    },
};
use actix_web::{
    error,
    http::StatusCode,
    web::{Data, Path},
    HttpRequest, HttpResponse,
};
use github_api_client::{types::GitHubWebhookPush, AppToken, GitHubClient};
use hex;
use openssl::{hash::MessageDigest, pkey::PKey, sign::Signer};
use serde_json;
use std::{collections::HashSet, path::PathBuf, str::FromStr};

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
            _ => {
                warn!("Received an unknown GitHub event type");
                Err(Error::BadRequest)
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PlanWithTarget(Plan, PackageTarget);

#[allow(clippy::needless_pass_by_value)]
pub async fn handle_event(req: HttpRequest, body: String) -> HttpResponse {
    Counter::GitHubEvent.increment();

    let event = match req.headers().get(headers::XGITHUBEVENT) {
        Some(event) => {
            let event_str = match event.to_str() {
                Ok(value) => value,
                Err(err) => {
                    warn!("Unable to read XGithubEvent header, {:?}", err);
                    return Error::BadRequest.into();
                }
            };

            match GitHubEvent::from_str(event_str) {
                Ok(event) => event,
                Err(err) => {
                    warn!("Unable to parse XGithubEvent header, {:?}", err);
                    return err.into();
                }
            }
        }
        None => {
            warn!("Received a GitHub hook with no XGithubEvent header");
            return Error::BadRequest.into();
        }
    };

    // Authenticate the hook
    let github = &req_state(&req).github;
    let gh_signature = match req.headers().get(headers::XHUBSIGNATURE) {
        Some(sig) => sig.clone(),
        None => {
            warn!("Received a GitHub hook with no signature");
            return Error::BadRequest.into();
        }
    };

    trace!("handle-notify, {}", body);

    let key = PKey::hmac(github.webhook_secret.as_bytes()).unwrap();
    let mut signer = Signer::new(MessageDigest::sha1(), &key).unwrap();
    signer.update(body.as_bytes()).unwrap();
    let hmac = signer.sign_to_vec().unwrap();
    let computed_signature = format!("sha1={}", &hex::encode(hmac));

    if !crypto::secure_eq(&gh_signature, &computed_signature) {
        warn!(
            "Web hook signatures don't match. GH = {:?}, Our = {:?}",
            gh_signature, computed_signature
        );
        return Error::BadRequest.into();
    }

    match event {
        GitHubEvent::Ping => HttpResponse::new(StatusCode::OK),
        GitHubEvent::Push => handle_push(&req, &body).await,
    }
}

#[allow(clippy::needless_pass_by_value)]
pub async fn repo_file_content(
    req: HttpRequest,
    path: Path<(u32, u32, String)>,
    state: Data<AppState>,
) -> HttpResponse {
    if let Err(err) = authorize_session(&req, None, None) {
        return err.into();
    }

    let github = &state.github;
    let (install_id, repo_id, path) = path.into_inner();

    let token = {
        match github.app_installation_token(install_id).await {
            Ok(token) => token,
            Err(err) => {
                warn!("Unable to generate GitHub app token, {:?}", err);
                return Error::Github(err).into();
            }
        }
    };

    match github.contents(&token, repo_id, &path).await {
        Ok(None) => HttpResponse::new(StatusCode::NOT_FOUND),
        Ok(search) => HttpResponse::Ok().json(&search),
        Err(err) => {
            warn!("Unable to fetch GitHub contents, {:?}", err);
            Error::Github(err).into()
        }
    }
}

async fn handle_push(req: &HttpRequest, body: &str) -> HttpResponse {
    let hook = match serde_json::from_str::<GitHubWebhookPush>(&body) {
        Ok(hook) => hook,
        Err(err) => return Error::SerdeJson(err).into(),
    };
    debug!(
        "Received GitHub web hook; sender={}, repository={} repository_id={} ref={} \
            installation_id={}",
        hook.sender.login,
        hook.repository.full_name,
        hook.repository.id,
        hook.git_ref,
        hook.installation.id
    );

    let conn = match req_state(req).db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(e) => return Error::DbError(e).into(),
    };

    if hook.commits.is_empty() {
        debug!("GitHub web hook does not have any commits!");
        return HttpResponse::new(StatusCode::OK);
    }

    let github = &req_state(req).github;

    let token = match github.app_installation_token(hook.installation.id).await {
        Ok(token) => token,
        Err(err) => {
            warn!("Unable to generate GitHub app token, {:?}", err);
            return Error::Github(err).into();
        }
    };

    let account_id = match Account::get(&hook.pusher.name.clone(), &*conn) {
        Ok(account) => Some(account.id as u64),
        Err(_) => None,
    };

    let config = match read_bldr_config(&github, &token, &hook).await {
        Ok(config) => config,
        Err(err) => return err.into(),
    };
    debug!("Config: {:#?}", config);

    let plans = match read_plans(&github, &token, &hook, &config).await {
        Ok(plans) => plans,
        Err(err) => return err.into(),
    };
    debug!("Triggered Plans: {:#?}", plans);

    build_plans(
        &req,
        &hook.repository.clone_url,
        &hook.pusher.name,
        account_id,
        &plans,
    )
    .await
}

async fn build_plans(
    req: &HttpRequest,
    repo_url: &str,
    pusher: &str,
    account_id: Option<u64>,
    plans: &[PlanWithTarget],
) -> HttpResponse {
    let mut request = JobGroupSpec::new();

    let conn = match req_state(req).db.get_conn() {
        Ok(conn_ref) => conn_ref,
        Err(e) => return Error::DbError(e).into(),
    };

    for plan in plans.iter() {
        let project_name = format!("{}/{}", &plan.0.origin, &plan.0.name);
        match Project::get(&project_name, &plan.1, &*conn) {
            Ok(project) => {
                if repo_url != project.vcs_data {
                    warn!(
                        "Repo URL ({}) doesn't match project vcs data ({}). Aborting.",
                        repo_url, project.vcs_data
                    );
                    continue;
                }
            }
            Err(err) => {
                debug!(
                    "Failed to fetch project (plan may not be connected): {}, {:?}",
                    project_name, err
                );
                continue;
            }
        }

        if !req_state(req).config.api.build_targets.contains(&plan.1) {
            debug!("Rejecting build with target: {:?}", plan.1);
            continue;
        }

        if feat::is_enabled(feat::Jobsrv) {
            debug!("Scheduling, {:?} ({})", plan.0, plan.1);
            request.set_origin(plan.0.origin.clone());
            request.set_package(plan.0.name.clone());
            request.set_target(plan.1.to_string());
            request.set_trigger(JobGroupTrigger::Webhook);
            request.set_requester_name(pusher.to_string());

            if let Some(a) = account_id {
                request.set_requester_id(a);
            }

            match route_message::<JobGroupSpec, JobGroup>(&req, &request).await {
                Ok(group) => debug!("JobGroup created, {:?}", group),
                Err(err) => debug!("Failed to create group, {:?}", err),
            }
        } else {
            debug!("Skipping scheduling build for {:?} (jobsrv disabled)", plan);
        }
    }

    debug!("Returning success response with {} plans", plans.len());
    HttpResponse::Ok().json(&plans)
}

async fn read_bldr_config(
    github: &GitHubClient,
    token: &AppToken,
    hook: &GitHubWebhookPush,
) -> Result<BuildCfg> {
    match github
        .contents(&token, hook.repository.id, BLDR_CFG)
        .await
        .map_err(Error::Github)
    {
        Ok(Some(contents)) => {
            debug!("Found a bldr.toml for repo {}", hook.repository.full_name);
            match contents.decode().map_err(Error::Github) {
                Ok(ref bytes) => match BuildCfg::from_slice(bytes).map_err(Error::BuilderCore) {
                    Ok(cfg) => Ok(cfg),
                    Err(err) => {
                        warn!("Unable to parse bldr.toml, {:?}", err);
                        Err(err)
                    }
                },
                Err(err) => {
                    warn!("Unable to decode bldr.toml, {:?}", err);
                    Err(err)
                }
            }
        }
        Ok(None) => {
            debug!(
                "No bldr.toml found for repo {}, using default config",
                hook.repository.full_name
            );
            Ok(BuildCfg::default())
        }
        Err(err) => {
            warn!("Unable to retrieve bldr.toml, {:?}", err);
            Err(err)
        }
    }
}

async fn read_plans(
    github: &GitHubClient,
    token: &AppToken,
    hook: &GitHubWebhookPush,
    bldr_cfg: &BuildCfg,
) -> Result<Vec<PlanWithTarget>> {
    let mut plans = Vec::with_capacity(bldr_cfg.projects().len());

    for project_cfg in bldr_cfg.triggered_by(hook.branch(), hook.changed().as_slice()) {
        for plan_path in project_cfg.plan_path_candidates() {
            debug!("Checking targets for plan_path candidate {:?}", plan_path);
            let targets = read_plan_targets(github, token, hook, &plan_path).await?;

            for target in targets {
                if project_cfg.build_targets.contains(&target) {
                    debug!("Project config contains target: {}", target);
                    let plan_file = if target == target::X86_64_WINDOWS {
                        "plan.ps1"
                    } else {
                        "plan.sh"
                    };
                    let plan_path = plan_path.join(plan_file);

                    if let Some(plan) =
                        read_plan(github, &token, hook, &plan_path.to_string_lossy()).await?
                    {
                        plans.push(PlanWithTarget(plan, target));
                    }
                }
            }
        }
    }

    Ok(plans)
}

async fn read_plan(
    github: &GitHubClient,
    token: &AppToken,
    hook: &GitHubWebhookPush,
    path: &str,
) -> Result<Option<Plan>> {
    debug!("Reading plan from: {:?}", path);

    match github
        .contents(&token, hook.repository.id, path)
        .await
        .map_err(Error::Github)
    {
        Ok(Some(contents)) => match contents.decode().map_err(Error::Github) {
            Ok(bytes) => match Plan::from_bytes(bytes.as_slice()).map_err(Error::HabitatCore) {
                Ok(plan) => Ok(Some(plan)),
                Err(err) => {
                    warn!("Failed to parse plan: {}, {:?}", path, err);
                    Err(err)
                }
            },
            Err(err) => {
                warn!("Failed to decode plan bytes: {}, {:?}", path, err);
                Err(err)
            }
        },
        Ok(None) => Ok(None),
        Err(err) => {
            warn!("Unable to read plan: {}, {:?}", path, err);
            Err(err)
        }
    }
}

async fn read_plan_targets(
    github: &GitHubClient,
    token: &AppToken,
    hook: &GitHubWebhookPush,
    path: &PathBuf,
) -> Result<HashSet<PackageTarget>> {
    debug!("Reading plan targets from {:?}", path);
    let mut targets: HashSet<PackageTarget> = HashSet::new();

    match github
        .directory(&token, hook.repository.id, &path.to_string_lossy())
        .await
        .map_err(Error::Github)
    {
        Ok(Some(entries)) => {
            for entry in entries {
                match &entry.name[..] {
                    "plan.ps1" => {
                        targets.insert(target::X86_64_WINDOWS);
                    }
                    "plan.sh" => {
                        targets.insert(target::X86_64_LINUX);
                        targets.insert(target::X86_64_LINUX_KERNEL2);
                    }
                    _ => (),
                }
            }
        }
        Ok(None) => debug!("No plan directory found for: {:?}", path),
        Err(err) => {
            warn!("Failed to read plan directory: {:?}, {:?}", path, err);
        }
    }

    Ok(targets)
}
