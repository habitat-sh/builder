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

use actix_web::http::{Method, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Json, Path};

use http_client::ApiClient;
use hyper;
use hyper::header::{Accept, ContentType};

use serde_json;
use server::error::{Error, Result};
use server::services::github;
use server::AppState;

const PRODUCT: &'static str = "builder-api";
const VERSION: &'static str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

#[derive(Deserialize, Serialize)]
pub struct Body {
    username: Option<String>,
    password: Option<String>,
    url: Option<String>,
}

pub struct Ext;

impl Ext {
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route(
            "/ext/installations/{install_id}/repos/{repo_id}/contents/{path}",
            Method::GET,
            github::repo_file_content,
        ).route(
            "/ext/integrations/{registry_type}/credentials/validate",
            Method::POST,
            validate_registry_credentials,
        )
    }
}

//
// Route handlers - these functions can return any Responder trait
//
pub fn validate_registry_credentials(
    (req, body): (HttpRequest<AppState>, Json<Body>),
) -> HttpResponse {
    let registry_type = Path::<(String)>::extract(&req).unwrap().into_inner();

    match do_validate_registry_credentials(body, registry_type) {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => err.into(),
    }
}

//
// Internal Functions - These functions are business logic for any handlers
//
fn do_validate_registry_credentials(body: Json<Body>, registry_type: String) -> Result<()> {
    if body.username.is_none() || body.password.is_none() {
        debug!("Error: Missing username or password");
        return Err(Error::BadRequest(
            "Error: Missing username or password".to_string(),
        ));
    }

    let url = match body.url {
        Some(ref url) => url.to_string(),
        None => match registry_type.as_ref() {
            "docker" => "https://hub.docker.com/v2".to_string(),
            _ => {
                return Err(Error::BadRequest(
                    "Error: No supported registry type found in request!".to_string(),
                ))
            }
        },
    };

    //TODO: This should absolutely not be our own custom http client type
    // if at all possible we should stop using raw hyper calls and use an
    // actix http client builder and stop doing this
    let actual_url: &str = url.as_ref();

    let client = match ApiClient::new(actual_url, PRODUCT, VERSION, None) {
        Ok(c) => c,
        Err(e) => {
            debug!("Error: Unable to create HTTP client: {}", e);
            return Err(Error::BadRequest(format!(
                "Error: unable to create HTTP client: {}",
                e
            )));
        }
    };

    let sbody = serde_json::to_string(&body.into_inner()).unwrap();
    let result = client
        .post("users/login")
        .header(Accept::json())
        .header(ContentType::json())
        .body(&sbody)
        .send();

    match result {
        Ok(response) => match response.status {
            hyper::status::StatusCode::Ok => Ok(()),
            _ => {
                debug!("Non-OK Response: {}", &response.status);
                Err(Error::BadRequest(format!(
                    "Non-OK Response: {}",
                    &response.status
                )))
            }
        },
        Err(e) => {
            debug!("Error sending request: {:?}", e);
            Err(Error::Authorization)
        }
    }
}
