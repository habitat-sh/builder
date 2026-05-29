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

use crate::{bldr_core::metrics::CounterMetric,
            server::{authorize::authorize_session,
                     error::{Error,
                             Result},
                     feat,
                     services::metrics::Counter}};

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
    Counter::ExtRegistryValidationRequests.increment();
    validate_credentials_body(body)?;
    let actual_url = registry_url(body, registry_type, strict_registry_validation_enabled())?;
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

fn strict_registry_validation_enabled() -> bool { feat::is_enabled(feat::StrictExtRegistryHttps) }

fn validate_credentials_body(body: &Body) -> Result<()> {
    if body.username.is_none() || body.password.is_none() {
        debug!("Error: Missing username or password");
        Err(Error::BadRequest)
    } else {
        Ok(())
    }
}

fn registry_url<'a>(body: &'a Body, registry_type: &str, enforce_https: bool) -> Result<&'a str> {
    match body.url.as_deref() {
        Some(url) => {
            let policy = registry_url_policy(url, true, enforce_https);
            observe_registry_url_policy(policy);
            if matches!(policy, RegistryUrlPolicy::InsecureExplicitBlocked) {
                debug!("Extension registry credential validation rejected non-https url");
                Err(Error::BadRequest)
            } else {
                Ok(url)
            }
        }
        None => {
            match registry_type {
                "docker" => Ok("https://hub.docker.com/v2"),
                _ => Err(Error::BadRequest),
            }
        }
    }
}

fn observe_registry_url_policy(policy: RegistryUrlPolicy) {
    match policy {
        RegistryUrlPolicy::InsecureExplicitAllowed => {
            Counter::ExtRegistryInsecureUrlAllowed.increment();
        }
        RegistryUrlPolicy::InsecureExplicitBlocked => {
            Counter::ExtRegistryInsecureUrlBlocked.increment();
        }
        RegistryUrlPolicy::Defaulted | RegistryUrlPolicy::ExplicitSecure => (),
    }
}

fn registry_url_policy(url: &str, explicit: bool, enforce_https: bool) -> RegistryUrlPolicy {
    if !explicit {
        RegistryUrlPolicy::Defaulted
    } else if is_secure_registry_url(url) {
        RegistryUrlPolicy::ExplicitSecure
    } else if enforce_https {
        RegistryUrlPolicy::InsecureExplicitBlocked
    } else {
        RegistryUrlPolicy::InsecureExplicitAllowed
    }
}

fn is_secure_registry_url(url: &str) -> bool {
    reqwest::Url::parse(url).map(|parsed| parsed.scheme() == "https")
                            .unwrap_or(false)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegistryUrlPolicy {
    Defaulted,
    ExplicitSecure,
    InsecureExplicitAllowed,
    InsecureExplicitBlocked,
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

        assert_eq!(registry_url(&body, "docker", false).unwrap(),
                   "https://hub.docker.com/v2");
    }

    #[test]
    fn registry_url_prefers_explicit_url() {
        let body = Body { url: Some("https://registry.example.test".to_string()),
                          ..valid_body() };

        assert_eq!(registry_url(&body, "docker", false).unwrap(),
                   "https://registry.example.test");
    }

    #[test]
    fn registry_url_rejects_unknown_registry_without_url() {
        let body = valid_body();

        assert!(matches!(registry_url(&body, "harbor", false), Err(Error::BadRequest)));
    }

    #[test]
    fn registry_url_allows_explicit_http_when_flag_is_off() {
        let body = Body { url: Some("http://registry.example.test".to_string()),
                          ..valid_body() };

        assert_eq!(registry_url(&body, "docker", false).unwrap(),
                   "http://registry.example.test");
    }

    #[test]
    fn registry_url_rejects_explicit_http_when_flag_is_on() {
        let body = Body { url: Some("http://registry.example.test".to_string()),
                          ..valid_body() };

        assert!(matches!(registry_url(&body, "docker", true), Err(Error::BadRequest)));
    }

    #[test]
    fn registry_url_rejects_non_https_schemes_when_flag_is_on() {
        let body = Body { url: Some("ftp://registry.example.test".to_string()),
                          ..valid_body() };

        assert!(matches!(registry_url(&body, "docker", true), Err(Error::BadRequest)));
    }

    #[test]
    fn registry_url_policy_tracks_on_off_behavior() {
        assert_eq!(registry_url_policy("https://registry.example.test", true, true),
                   RegistryUrlPolicy::ExplicitSecure);
        assert_eq!(registry_url_policy("http://registry.example.test", true, false),
                   RegistryUrlPolicy::InsecureExplicitAllowed);
        assert_eq!(registry_url_policy("http://registry.example.test", true, true),
                   RegistryUrlPolicy::InsecureExplicitBlocked);
        assert_eq!(registry_url_policy("https://hub.docker.com/v2", false, true),
                   RegistryUrlPolicy::Defaulted);
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
