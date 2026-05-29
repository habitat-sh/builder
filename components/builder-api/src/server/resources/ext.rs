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

use actix_web::{http::StatusCode,
                web::{self,
                      Json,
                      Path,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};

use reqwest::header::HeaderMap;

use builder_core::http_client::{HttpClient,
                                ACCEPT_APPLICATION_JSON,
                                CONTENT_TYPE_APPLICATION_JSON,
                                USER_AGENT_BLDR};

use crate::server::{authorize::authorize_session,
                    error::{Error,
                            Result}};

#[derive(Deserialize, Serialize)]
pub struct Body {
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url:      Option<String>,
}

pub struct Ext;

impl Ext {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/ext/integrations/{registry_type}/credentials/validate",
                  web::post().to(validate_registry_credentials));
    }
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
pub async fn validate_registry_credentials(req: HttpRequest,
                                           path: Path<String>,
                                           body: Json<Body>)
                                           -> HttpResponse {
    if let Err(err) = authorize_session(&req, None, None) {
        return err.into();
    }

    let registry_type = path.into_inner();

    match do_validate_registry_credentials(&body, &registry_type).await {
        Ok(_) => HttpResponse::new(StatusCode::OK),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// Internal Functions - These functions are business logic for any handlers
//
async fn do_validate_registry_credentials(body: &Body, registry_type: &str) -> Result<()> {
    validate_credentials_body(body)?;
    let actual_url = registry_url(body, registry_type)?;
    let request_body = login_request_body(body)?;

    let header_values = vec![USER_AGENT_BLDR.clone(),
                             ACCEPT_APPLICATION_JSON.clone(),
                             CONTENT_TYPE_APPLICATION_JSON.clone(),];
    let headers = header_values.into_iter().collect::<HeaderMap<_>>();

    let client = HttpClient::new(actual_url, headers)?;
    let post_url = format!("{}/users/login", actual_url);

    match client.post(&post_url)
                .body(request_body)
                .send()
                .await
                .map_err(Error::HttpClient)
    {
        Ok(response) => {
            match response.status() {
                reqwest::StatusCode::OK => Ok(()),
                status => {
                    debug!("Extension registry credential validation failed: status={}",
                           status);
                    Err(Error::BadRequest)
                }
            }
        }
        Err(err) => {
            debug!("Extension registry credential validation request failed: {:?}",
                   err);
            Err(Error::Authorization)
        }
    }
}

fn validate_credentials_body(body: &Body) -> Result<()> {
    if body.username.is_none() || body.password.is_none() {
        debug!("Error: Missing username or password");
        Err(Error::BadRequest)
    } else {
        Ok(())
    }
}

fn registry_url<'a>(body: &'a Body, registry_type: &str) -> Result<&'a str> {
    match body.url.as_deref() {
        Some(url) => Ok(url),
        None => {
            match registry_type {
                "docker" => Ok("https://hub.docker.com/v2"),
                _ => Err(Error::BadRequest),
            }
        }
    }
}

fn login_request_body(body: &Body) -> Result<Vec<u8>> {
    serde_json::to_vec(body).map_err(Error::SerdeJson)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_body() -> Body {
        Body { username: Some("hab".to_string()),
               password: Some("secret".to_string()),
               url:      None, }
    }

    #[test]
    fn validate_credentials_body_requires_username_and_password() {
        assert!(validate_credentials_body(&valid_body()).is_ok());

        let missing_username = Body { username: None,
                                      password: Some("secret".to_string()),
                                      url:      None, };
        assert!(matches!(validate_credentials_body(&missing_username),
                         Err(Error::BadRequest)));

        let missing_password = Body { username: Some("hab".to_string()),
                                      password: None,
                                      url:      None, };
        assert!(matches!(validate_credentials_body(&missing_password),
                         Err(Error::BadRequest)));
    }

    #[test]
    fn registry_url_uses_default_for_docker() {
        let body = valid_body();

        assert_eq!(registry_url(&body, "docker").unwrap(),
                   "https://hub.docker.com/v2");
    }

    #[test]
    fn registry_url_prefers_explicit_url() {
        let body = Body { url: Some("https://registry.example.test".to_string()),
                          ..valid_body() };

        assert_eq!(registry_url(&body, "docker").unwrap(),
                   "https://registry.example.test");
    }

    #[test]
    fn registry_url_rejects_unknown_registry_without_url() {
        let body = valid_body();

        assert!(matches!(registry_url(&body, "harbor"), Err(Error::BadRequest)));
    }

    #[test]
    fn login_request_body_serializes_credentials_payload() {
        let body = Body { url: Some("https://registry.example.test".to_string()),
                          ..valid_body() };

        let request_body = login_request_body(&body).unwrap();
        let serialized = String::from_utf8(request_body).unwrap();

        assert_eq!(serialized,
                   r#"{"username":"hab","password":"secret","url":"https://registry.example.test"}"#);
    }

    #[test]
    fn login_request_body_omits_optional_url_when_absent() {
        let request_body = login_request_body(&valid_body()).unwrap();
        let serialized = String::from_utf8(request_body).unwrap();

        assert_eq!(serialized, r#"{"username":"hab","password":"secret"}"#);
    }
}
