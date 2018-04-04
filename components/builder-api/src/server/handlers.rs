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

//! A collection of handlers for the HTTP server's router

use std::env;

use bodyparser;
use bldr_core;
use bldr_core::helpers::transition_visibility;
use hab_core::package::{Identifiable, Plan};
use http_client::ApiClient;
use oauth_common::OAuthError;
use http_gateway::http::controller::*;
use http_gateway::http::helpers::{self, check_origin_access, get_param, validate_params};
use hyper::header::{Accept, ContentType};
use hyper::status::StatusCode;
use iron::status;
use params::{FromValue, Params};
use persistent;
use protocol::jobsrv::{Job, JobGet, JobLogGet, JobLog, JobState, ProjectJobsGet,
                       ProjectJobsGetResponse, JobGroupCancel, JobGroupGet, JobGroup};
use protocol::jobsrv::{JobGraphPackageReverseDependenciesGet, JobGraphPackageReverseDependencies};
use protocol::originsrv::*;
use protocol::sessionsrv::{Account, AccountGet, AccountGetId, AccountInvitationListRequest,
                           AccountInvitationListResponse, AccountOriginListRequest,
                           AccountOriginListResponse, AccountUpdate, AccountToken,
                           AccountTokenCreate, AccountTokensGet, AccountTokens, AccountTokenRevoke};
use router::Router;
use serde_json;

use config::Config;
use github;
use headers::*;
use types::*;
use super::SegmentCli;

// A default name for per-project integrations. Currently, there
// can only be one.
const DEFAULT_PROJECT_INTEGRATION: &'static str = "default";

const PRODUCT: &'static str = "builder-api";
const VERSION: &'static str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

#[derive(Clone, Serialize, Deserialize)]
struct SearchTerm {
    attr: String,
    entity: String,
    value: String,
}

pub fn account_show(req: &mut Request) -> IronResult<Response> {
    let mut account_get_id = AccountGetId::new();
    {
        let params = req.extensions.get::<Router>().unwrap();
        let stringy_id = params.find("id").unwrap();
        match stringy_id.parse::<u64>() {
            Ok(id) => account_get_id.set_id(id),
            Err(_) => return Ok(Response::with(status::BadRequest)),
        }
    }
    match route_message::<AccountGetId, Account>(req, &account_get_id) {
        Ok(account) => Ok(render_json(status::Ok, &account)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn search(req: &mut Request) -> IronResult<Response> {
    match req.get::<bodyparser::Struct<SearchTerm>>() {
        Ok(Some(body)) => {
            match &*body.entity.to_lowercase() {
                "account" => search_account(req, body.attr, body.value),
                entity => {
                    Ok(Response::with((
                        status::UnprocessableEntity,
                        format!("Unknown search entity: {}", entity),
                    )))
                }
            }
        }
        _ => Ok(Response::with(status::UnprocessableEntity)),
    }
}

fn search_account(req: &mut Request, key: String, value: String) -> IronResult<Response> {
    match key.as_str() {
        "id" => {
            let mut account_get_id = AccountGetId::new();
            let id = match value.parse::<u64>() {
                Ok(id) => id,
                Err(_) => return Ok(Response::with(status::BadRequest)),
            };
            account_get_id.set_id(id);
            match route_message::<AccountGetId, Account>(req, &account_get_id) {
                Ok(account) => Ok(render_json(status::Ok, &account)),
                Err(err) => Ok(render_net_error(&err)),
            }
        }
        "name" => {
            let mut account_get = AccountGet::new();
            account_get.set_name(value);
            match route_message::<AccountGet, Account>(req, &account_get) {
                Ok(account) => Ok(render_json(status::Ok, &account)),
                Err(err) => Ok(render_net_error(&err)),
            }
        }
        _ => Ok(Response::with(status::UnprocessableEntity)),
    }
}

pub fn authenticate(req: &mut Request) -> IronResult<Response> {
    println!("AUTHENTICATE CALLED");

    let code = match get_param(req, "code") {
        Some(c) => c,
        None => return Ok(Response::with(status::BadRequest)),
    };

    // TBD -
    // let state = match get_param(req, "state") {
    //     Some(s) => s,
    //     None => return Ok(Response::with(status::BadRequest)),
    // };

    if env::var_os("HAB_FUNC_TEST").is_some() {
        let session = {
            session_create_short_circuit(req, &code)?
        };
        return Ok(render_json(status::Ok, &session));
    }

    let oauth = req.get::<persistent::Read<OAuthCli>>().unwrap();
    let segment = req.get::<persistent::Read<SegmentCli>>().unwrap();

    match oauth.authenticate(&code, "state") {
        Ok((token, user)) => {
            let session = session_create_oauth(req, &token, &user, &oauth.config.provider)?;

            // We don't really want to abort anything just because a call to segment failed. Let's
            // just log it and move on.
            // TODO JB: this likely needs to change after we switch to our own internal session
            // tokens
            let id_str = session.get_id().to_string();
            if let Err(e) = segment.identify(&id_str) {
                warn!("Error identifying a user in segment, {}", e);
            }

            Ok(render_json(status::Ok, &session))
        }
        /* TBD - Fix
        Err(OAuthError::HttpResponse(code, response)) => {
            let msg = format!("{}-{}", code, response);
            let err = NetError::new(ErrCode::ACCESS_DENIED, msg);
            Ok(render_net_error(&err))
        }
        Err(OAuthError::Serialization(e)) => {
            warn!("bad reply from our oauth provider, {}", e);
            let err = NetError::new(ErrCode::BAD_REMOTE_REPLY, "rg:auth:1");
            Ok(render_net_error(&err))
        }
        */
        Err(e) => {
            warn!("unhandled authentication error, {:?}", e);
            let err = NetError::new(ErrCode::BUG, "rg:auth:2");
            Ok(render_net_error(&err))
        }
    }
}

pub fn update_profile(req: &mut Request) -> IronResult<Response> {
    let session_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    let body = match req.get::<bodyparser::Struct<UserUpdateReq>>() {
        Ok(Some(body)) => {
            if body.email.len() <= 0 {
                return Ok(Response::with((
                    status::UnprocessableEntity,
                    "Missing value for field: `email`",
                )));
            }

            body
        }
        _ => return Ok(Response::with(status::UnprocessableEntity)),
    };

    let mut request = AccountUpdate::new();
    request.set_id(session_id);
    request.set_email(body.email);

    match route_message::<AccountUpdate, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::Ok)),
        Err(err) => return Ok(render_net_error(&err)),
    }
}

