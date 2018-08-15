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

use actix_web::http;
use actix_web::FromRequest;
use actix_web::{HttpRequest, HttpResponse, Path};

use protocol::jobsrv::*;

use server::error::Error;
use server::framework::headers;
use server::framework::middleware::route_message;
use server::AppState;

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

/* Old code:

fn package_stats(req: &mut Request) -> IronResult<Response> {
    let mut request = JobGraphPackageStatsGet::new();
    match get_param(req, "origin") {
        Some(origin) => request.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }

    match route_message::<JobGraphPackageStatsGet, JobGraphPackageStats>(req, &request) {
        Ok(stats) => {
            let mut response = render_json(status::Ok, &stats);
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

*/
