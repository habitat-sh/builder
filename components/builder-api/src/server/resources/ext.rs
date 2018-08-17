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

pub struct Ext;

impl Ext {
    // Internal - these functions should return Result<..>

    // Route handlers - these functions should return HttpResponse

    // Route registration
    pub fn register(app: App<AppState>) -> App<AppState> {
        app
    }
}

/*
// TODO: EXT HANDLERS

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

/*


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
        None => match registry_type.as_ref() {
            "docker" => "https://hub.docker.com/v2",
            _ => return Ok(Response::with(status::BadRequest)),
        },
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
        Ok(response) => match response.status {
            StatusCode::Ok => Ok(Response::with(status::NoContent)),
            _ => {
                debug!("Non-OK Response: {}", &response.status);
                Ok(Response::with(response.status))
            }
        },
        Err(e) => {
            debug!("Error sending request: {:?}", e);
            Ok(Response::with(status::Forbidden))
        }
    }
}

*/
