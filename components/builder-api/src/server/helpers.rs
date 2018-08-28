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

use std::str::FromStr;

use actix_web::http::header;
use actix_web::{HttpRequest, Query};
use regex::Regex;
use serde::Serialize;
use serde_json;

use hab_core::crypto::SigKeyPair;
use hab_core::package::PackageTarget;

use protocol::jobsrv::*;
use protocol::originsrv::*;

use server::authorize::authorize_session;
use server::error::Result;
use server::framework::middleware::route_message;
use server::AppState;

//
// TO DO - this module should not just be a grab bag of stuff
//

pub const PAGINATION_RANGE_MAX: isize = 50;

#[derive(Deserialize)]
pub struct Target {
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Deserialize)]
pub struct Pagination {
    #[serde(default)]
    pub range: isize,
    #[serde(default)]
    pub distinct: bool,
}

#[derive(Serialize)]
pub struct PaginatedResults<'a, T: 'a> {
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

pub fn extract_pagination(pagination: &Query<Pagination>) -> (isize, isize) {
    (
        pagination.range,
        pagination.range + PAGINATION_RANGE_MAX - 1,
    )
}

// TODO: Deprecate getting target from User Agent header
pub fn target_from_headers(req: &HttpRequest<AppState>) -> PackageTarget {
    let user_agent_header = match req.headers().get(header::USER_AGENT) {
        Some(s) => s,
        None => return PackageTarget::from_str("x86_64-linux").unwrap(),
    };

    let user_agent = match user_agent_header.to_str() {
        Ok(ref s) => s.to_string(),
        Err(_) => return PackageTarget::from_str("x86_64-linux").unwrap(),
    };

    debug!("Parsing target from UserAgent header: {}", &user_agent);

    let user_agent_regex =
        Regex::new(r"(?P<client>[^\s]+)\s?(\((?P<target>\w+-\w+); (?P<kernel>.*)\))?").unwrap();

    let target = match user_agent_regex.captures(&user_agent) {
        Some(user_agent_capture) => {
            if let Some(target_match) = user_agent_capture.name("target") {
                target_match.as_str().to_string()
            } else {
                return PackageTarget::from_str("x86_64-linux").unwrap();
            }
        }
        None => return PackageTarget::from_str("x86_64-linux").unwrap(),
    };

    // All of our tooling that depends on this function to return a target will have a user
    // agent that includes the platform, or will specify a target in the query.
    // Therefore, if we can't find a valid target, it's safe to assume that some other kind of HTTP
    // tool is being used, e.g. curl, with looser constraints. For those kinds of cases,
    // let's default it to Linux instead of returning a bad request if we can't properly parse
    // the inbound target.
    match PackageTarget::from_str(&target) {
        Ok(t) => t,
        Err(_) => PackageTarget::from_str("x86_64-linux").unwrap(),
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

pub fn visibility_for_optional_session(
    req: &HttpRequest<AppState>,
    optional_session_id: Option<u64>,
    origin: &str,
) -> Vec<OriginPackageVisibility> {
    let mut v = Vec::new();
    v.push(OriginPackageVisibility::Public);

    if optional_session_id.is_some() && authorize_session(req, Some(&origin)).is_ok() {
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
    let opt_session_id = match authorize_session(req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let mut opclr = OriginPackageChannelListRequest::new();
    opclr.set_ident(package.clone());
    opclr.set_visibilities(visibility_for_optional_session(
        req,
        opt_session_id,
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
    let opt_session_id = match authorize_session(req, None) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let mut opplr = OriginPackagePlatformListRequest::new();
    opplr.set_ident(package.clone());
    opplr.set_visibilities(visibility_for_optional_session(
        req,
        opt_session_id,
        package.get_origin(),
    ));

    match route_message::<OriginPackagePlatformListRequest, OriginPackagePlatformListResponse>(
        req, &opplr,
    ) {
        Ok(p) => Some(p.get_platforms().to_vec()),
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

pub fn create_channel(
    req: &HttpRequest<AppState>,
    origin: &str,
    channel: &str,
) -> Result<OriginChannel> {
    let session_id = authorize_session(req, Some(&origin))?;

    let mut origin = get_origin(req, origin)?;
    let mut request = OriginChannelCreate::new();

    request.set_owner_id(session_id);
    request.set_origin_name(origin.take_name());
    request.set_origin_id(origin.get_id());
    request.set_name(channel.to_string());

    route_message::<OriginChannelCreate, OriginChannel>(req, &request)
}

pub fn trigger_from_request(req: &HttpRequest<AppState>) -> JobGroupTrigger {
    // TODO: the search strings should be configurable.
    match req.headers().get(header::USER_AGENT) {
        Some(ref agent) => match agent.to_str() {
            Ok(s) => if s.starts_with("hab/") {
                return JobGroupTrigger::HabClient;
            },
            Err(_) => (),
        },
        None => (),
    }

    match req.headers().get(header::REFERER) {
        Some(ref referer) => match referer.to_str() {
            // this needs to be as generic as possible otherwise local dev envs and on-prem depots won't work
            Ok(s) => if s.contains("http") {
                return JobGroupTrigger::BuilderUI;
            },
            Err(_) => (),
        },
        None => (),
    }

    JobGroupTrigger::Unknown
}
