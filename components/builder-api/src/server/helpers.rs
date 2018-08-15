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

// use std::collections::HashMap;
// use std::str::FromStr;

use actix_web::HttpRequest;
use serde::Serialize;
use serde_json;

// use hab_core::channel::{STABLE_CHANNEL, UNSTABLE_CHANNEL};
use hab_core::crypto::SigKeyPair;
use hab_net::privilege::FeatureFlags;
//use hab_net::NetResult;

// use protocol::jobsrv::*;
use protocol::originsrv::*;
use protocol::sessionsrv::*;

use server::error::{Error, Result};
use server::framework::middleware::route_message;
use server::AppState;

//
// TO DO - this module has become a big grab bag of stuff - needs to be
// reviewed and broken up
//

pub fn check_origin_access<T>(req: &HttpRequest<AppState>, origin: &T) -> Result<u64>
where
    T: ToString,
{
    let account_id = {
        let extensions = req.extensions();
        match extensions.get::<Session>() {
            Some(session) => {
                let flags = FeatureFlags::from_bits(session.get_flags()).unwrap(); // unwrap Ok
                if flags.contains(FeatureFlags::BUILD_WORKER) {
                    return Ok(session.get_id());
                }
                session.get_id()
            }
            None => return Err(Error::Authorization(origin.to_string())),
        }
    };

    let mut request = CheckOriginAccessRequest::new();
    request.set_account_id(account_id);
    request.set_origin_name(origin.to_string());

    match route_message::<CheckOriginAccessRequest, CheckOriginAccessResponse>(req, &request) {
        Ok(ref response) if response.get_has_access() => Ok(account_id),
        _ => Err(Error::Authorization(origin.to_string())),
    }
}

pub fn generate_origin_keys(
    req: &HttpRequest<AppState>,
    session_id: u64,
    origin: Origin,
) -> Result<()> {
    let mut public_request = OriginPublicSigningKeyCreate::new();
    let mut secret_request = OriginPrivateSigningKeyCreate::new();
    public_request.set_owner_id(session_id);
    secret_request.set_owner_id(session_id);
    public_request.set_name(origin.get_name().to_string());
    public_request.set_origin_id(origin.get_id());
    secret_request.set_name(origin.get_name().to_string());
    secret_request.set_origin_id(origin.get_id());

    let pair = SigKeyPair::generate_pair_for_origin(origin.get_name())
        .expect("failed to generate origin key pair");
    public_request.set_revision(pair.rev.clone());
    public_request.set_body(
        pair.to_public_string()
            .expect("no public key in generated pair")
            .into_bytes(),
    );
    secret_request.set_revision(pair.rev.clone());
    secret_request.set_body(
        pair.to_secret_string()
            .expect("no secret key in generated pair")
            .into_bytes(),
    );

    route_message::<OriginPublicSigningKeyCreate, OriginPublicSigningKey>(req, &public_request)?;
    route_message::<OriginPrivateSigningKeyCreate, OriginPrivateSigningKey>(req, &secret_request)?;

    Ok(())
}

pub fn get_origin<T>(req: &HttpRequest<AppState>, origin: T) -> Result<Origin>
where
    T: ToString,
{
    let mut request = OriginGet::new();
    request.set_name(origin.to_string());
    route_message::<OriginGet, Origin>(req, &request)
}

pub fn get_session_id(req: &HttpRequest<AppState>) -> u64 {
    req.extensions().get::<Session>().unwrap().get_id()
}

pub fn get_optional_session_id(req: &HttpRequest<AppState>) -> Option<u64> {
    match req.extensions().get::<Session>() {
        Some(session) => Some(session.get_id()),
        None => None,
    }
}

pub fn get_session_id_and_name(req: &HttpRequest<AppState>) -> (u64, String) {
    let (session_id, mut session_name) = {
        let extension = req.extensions();
        let session = extension.get::<Session>().unwrap();
        (session.get_id(), session.get_name().to_string())
    };

    // Sessions created via Personal Access Tokens only have ids, so we may need
    // to get the username explicitly.
    if session_name.is_empty() {
        session_name = get_session_user_name(req, session_id)
    }

    (session_id, session_name)
}

pub fn get_session_user_name(req: &HttpRequest<AppState>, account_id: u64) -> String {
    let mut msg = AccountGetId::new();
    msg.set_id(account_id);

    match route_message::<AccountGetId, Account>(req, &msg) {
        Ok(account) => account.get_name().to_string(),
        Err(err) => {
            warn!("Failed to get account, err={:?}", err);
            "".to_string()
        }
    }
}

