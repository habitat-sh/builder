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

use actix_web::http::{self, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Json, Path};
use protocol::originsrv::*;

use hab_core::package::ident;

use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers;
use server::AppState;

pub struct Notify;

impl Notify {
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app
    }
}

// r.post("/notify", notify, "notify");

/*
pub fn notify(req: &mut Request) -> IronResult<Response> {
    if req.headers.has::<XGitHubEvent>() {
        return github::handle_event(req);
    }
    Ok(Response::with(status::BadRequest))
}

*/
