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

use std::iter::FromIterator;

use actix_web::{
    http::StatusCode,
    web::{self, Json, Path, ServiceConfig},
    HttpRequest, HttpResponse,
};

use serde_json;

use reqwest::header::HeaderMap;

use builder_core::http_client::{
    HttpClient, ACCEPT_APPLICATION_JSON, CONTENT_TYPE_APPLICATION_JSON, USER_AGENT_BLDR,
};

use crate::server::{
    authorize::authorize_session,
    error::{Error, Result},
    services::github,
};

#[derive(Deserialize, Serialize)]
pub struct Body {
    username: Option<String>,
    password: Option<String>,
    url: Option<String>,
}

pub struct Ext;

impl Ext {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route(
            "/ext/installations/{install_id}/repos/{repo_id}/contents/{path}",
            web::get().to(github::repo_file_content),
        )
        .route(
            "/ext/integrations/{registry_type}/credentials/validate",
            web::post().to(validate_registry_credentials),
        );
    }
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
pub async fn validate_registry_credentials(
    req: HttpRequest,
    path: Path<String>,
    body: Json<Body>,
) -> HttpResponse {
    if let Err(err) = authorize_session(&req, None, None) {
        return err.into();
    }

    let registry_type = path.into_inner();

    match do_validate_registry_credentials(body, &registry_type).await {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// Internal Functions - These functions are business logic for any handlers
//
async fn do_validate_registry_credentials(body: Json<Body>, registry_type: &str) -> Result<()> {
    if body.username.is_none() || body.password.is_none() {
        debug!("Error: Missing username or password");
        return Err(Error::BadRequest);
    }

    let url = match body.url {
        Some(ref url) => url.to_string(),
        None => match registry_type {
            "docker" => "https://hub.docker.com/v2".to_string(),
            _ => return Err(Error::BadRequest),
        },
    };

    let actual_url: &str = url.as_ref();

    let header_values = vec![
        USER_AGENT_BLDR.clone(),
        ACCEPT_APPLICATION_JSON.clone(),
        CONTENT_TYPE_APPLICATION_JSON.clone(),
    ];
    let headers = HeaderMap::from_iter(header_values.into_iter());

    let client = HttpClient::new(actual_url, headers)?;
    let sbody = serde_json::to_string(&body.into_inner()).unwrap();

    let body: reqwest::Body = sbody.into();

    let post_url = format!("{}/users/login", actual_url);

    match client
        .post(&post_url)
        .body(body)
        .send()
        .await
        .map_err(Error::HttpClient)
    {
        Ok(response) => match response.status() {
            StatusCode::OK => Ok(()),
            _ => {
                debug!("Non-OK Response: {}", &response.status());
                Err(Error::BadRequest)
            }
        },
        Err(e) => {
            debug!("Error sending request: {:?}", e);
            Err(Error::Authorization)
        }
    }
}