pub fn get_profile(req: &mut Request) -> IronResult<Response> {
    let session_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    let mut request = AccountGetId::new();
    request.set_id(session_id);

    match route_message::<AccountGetId, Account>(req, &request) {
        Ok(account) => Ok(render_json(status::Ok, &account)),
        Err(err) => return Ok(render_net_error(&err)),
    }
}

pub fn get_access_tokens(req: &mut Request) -> IronResult<Response> {
    let session_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    let mut request = AccountTokensGet::new();
    request.set_account_id(session_id);

    match route_message::<AccountTokensGet, AccountTokens>(req, &request) {
        Ok(account_tokens) => Ok(render_json(status::Ok, &account_tokens)),
        Err(err) => return Ok(render_net_error(&err)),
    }
}

pub fn generate_access_token(req: &mut Request) -> IronResult<Response> {
    let (session_id, flags) = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        (session.get_id(), session.get_flags())
    };

    let mut request = AccountGetId::new();
    request.set_id(session_id);

    let account = match route_message::<AccountGetId, Account>(req, &request) {
        Ok(account) => account,
        Err(err) => return Ok(render_net_error(&err)),
    };

    let mut request = AccountTokenCreate::new();
    let cfg = req.get::<persistent::Read<Config>>().unwrap();
    let token =
        bldr_core::access_token::generate_user_token(&cfg.depot.key_dir, account.get_id(), flags)
            .unwrap();

    request.set_account_id(account.get_id());
    request.set_token(token);

    match route_message::<AccountTokenCreate, AccountToken>(req, &request) {
        Ok(account_token) => Ok(render_json(status::Ok, &account_token)),
        Err(err) => return Ok(render_net_error(&err)),
    }
}

