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

use actix_web::{http::header,
                web::Query,
                HttpRequest};
use regex::Regex;
use serde::Serialize;
use serde_json;

use crate::{hab_core::package::PackageTarget,
            protocol::jobsrv};

use crate::{db::models::{channel::PackageChannelTrigger as PCT,
                         package::PackageVisibility},
            server::{authorize::authorize_session,
                     AppState}};

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
    range_end:   isize,
    total_count: isize,
    data:        &'a [T],
}

pub fn package_results_json<T: Serialize>(packages: &[T],
                                          count: isize,
                                          start: isize,
                                          end: isize)
                                          -> String {
    let results = PaginatedResults { range_start: start,
                                     range_end:   end,
                                     total_count: count,
                                     data:        packages, };

    serde_json::to_string(&results).unwrap()
}

pub fn extract_pagination(pagination: &Query<Pagination>) -> (isize, isize) {
    (pagination.range, pagination.range + PAGINATION_RANGE_MAX - 1)
}

// Returns the page number we are currently on and the per_page size
pub fn extract_pagination_in_pages(pagination: &Query<Pagination>) -> (isize, isize) {
    (pagination.range / PAGINATION_RANGE_MAX + 1, PAGINATION_RANGE_MAX)
}

// TODO: Deprecate getting target from User Agent header
pub fn target_from_headers(req: &HttpRequest) -> PackageTarget {
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

pub fn visibility_for_optional_session(req: &HttpRequest,
                                       optional_session_id: Option<u64>,
                                       origin: &str)
                                       -> Vec<PackageVisibility> {
    let mut v = Vec::new();
    v.push(PackageVisibility::Public);

    if optional_session_id.is_some() && authorize_session(req, Some(&origin)).is_ok() {
        v.push(PackageVisibility::Hidden);
        v.push(PackageVisibility::Private);
    }

    v
}

pub fn all_visibilities() -> Vec<PackageVisibility> {
    vec![PackageVisibility::Public,
         PackageVisibility::Private,
         PackageVisibility::Hidden,]
}

pub fn trigger_from_request(req: &HttpRequest) -> jobsrv::JobGroupTrigger {
    // TODO: the search strings should be configurable.
    if let Some(ref agent) = req.headers().get(header::USER_AGENT) {
        if let Ok(s) = agent.to_str() {
            if s.starts_with("hab/") {
                return jobsrv::JobGroupTrigger::HabClient;
            }
        }
    }

    if let Some(ref referer) = req.headers().get(header::REFERER) {
        if let Ok(s) = referer.to_str() {
            // this needs to be as generic as possible otherwise local dev envs and on-prem depots
            // won't work
            if s.contains("http") {
                return jobsrv::JobGroupTrigger::BuilderUI;
            }
        }
    }

    jobsrv::JobGroupTrigger::Unknown
}

// TED remove function above when it's no longer used anywhere
pub fn trigger_from_request_model(req: &HttpRequest) -> PCT {
    // TODO: the search strings should be configurable.
    if let Some(ref agent) = req.headers().get(header::USER_AGENT) {
        if let Ok(s) = agent.to_str() {
            if s.starts_with("hab/") {
                return PCT::HabClient;
            }
        }
    }

    if let Some(ref referer) = req.headers().get(header::REFERER) {
        if let Ok(s) = referer.to_str() {
            // this needs to be as generic as possible otherwise local dev envs and on-prem depots
            // won't work
            if s.contains("http") {
                return PCT::BuilderUi;
            }
        }
    }

    PCT::Unknown
}

pub fn req_state(req: &HttpRequest) -> &AppState { req.app_data().expect("request state") }