pub fn visibility_for_optional_session(
    req: &HttpRequest<AppState>,
    optional_session_id: Option<u64>,
    origin: &str,
) -> Vec<OriginPackageVisibility> {
    let mut v = Vec::new();
    v.push(OriginPackageVisibility::Public);

    if optional_session_id.is_some() && check_origin_access(req, &origin).is_ok() {
        v.push(OriginPackageVisibility::Hidden);
        v.push(OriginPackageVisibility::Private);
    }

    v
}

// Get channels for a package
pub fn channels_for_package_ident(
    req: &HttpRequest<AppState>,
    package: &OriginPackageIdent,
) -> Option<Vec<String>> {
    let session_id = get_optional_session_id(req);

    let mut opclr = OriginPackageChannelListRequest::new();
    opclr.set_ident(package.clone());
    opclr.set_visibilities(visibility_for_optional_session(
        req,
        session_id,
        package.get_origin(),
    ));

    match route_message::<OriginPackageChannelListRequest, OriginPackageChannelListResponse>(
        req, &opclr,
    ) {
        Ok(channels) => {
            let list: Vec<String> = channels
                .get_channels()
                .iter()
                .map(|channel| channel.get_name().to_string())
                .collect();

            Some(list)
        }
        Err(_) => None,
    }
}

// Get platforms for a package
pub fn platforms_for_package_ident(
    req: &HttpRequest<AppState>,
    package: &OriginPackageIdent,
) -> Option<Vec<String>> {
    let session_id = get_optional_session_id(req);

    let mut opplr = OriginPackagePlatformListRequest::new();
    opplr.set_ident(package.clone());
    opplr.set_visibilities(visibility_for_optional_session(
        req,
        session_id,
        package.get_origin(),
    ));

    match route_message::<OriginPackagePlatformListRequest, OriginPackagePlatformListResponse>(
        req, &opplr,
    ) {
        Ok(p) => Some(p.get_platforms().to_vec()),
        Err(_) => None,
    }
}

const PAGINATION_RANGE_DEFAULT: isize = 0;
const PAGINATION_RANGE_MAX: isize = 50;

#[derive(Serialize)]
struct PaginatedResults<'a, T: 'a> {
    range_start: isize,
    range_end: isize,
    total_count: isize,
    data: &'a Vec<T>,
}

pub fn package_results_json<T: Serialize>(
    packages: &Vec<T>,
    count: isize,
    start: isize,
    end: isize,
) -> String {
    let results = PaginatedResults {
        range_start: start,
        range_end: end,
        total_count: count,
        data: packages,
    };

    serde_json::to_string(&results).unwrap()
}