pub fn revoke_access_token(req: &mut Request) -> IronResult<Response> {
    let token_id = match get_param(req, "id") {
        Some(id) => {
            match id.parse::<u64>() {
                Ok(n) => n,
                Err(e) => {
                    debug!("Bad id param. e = {:?}", e);
                    return Ok(Response::with(status::BadRequest));
                }
            }
        }
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut request = AccountTokenRevoke::new();
    request.set_id(token_id);

    match route_message::<AccountTokenRevoke, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::Ok)),
        Err(err) => return Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn job_group_promote(req: &mut Request) -> IronResult<Response> {
    job_group_promote_or_demote(req, true)

}

// This route is only available if jobsrv_enabled is true
pub fn job_group_demote(req: &mut Request) -> IronResult<Response> {
    job_group_promote_or_demote(req, false)
}

// This route is only available if jobsrv_enabled is true
fn job_group_promote_or_demote(req: &mut Request, promote: bool) -> IronResult<Response> {
    let group_id = match get_param(req, "id") {
        Some(id) => {
            match id.parse::<u64>() {
                Ok(g) => g,
                Err(e) => {
                    debug!("Error finding group. e = {:?}", e);
                    return Ok(Response::with(status::BadRequest));
                }
            }
        }
        None => return Ok(Response::with(status::BadRequest)),
    };

    let channel = match get_param(req, "channel") {
        Some(c) => c,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let idents = if promote {
        match req.get::<bodyparser::Struct<GroupPromoteReq>>() {
            Ok(Some(gpr)) => Some(gpr.idents),
            Ok(None) => None,
            Err(err) => {
                debug!("Error decoding json struct: {:?}", err);
                return Ok(Response::with(status::BadRequest));
            }
        }
    } else {
        match req.get::<bodyparser::Struct<GroupDemoteReq>>() {
            Ok(Some(gpr)) => Some(gpr.idents),
            Ok(None) => None,
            Err(err) => {
                debug!("Error decoding json struct: {:?}", err);
                return Ok(Response::with(status::BadRequest));
            }
        }
    };

    match helpers::promote_or_demote_job_group(req, group_id, idents, &channel, promote) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn job_group_cancel(req: &mut Request) -> IronResult<Response> {
    let group_id = match get_param(req, "id") {
        Some(id) => {
            match id.parse::<u64>() {
                Ok(g) => g,
                Err(e) => {
                    debug!("Error finding group. e = {:?}", e);
                    return Ok(Response::with(status::BadRequest));
                }
            }
        }
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut jgg = JobGroupGet::new();
    jgg.set_group_id(group_id);
    jgg.set_include_projects(true);

    let group = match route_message::<JobGroupGet, JobGroup>(req, &jgg) {
        Ok(group) => group,
        Err(err) => return Ok(render_net_error(&err)),
    };

    let name_split: Vec<&str> = group.get_project_name().split("/").collect();
    assert!(name_split.len() == 2);

    if !check_origin_access(req, &name_split[0]).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut jgc = JobGroupCancel::new();
    jgc.set_group_id(group_id);

    match route_message::<JobGroupCancel, NetOk>(req, &jgc) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn validate_registry_credentials(req: &mut Request) -> IronResult<Response> {
    let json_body = req.get::<bodyparser::Json>();

    let registry_type: String = match get_param(req, "registry_type") {
        Some(t) => t,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let body = match json_body {
        Ok(Some(b)) => b,
        Ok(None) => {
            debug!("Error: Missing request body");
            return Ok(Response::with(status::BadRequest));
        }
        Err(err) => {
            debug!("Error: {:?}", err);
            return Ok(Response::with(status::BadRequest));
        }
    };

    if !body["username"].is_string() || !body["password"].is_string() {
        debug!("Error: Missing username or password");
        return Ok(Response::with(status::BadRequest));
    }

    let url = match body["url"].as_str() {
        Some(url) => url,
        None => {
            match registry_type.as_ref() {
                "docker" => "https://hub.docker.com/v2",
                _ => return Ok(Response::with(status::BadRequest)),
            }
        }
    };

    let client = match ApiClient::new(url, PRODUCT, VERSION, None) {
        Ok(c) => c,
        Err(e) => {
            debug!("Error: Unable to create HTTP client: {}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    };

    let sbody = serde_json::to_string(&body).unwrap();
    let result = client
        .post("users/login")
        .header(Accept::json())
        .header(ContentType::json())
        .body(&sbody)
        .send();

    match result {
        Ok(response) => {
            match response.status {
                StatusCode::Ok => Ok(Response::with(status::NoContent)),
                _ => {
                    debug!("Non-OK Response: {}", &response.status);
                    Ok(Response::with(response.status))
                }
            }
        }
        Err(e) => {
            debug!("Error sending request: {:?}", e);
            Ok(Response::with(status::Forbidden))
        }
    }
}

// This route is only available if jobsrv_enabled is true
pub fn project_privacy_toggle(req: &mut Request) -> IronResult<Response> {
    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let name = match get_param(req, "name") {
        Some(n) => n,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let vis = match get_param(req, "visibility") {
        Some(v) => v,
        None => return Ok(Response::with(status::BadRequest)),
    };

    // users aren't allowed to set projects to hidden manually
    if vis.to_lowercase() == "hidden" {
        return Ok(Response::with(status::BadRequest));
    }

    let opv: OriginPackageVisibility = match vis.parse() {
        Ok(o) => o,
        Err(_) => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut project_get = OriginProjectGet::new();
    project_get.set_name(format!("{}/{}", origin, name));

    match route_message::<OriginProjectGet, OriginProject>(req, &project_get) {
        Ok(mut project) => {
            let real_visibility = transition_visibility(opv, project.get_visibility());
            let mut opu = OriginProjectUpdate::new();
            project.set_visibility(real_visibility);
            opu.set_project(project);

            match route_message::<OriginProjectUpdate, NetOk>(req, &opu) {
                Ok(_) => Ok(Response::with(status::NoContent)),
                Err(err) => Ok(render_net_error(&err)),
            }
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn rdeps_show(req: &mut Request) -> IronResult<Response> {
    let mut rdeps_get = JobGraphPackageReverseDependenciesGet::new();
    match get_param(req, "origin") {
        Some(origin) => rdeps_get.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "name") {
        Some(name) => rdeps_get.set_name(name),
        None => return Ok(Response::with(status::BadRequest)),
    }

    // TODO (SA): The rdeps API needs to be extended to support a target param.
    // For now, hard code a default value
    rdeps_get.set_target("x86_64-linux".to_string());

    match route_message::<
        JobGraphPackageReverseDependenciesGet,
        JobGraphPackageReverseDependencies,
    >(req, &rdeps_get) {
        Ok(rdeps) => Ok(render_json(status::Ok, &rdeps)),
        Err(err) => return Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn job_show(req: &mut Request) -> IronResult<Response> {
    let mut request = JobGet::new();
    match get_param(req, "id") {
        Some(id) => {
            match id.parse::<u64>() {
                Ok(i) => request.set_id(i),
                Err(e) => {
                    debug!("Error finding id. e = {:?}", e);
                    return Ok(Response::with(status::BadRequest));
                }
            }
        }
        None => return Ok(Response::with(status::BadRequest)),
    }

    match route_message::<JobGet, Job>(req, &request) {
        Ok(job) => {
            debug!("job = {:?}", &job);

            if !check_origin_access(req, job.get_project().get_origin_name()).unwrap_or(false) {
                return Ok(Response::with(status::Forbidden));
            }

            if job.get_package_ident().fully_qualified() {
                let channels = helpers::channels_for_package_ident(req, job.get_package_ident());
                let platforms = helpers::platforms_for_package_ident(req, job.get_package_ident());
                let mut job_json = serde_json::to_value(job).unwrap();

                if channels.is_some() {
                    job_json["channels"] = json!(channels);
                }

                if platforms.is_some() {
                    job_json["platforms"] = json!(platforms);
                }

                Ok(render_json(status::Ok, &job_json))
            } else {
                Ok(render_json(status::Ok, &job))
            }
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn job_log(req: &mut Request) -> IronResult<Response> {
    let start = req.get_ref::<Params>()
        .unwrap()
        .find(&["start"])
        .and_then(FromValue::from_value)
        .unwrap_or(0);

    let include_color = req.get_ref::<Params>()
        .unwrap()
        .find(&["color"])
        .and_then(FromValue::from_value)
        .unwrap_or(false);

    let mut job_get = JobGet::new();
    let mut request = JobLogGet::new();
    request.set_start(start);

    match get_param(req, "id") {
        Some(id) => {
            match id.parse::<u64>() {
                Ok(i) => {
                    request.set_id(i);
                    job_get.set_id(i);
                }
                Err(e) => {
                    debug!("Error parsing id. e = {:?}", e);
                    return Ok(Response::with(status::BadRequest));
                }
            }
        }
        None => return Ok(Response::with(status::BadRequest)),
    }

    // Before fetching the logs, we need to check and see if the logs we want to fetch are for
    // a job that's building a private package, and if so, do we have the right to see said
    // package.
    match route_message::<JobGet, Job>(req, &job_get) {
        Ok(job) => {
            // It's not sufficient to check the project that's on the job itself, since that
            // project is reconstructed from information available in the jobsrv database and does
            // not contain things like visibility settings. We need to fetch the project from
            // originsrv.
            let mut project_get = OriginProjectGet::new();
            project_get.set_name(job.get_project().get_name().to_string());

            let project =
                match route_message::<OriginProjectGet, OriginProject>(req, &project_get) {
                    Ok(p) => p,
                    Err(err) => return Ok(render_net_error(&err)),
                };

            if vec![
                OriginPackageVisibility::Private,
                OriginPackageVisibility::Hidden,
            ].contains(&project.get_visibility())
            {
                if !check_origin_access(req, project.get_origin_name()).unwrap_or(false) {
                    return Ok(Response::with(status::Forbidden));
                }
            }

            match route_message::<JobLogGet, JobLog>(req, &request) {
                Ok(mut log) => {
                    if !include_color {
                        log.strip_ansi();
                    }
                    Ok(render_json(status::Ok, &log))
                }
                Err(err) => Ok(render_net_error(&err)),
            }
        }
        Err(e) => return Ok(render_net_error(&e)),
    }
}

pub fn notify(req: &mut Request) -> IronResult<Response> {
    if req.headers.has::<XGitHubEvent>() {
        return github::handle_event(req);
    }
    Ok(Response::with(status::BadRequest))
}

/// Endpoint for determining availability of builder-api components.
///
/// Returns a status 200 on success. Any non-200 responses are an outage or a partial outage.
pub fn status(_req: &mut Request) -> IronResult<Response> {
    Ok(Response::with(status::Ok))
}

pub fn list_account_invitations(req: &mut Request) -> IronResult<Response> {
    let mut request = AccountInvitationListRequest::new();
    {
        let session = req.extensions.get::<Authenticated>().unwrap();
        request.set_account_id(session.get_id());
    }
    match route_message::<AccountInvitationListRequest, AccountInvitationListResponse>(
        req,
        &request,
    ) {
        Ok(invites) => Ok(render_json(status::Ok, &invites)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

pub fn list_user_origins(req: &mut Request) -> IronResult<Response> {
    let mut request = AccountOriginListRequest::new();
    {
        let session = req.extensions.get::<Authenticated>().unwrap();
        request.set_account_id(session.get_id());
    }
    match route_message::<AccountOriginListRequest, AccountOriginListResponse>(req, &request) {
        Ok(invites) => Ok(render_json(status::Ok, &invites)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Create a new project as the authenticated user and associated to
/// the given origin.
// This route is only available if jobsrv_enabled is true
pub fn project_create(req: &mut Request) -> IronResult<Response> {
    let mut request = OriginProjectCreate::new();
    let mut project = OriginProject::new();
    let mut origin_get = OriginGet::new();
    let github = req.get::<persistent::Read<GitHubCli>>().unwrap();
    let session = req.extensions.get::<Authenticated>().unwrap().clone();

    let (token, repo_id) = match req.get::<bodyparser::Struct<ProjectCreateReq>>() {
        Ok(Some(body)) => {
            if body.origin.len() <= 0 {
                return Ok(Response::with((
                    status::UnprocessableEntity,
                    "Missing value for field: `origin`",
                )));
            }
            if body.plan_path.len() <= 0 {
                return Ok(Response::with((
                    status::UnprocessableEntity,
                    "Missing value for field: `plan_path`",
                )));
            }

            if !check_origin_access(req, &body.origin).unwrap_or(false) {
                return Ok(Response::with(status::Forbidden));
            }

            debug!(
                "GITHUB-CALL builder_api::server::handlers::project_create: Getting app_installation_token; repo_id={} installation_id={}",
                body.repo_id,
                body.installation_id
            );

            let token = match github.app_installation_token(body.installation_id) {
                Ok(token) => token,
                Err(err) => {
                    warn!("Error authenticating github app installation, {}", err);
                    return Ok(Response::with(status::Forbidden));
                }
            };

            origin_get.set_name(body.origin);
            project.set_plan_path(body.plan_path);
            project.set_vcs_type(String::from("git"));
            project.set_vcs_installation_id(body.installation_id);

            match github.repo(&token, body.repo_id) {
                Ok(Some(repo)) => project.set_vcs_data(repo.clone_url),
                Ok(None) => return Ok(Response::with((status::NotFound, "rg:pc:2"))),
                Err(e) => {
                    warn!("Error finding github repo. e = {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "rg:pc:1")));
                }
            }
            (token, body.repo_id)
        }
        _ => return Ok(Response::with(status::UnprocessableEntity)),
    };

    let origin = match route_message::<OriginGet, Origin>(req, &origin_get) {
        Ok(response) => response,
        Err(err) => return Ok(render_net_error(&err)),
    };

    match github.contents(&token, repo_id, &project.get_plan_path()) {
        Ok(Some(contents)) => {
            match contents.decode() {
                Ok(bytes) => {
                    match Plan::from_bytes(bytes.as_slice()) {
                        Ok(plan) => {
                            project.set_origin_name(String::from(origin.get_name()));
                            project.set_origin_id(origin.get_id());
                            project.set_package_name(String::from(plan.name.trim_matches('"')));
                        }
                        Err(e) => {
                            debug!("Error matching Plan. e = {:?}", e);
                            return Ok(Response::with((status::UnprocessableEntity, "rg:pc:3")));
                        }
                    }
                }
                Err(e) => {
                    error!("Base64 decode failure: {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "rg:pc:4")));
                }
            }
        }
        Ok(None) => return Ok(Response::with((status::NotFound, "rg:pc:5"))),
        Err(e) => {
            warn!("Error fetching contents from GH. e = {:?}", e);
            return Ok(Response::with((status::UnprocessableEntity, "rg:pc:2")));
        }
    }

    project.set_owner_id(session.get_id());
    project.set_visibility(origin.get_default_package_visibility());
    request.set_project(project);
    match route_message::<OriginProjectCreate, OriginProject>(req, &request) {
        Ok(response) => Ok(render_json(status::Created, &response)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Delete the given project
// This route is only available if jobsrv_enabled is true
pub fn project_delete(req: &mut Request) -> IronResult<Response> {
    let mut project_del = OriginProjectDelete::new();

    let session_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let name = match get_param(req, "name") {
        Some(n) => n,
        None => return Ok(Response::with(status::BadRequest)),
    };

    project_del.set_name(format!("{}/{}", &origin, &name));

    if !check_origin_access(req, origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    project_del.set_requestor_id(session_id);
    match route_message::<OriginProjectDelete, NetOk>(req, &project_del) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Update the given project
// This route is only available if jobsrv_enabled is true
pub fn project_update(req: &mut Request) -> IronResult<Response> {
    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let name = match get_param(req, "name") {
        Some(n) => n,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let session_id = {
        let session = req.extensions.get::<Authenticated>().unwrap();
        session.get_id()
    };

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut project_get = OriginProjectGet::new();
    project_get.set_name(format!("{}/{}", &origin, &name));
    let mut project = match route_message::<OriginProjectGet, OriginProject>(req, &project_get) {
        Ok(project) => project,
        Err(err) => return Ok(render_net_error(&err)),
    };

    let mut request = OriginProjectUpdate::new();
    let github = req.get::<persistent::Read<GitHubCli>>().unwrap();

    let (token, repo_id) = match req.get::<bodyparser::Struct<ProjectUpdateReq>>() {
        Ok(Some(body)) => {
            if body.plan_path.len() <= 0 {
                return Ok(Response::with((
                    status::UnprocessableEntity,
                    "Missing value for field: `plan_path`",
                )));
            }

            debug!(
                "GITHUB-CALL builder_api::server::handlers::project_update: Getting app_installation_token; repo_id={} installation_id={}",
                body.repo_id,
                body.installation_id
            );

            let token = match github.app_installation_token(body.installation_id) {
                Ok(token) => token,
                Err(err) => {
                    warn!("Error authenticating github app installation, {}", err);
                    return Ok(Response::with(status::Forbidden));
                }
            };

            project.set_plan_path(body.plan_path);
            project.set_vcs_installation_id(body.installation_id);
            match github.repo(&token, body.repo_id) {
                Ok(Some(repo)) => project.set_vcs_data(repo.clone_url),
                Ok(None) => return Ok(Response::with((status::NotFound, "rg:pu:2"))),
                Err(e) => {
                    warn!("Error finding GH repo. e = {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "rg:pu:1")));
                }
            }
            (token, body.repo_id)
        }
        _ => return Ok(Response::with(status::UnprocessableEntity)),
    };

    match github.contents(&token, repo_id, &project.get_plan_path()) {
        Ok(Some(contents)) => {
            match contents.decode() {
                Ok(bytes) => {
                    match Plan::from_bytes(bytes.as_slice()) {
                        Ok(plan) => {
                            debug!("plan = {:?}", &plan);
                            if plan.name != name {
                                return Ok(Response::with((status::UnprocessableEntity, "rg:pu:7")));
                            }
                            project.set_origin_name(String::from(origin));
                            project.set_package_name(String::from(name));
                        }
                        Err(e) => {
                            debug!("Error matching Plan. e = {:?}", e);
                            return Ok(Response::with((status::UnprocessableEntity, "rg:pu:3")));
                        }
                    }
                }
                Err(e) => {
                    debug!("Error decoding content from b64. e = {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "rg:pu:4")));
                }
            }
        }
        Ok(None) => return Ok(Response::with((status::NotFound, "rg:pu:6"))),
        Err(e) => {
            warn!("Erroring fetching contents from GH. e = {:?}", e);
            return Ok(Response::with((status::UnprocessableEntity, "rg:pu:5")));
        }
    }

    request.set_requestor_id(session_id);
    request.set_project(project);
    match route_message::<OriginProjectUpdate, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Display the the given project's details
// This route is only available if jobsrv_enabled is true
pub fn project_show(req: &mut Request) -> IronResult<Response> {
    let mut project_get = OriginProjectGet::new();

    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let cfg = req.get::<persistent::Read<Config>>().unwrap();
    if !cfg.depot.non_core_builds_enabled {
        if origin != "core" {
            return Ok(Response::with(status::Forbidden));
        }
    }

    let name = match get_param(req, "name") {
        Some(n) => n,
        None => return Ok(Response::with(status::BadRequest)),
    };

    project_get.set_name(format!("{}/{}", &origin, &name));

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    match route_message::<OriginProjectGet, OriginProject>(req, &project_get) {
        Ok(project) => Ok(render_json(status::Ok, &project)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Return names of all the projects in the given origin
// This route is only available if jobsrv_enabled is true
pub fn project_list(req: &mut Request) -> IronResult<Response> {
    let mut projects_get = OriginProjectListGet::new();

    let origin = match get_param(req, "origin") {
        Some(o) => o,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    projects_get.set_origin(origin);

    match route_message::<OriginProjectListGet, OriginProjectList>(req, &projects_get) {
        Ok(projects) => Ok(render_json(status::Ok, &projects.get_names())),
        Err(err) => Ok(render_net_error(&err)),
    }
}

/// Retrieve the most recent 50 jobs for a project.
// This route is only available if jobsrv_enabled is true
pub fn project_jobs(req: &mut Request) -> IronResult<Response> {
    let mut jobs_get = ProjectJobsGet::new();

    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    match get_param(req, "name") {
        Some(name) => jobs_get.set_name(format!("{}/{}", origin, name)),
        None => return Ok(Response::with(status::BadRequest)),
    }

    if !check_origin_access(req, &origin).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    match helpers::extract_pagination(req) {
        Ok((start, stop)) => {
            jobs_get.set_start(start as u64);
            jobs_get.set_stop(stop as u64);
        }
        Err(response) => return Ok(response),
    }
    match route_message::<ProjectJobsGet, ProjectJobsGetResponse>(req, &jobs_get) {
        Ok(response) => {
            let list: Vec<serde_json::Value> = response
                .get_jobs()
                .iter()
                .map(|job| if job.get_state() == JobState::Complete {
                    let channels =
                        helpers::channels_for_package_ident(req, &job.get_package_ident());
                    let platforms =
                        helpers::platforms_for_package_ident(req, &job.get_package_ident());
                    let mut job_json = serde_json::to_value(job).unwrap();

                    if channels.is_some() {
                        job_json["channels"] = json!(channels);
                    }

                    if platforms.is_some() {
                        job_json["platforms"] = json!(platforms);
                    }

                    job_json
                } else {
                    serde_json::to_value(job).unwrap()
                })
                .collect();

            helpers::paginated_response(
                &list,
                response.get_count() as isize,
                response.get_start() as isize,
                response.get_stop() as isize,
            )
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

// This route is only available if jobsrv_enabled is true
pub fn create_project_integration(req: &mut Request) -> IronResult<Response> {
    let params = match validate_params(req, &["origin", "name", "integration"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    let body = req.get::<bodyparser::Json>();
    match body {
        Ok(Some(_)) => (),
        Ok(None) => {
            warn!("create_project_integration: Empty body in request");
            return Ok(Response::with(status::BadRequest));
        }
        Err(e) => {
            warn!("create_project_integration, Error parsing body: {:?}", e);
            return Ok(Response::with(status::BadRequest));
        }
    };

    if !check_origin_access(req, &params["origin"]).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    // We know body exists and is valid, non-empty JSON, so we can unwrap safely
    let json_body = req.get::<bodyparser::Raw>().unwrap().unwrap();

    let mut opi = OriginProjectIntegration::new();
    opi.set_origin(params["origin"].clone());
    opi.set_name(params["name"].clone());
    opi.set_integration(params["integration"].clone());
    opi.set_integration_name(String::from(DEFAULT_PROJECT_INTEGRATION));
    opi.set_body(json_body);

    let mut request = OriginProjectIntegrationCreate::new();
    request.set_integration(opi);

    match route_message::<OriginProjectIntegrationCreate, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => {
            if err.get_code() == ErrCode::ENTITY_CONFLICT {
                warn!("Failed to create integration as it already exists");
                Ok(Response::with(status::Conflict))
            } else {
                error!("create_project_integration:1, err={:?}", err);
                Ok(Response::with(status::InternalServerError))
            }
        }
    }
}

// This route is only available if jobsrv_enabled is true
pub fn delete_project_integration(req: &mut Request) -> IronResult<Response> {
    let params = match validate_params(req, &["origin", "name", "integration"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    if !check_origin_access(req, &params["origin"]).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut request = OriginProjectIntegrationDelete::new();
    request.set_origin(params["origin"].clone());
    request.set_name(params["name"].clone());
    request.set_integration(params["integration"].clone());

    match route_message::<OriginProjectIntegrationDelete, NetOk>(req, &request) {
        Ok(_) => Ok(Response::with(status::NoContent)),
        Err(err) => {
            error!("delete_project_integration:1, err={:?}", err);
            Ok(Response::with(status::InternalServerError))
        }
    }
}

// This route is only available if jobsrv_enabled is true
pub fn get_project_integration(req: &mut Request) -> IronResult<Response> {
    let params = match validate_params(req, &["origin", "name", "integration"]) {
        Ok(p) => p,
        Err(st) => return Ok(Response::with(st)),
    };

    let mut opi = OriginProjectIntegration::new();
    opi.set_origin(params["origin"].clone());
    opi.set_name(params["name"].clone());
    opi.set_integration(params["integration"].clone());
    opi.set_integration_name(String::from(DEFAULT_PROJECT_INTEGRATION));

    let mut request = OriginProjectIntegrationGet::new();
    request.set_integration(opi);

    match route_message::<OriginProjectIntegrationGet, OriginProjectIntegration>(req, &request) {
        Ok(integration) => {
            let v: serde_json::Value = match serde_json::from_str(&integration.get_body()) {
                Ok(v) => v,
                Err(e) => {
                    debug!("Error parsing to JSON. e = {:?}", e);
                    return Ok(Response::with((status::UnprocessableEntity, "api:gpi:1")));
                }
            };
            Ok(render_json(status::Ok, &v))
        }
        Err(err) => {
            match err.get_code() {
                ErrCode::ENTITY_NOT_FOUND => Ok(Response::with((status::NotFound))),
                _ => {
                    error!(
                        "Unexpected error retrieving project integration, err={:?}",
                        err
                    );
                    Ok(Response::with((status::InternalServerError, "api:gpi:2")))
                }
            }
        }
    }
}
