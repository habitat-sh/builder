// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
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

use actix_web::http;
use actix_web::FromRequest;
use actix_web::{HttpRequest, HttpResponse, Path, Query};
use serde_json;

use protocol::jobsrv::*;
use protocol::originsrv::*;

use server::error::Error;
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers;
use server::AppState;

#[derive(Deserialize)]
pub struct Pagination {
    range: isize,
    distinct: bool,
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

pub fn package_stats(req: &HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(req).unwrap().into_inner(); // Unwrap Ok
    debug!("package_stats called, origin = {}", origin);

    let mut request = JobGraphPackageStatsGet::new();
    request.set_origin(origin);

    match route_message::<JobGraphPackageStatsGet, JobGraphPackageStats>(req, &request) {
        Ok(stats) => HttpResponse::Ok()
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .json(stats),
        Err(err) => Error::NetError(err).into(),
    }
}

// TODO : this needs to be re-designed to not fan out
fn postprocess_package_list(
    req: &HttpRequest<AppState>,
    oplr: &OriginPackageListResponse,
    distinct: bool,
) -> HttpResponse {
    let mut results = Vec::new();

    // The idea here is for every package we get back, pull its channels using the zmq API
    // and accumulate those results. This avoids the N+1 HTTP requests that would be
    // required to fetch channels for a list of packages in the UI. However, if our request
    // has been marked as "distinct" then skip this step because it doesn't make sense in
    // that case. Let's get platforms at the same time.
    for package in oplr.get_idents().to_vec() {
        let mut channels: Option<Vec<String>> = None;
        let mut platforms: Option<Vec<String>> = None;

        if !distinct {
            channels = helpers::channels_for_package_ident(req, &package);
            platforms = helpers::platforms_for_package_ident(req, &package);
        }

        let mut pkg_json = serde_json::to_value(package).unwrap();

        if channels.is_some() {
            pkg_json["channels"] = json!(channels);
        }

        if platforms.is_some() {
            pkg_json["platforms"] = json!(platforms);
        }

        results.push(pkg_json);
    }

    let body = helpers::package_results_json(
        &results,
        oplr.get_count() as isize,
        oplr.get_start() as isize,
        oplr.get_stop() as isize,
    );

    let mut response = if oplr.get_count() as isize > (oplr.get_stop() as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response
        .header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
        .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
        .body(body)
}

pub fn package_list((pagination, req): (Query<Pagination>, HttpRequest<AppState>)) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
    let opt_session_id = helpers::get_optional_session_id(&req);

    let (start, stop) = (
        pagination.range,
        pagination.range + PAGINATION_RANGE_MAX - 1,
    );

    let mut request = OriginPackageListRequest::new();
    request.set_start(start as u64);
    request.set_stop(stop as u64);
    request.set_visibilities(helpers::visibility_for_optional_session(
        &req,
        opt_session_id,
        &origin,
    ));
    request.set_distinct(pagination.distinct);
    request.set_ident(OriginPackageIdent::from_str(origin.as_str()).unwrap());

    match route_message::<OriginPackageListRequest, OriginPackageListResponse>(&req, &request) {
        Ok(olpr) => postprocess_package_list(&req, &olpr, pagination.distinct),
        Err(err) => Error::NetError(err).into(),
    }
}