/*

// Returns a tuple representing the from and to values representing a paginated set.
// The range (start, stop) values are zero-based.
pub fn extract_pagination(req: &mut Request) -> Result<(isize, isize), Response> {
    let range_from_param = match extract_query_value("range", req) {
        Some(range) => range,
        None => PAGINATION_RANGE_DEFAULT.to_string(),
    };

    let offset = {
        match range_from_param.parse::<usize>() {
            Ok(range) => range as isize,
            Err(_) => return Err(Response::with(status::BadRequest)),
        }
    };

    debug!(
        "extract_pagination range: (start, end): ({}, {})",
        offset,
        (offset + PAGINATION_RANGE_MAX - 1)
    );
    Ok((offset, offset + PAGINATION_RANGE_MAX - 1))
}

fn is_worker(req: &mut Request) -> bool {
    match req.extensions.get::<Authenticated>() {
        Some(session) => {
            let flags = FeatureFlags::from_bits(session.get_flags()).unwrap();
            flags.contains(FeatureFlags::BUILD_WORKER)
        }
        None => false,
    }
}

pub fn is_request_from_hab(req: &mut Request) -> bool {
    match req.headers.get::<UserAgent>() {
        Some(ref agent) => agent.as_str().starts_with("hab/"),
        None => false,
    }
}

pub fn validate_params(
    req: &mut Request,
    expected_params: &[&str],
) -> Result<HashMap<String, String>, Status> {
    let mut res = HashMap::new();
    // Get the expected params
    {
        let params = req.extensions.get::<Router>().unwrap();

        if expected_params.iter().any(|p| params.find(p).is_none()) {
            return Err(status::BadRequest);
        }

        for p in expected_params {
            res.insert(p.to_string(), params.find(p).unwrap().to_string());
        }
    }
    // Check that we have origin access
    {
        if !check_origin_access(req, &res["origin"]).unwrap_or(false) {
            debug!("Failed origin access check, origin: {}", &res["origin"]);
            return Err(status::Forbidden);
        }
    }
    Ok(res)
}

pub fn paginated_response<T>(
    body: &Vec<T>,
    count: isize,
    start: isize,
    end: isize,
) -> IronResult<Response>
where
    T: Serialize,
{
    let body = package_results_json(body, count, start, end);
    let headers = Header(ContentType(Mime(
        TopLevel::Application,
        SubLevel::Json,
        vec![(Attr::Charset, Value::Utf8)],
    )));

    if count > end + 1 {
        Ok(Response::with((status::PartialContent, body, headers)))
    } else {
        Ok(Response::with((status::Ok, body, headers)))
    }
}

pub fn extract_query_value(key: &str, req: &mut Request) -> Option<String> {
    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref map) => {
            for (k, v) in map.iter() {
                if key == *k {
                    if v.len() < 1 {
                        return None;
                    }
                    return Some(v[0].clone());
                }
            }
            None
        }
        Err(_) => None,
    }
}

pub fn all_visibilities() -> Vec<OriginPackageVisibility> {
    vec![
        OriginPackageVisibility::Public,
        OriginPackageVisibility::Private,
        OriginPackageVisibility::Hidden,
    ]
}

pub fn get_param(req: &mut Request, name: &str) -> Option<String> {
    let params = req.extensions.get::<Router>().unwrap();
    match params.find(name) {
        Some(x) => Some(x.to_string()),
        None => None,
    }
}

pub fn check_origin_owner<T>(req: &mut Request, account_id: u64, origin: T) -> IronResult<bool>
where
    T: ToString,
{
    let mut request = CheckOriginOwnerRequest::new();
    request.set_account_id(account_id);
    request.set_origin_name(origin.to_string());
    match route_message::<CheckOriginOwnerRequest, CheckOriginOwnerResponse>(req, &request) {
        Ok(response) => Ok(response.get_is_owner()),
        Err(err) => {
            let body = serde_json::to_string(&err).unwrap();
            let status = net_err_to_http(err.get_code());
            Err(IronError::new(err, (body, status)))
        }
    }
}

pub fn create_channel(req: &mut Request, origin: &str, channel: &str) -> NetResult<OriginChannel> {
    let mut origin = get_origin(req, origin)?;
    let mut request = OriginChannelCreate::new();

    {
        let session = req.extensions.get::<Authenticated>().unwrap();
        request.set_owner_id(session.get_id());
    }

    request.set_origin_name(origin.take_name());
    request.set_origin_id(origin.get_id());
    request.set_name(channel.to_string());
    route_message::<OriginChannelCreate, OriginChannel>(req, &request)
}

pub fn promote_or_demote_job_group(
    req: &mut Request,
    group_id: u64,
    idents: Option<Vec<String>>,
    channel: &str,
    promote: bool,
) -> NetResult<NetOk> {
    let mut group_get = JobGroupGet::new();
    group_get.set_group_id(group_id);
    group_get.set_include_projects(true);
    let group = route_message::<JobGroupGet, JobGroup>(req, &group_get)?;

    // This only makes sense if the group is complete. If the group isn't complete, return now and
    // let the user know. Check the completion state by checking the individual project states,
    // as if this is called by the scheduler it needs to promote/demote the group before marking it
    // Complete.
    if group.get_projects().iter().any(|&ref p| {
        p.get_state() == JobGroupProjectState::NotStarted
            || p.get_state() == JobGroupProjectState::InProgress
    }) {
        return Err(NetError::new(
            ErrCode::GROUP_NOT_COMPLETE,
            "hg:promote-or-demote-job-group:0",
        ));
    }

    let mut origin_map = HashMap::new();

    let mut ident_map = HashMap::new();
    let has_idents = if idents.is_some() {
        for ident in idents.unwrap().iter() {
            ident_map.insert(ident.clone(), 1);
        }
        true
    } else {
        false
    };

    // We can't assume that every project in the group belongs to the same origin. It's entirely
    // possible that there are multiple origins present within the group. Because of this, there's
    // no way to atomically commit the entire promotion/demotion at once. It's possible origin
    // shards can be on different machines, so for now, the best we can do is partition the projects
    // by origin, and commit each origin at once. Ultimately, it'd be nice to have a way to
    // atomically commit the entire promotion/demotion at once, but that would require a cross-shard
    // tool that we don't currently have.
    for project in group.get_projects().into_iter() {
        if project.get_state() == JobGroupProjectState::Success {
            let ident_str = project.get_ident();
            if has_idents && !ident_map.contains_key(ident_str) {
                continue;
            }

            let ident = OriginPackageIdent::from_str(ident_str).unwrap();
            let project_list = origin_map
                .entry(ident.get_origin().to_string())
                .or_insert(Vec::new());
            project_list.push(project);
        }
    }

    let jgt = trigger_from_request(req);
    let trigger = PackageChannelTrigger::from(jgt);

    for (origin, projects) in origin_map.iter() {
        match do_group_promotion_or_demotion(req, channel, projects.to_vec(), &origin, promote) {
            Ok(package_ids) => {
                let mut pgca = PackageGroupChannelAudit::new();

                let mut channel_get = OriginChannelGet::new();
                channel_get.set_origin_name(origin.clone());
                channel_get.set_name(channel.to_string());
                match route_message::<OriginChannelGet, OriginChannel>(req, &channel_get) {
                    Ok(origin_channel) => pgca.set_channel_id(origin_channel.get_id()),
                    Err(err) => return Err(err),
                }

                let mut origin_get = OriginGet::new();
                origin_get.set_name(origin.clone());
                match route_message::<OriginGet, Origin>(req, &origin_get) {
                    Ok(origin_origin) => pgca.set_origin_id(origin_origin.get_id()),
                    Err(err) => return Err(err),
                }

                pgca.set_package_ids(package_ids);

                if promote {
                    pgca.set_operation(PackageChannelOperation::Promote);
                } else {
                    pgca.set_operation(PackageChannelOperation::Demote);
                }

                let (session_id, session_name) = get_session_id_and_name(req);

                pgca.set_trigger(trigger);
                pgca.set_requester_id(session_id);
                pgca.set_requester_name(session_name);
                pgca.set_group_id(group_id);

                route_message::<PackageGroupChannelAudit, NetOk>(req, &pgca)?;
            }
            Err(e) => {
                if e.get_code() != ErrCode::ACCESS_DENIED {
                    warn!("Failed to promote or demote group, err: {:?}", e);
                    return Err(e);
                }
            }
        }
    }

    Ok(NetOk::new())
}

pub fn get_optional_session_id(req: &mut Request) -> Option<u64> {
    match req.extensions.get::<Authenticated>() {
        Some(session) => Some(session.get_id()),
        None => None,
    }
}

pub fn get_optional_oauth_token(req: &mut Request) -> Option<String> {
    match req.extensions.get::<Authenticated>() {
        Some(session) => Some(session.get_oauth_token().to_owned()),
        None => None,
    }
}

pub fn trigger_from_request(req: &mut Request) -> JobGroupTrigger {
    let user_agent = &req.headers.get::<UserAgent>().unwrap().as_str();
    let referer = match req.headers.get::<Referer>() {
        Some(s) => s.as_str(),
        None => "",
    };

    // TODO: the search strings should be configurable.
    if user_agent.starts_with("hab/") {
        JobGroupTrigger::HabClient
    // this needs to be as generic as possible otherwise local dev envs and on-prem depots won't work
    } else if referer.contains("http") {
        JobGroupTrigger::BuilderUI
    } else {
        JobGroupTrigger::Unknown
    }
}

fn do_group_promotion_or_demotion(
    req: &mut Request,
    channel: &str,
    projects: Vec<&JobGroupProject>,
    origin: &str,
    promote: bool,
) -> NetResult<Vec<u64>> {
    if !check_origin_access(req, origin).unwrap_or(false) {
        return Err(NetError::new(
            ErrCode::ACCESS_DENIED,
            "hg:promote-or-demote-job-group:1",
        ));
    }

    let mut ocg = OriginChannelGet::new();
    ocg.set_origin_name(origin.to_string());
    ocg.set_name(channel.to_string());

    let channel = match route_message::<OriginChannelGet, OriginChannel>(req, &ocg) {
        Ok(channel) => channel,
        Err(e) => {
            if e.get_code() == ErrCode::ENTITY_NOT_FOUND {
                if channel != STABLE_CHANNEL || channel != UNSTABLE_CHANNEL {
                    create_channel(req, &origin, channel)?
                } else {
                    info!("Unable to retrieve default channel, err: {:?}", e);
                    return Err(e);
                }
            } else {
                info!("Unable to retrieve channel, err: {:?}", e);
                return Err(e);
            }
        }
    };

    let mut package_ids = Vec::new();

    for project in projects {
        let opi = OriginPackageIdent::from_str(project.get_ident()).unwrap();
        let mut opg = OriginPackageGet::new();
        opg.set_ident(opi);
        opg.set_visibilities(all_visibilities());

        let op = route_message::<OriginPackageGet, OriginPackage>(req, &opg)?;
        package_ids.push(op.get_id());
    }

    if promote {
        let mut opgp = OriginPackageGroupPromote::new();
        opgp.set_channel_id(channel.get_id());
        opgp.set_package_ids(package_ids.clone());
        opgp.set_origin(origin.to_string());

        route_message::<OriginPackageGroupPromote, NetOk>(req, &opgp)?;
    } else {
        let mut opgp = OriginPackageGroupDemote::new();
        opgp.set_channel_id(channel.get_id());
        opgp.set_package_ids(package_ids.clone());
        opgp.set_origin(origin.to_string());

        route_message::<OriginPackageGroupDemote, NetOk>(req, &opgp)?;
    }

    Ok(package_ids)
}

*/
