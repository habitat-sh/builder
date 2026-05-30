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

use reqwest::header::HeaderMap;

use builder_core::http_client::{HttpClient,
                                ACCEPT_APPLICATION_JSON,
                                CONTENT_TYPE_FORM_URL_ENCODED};

use crate::{config::OAuth2Cfg,
            error::{Error,
                    Result},
            form::encode,
            logging::debug_response,
            metrics::{observe_failure,
                      observe_http_failure,
                      observe_request,
                      FailureKind,
                      Operation},
            request::send_with_retry,
            types::*};
use async_trait::async_trait;

pub struct Okta;

#[derive(Deserialize)]
struct AuthOk {
    pub access_token: String,
}

#[derive(Deserialize)]
struct User {
    pub sub:                String,
    pub preferred_username: String,
    pub email:              Option<String>,
}

impl Okta {
    async fn user(&self,
                  config: &OAuth2Cfg,
                  client: &HttpClient,
                  token: &str)
                  -> Result<OAuth2User> {
        observe_request(&config.provider, Operation::UserInfo);
        let header_values = vec![ACCEPT_APPLICATION_JSON.clone()];
        let headers = header_values.into_iter().collect::<HeaderMap<_>>();

        let provider = config.provider.as_str();
        let userinfo_url = config.userinfo_url.as_str();
        let token = token.to_string();
        let resp = send_with_retry(config, "userinfo", || {
                       client.get(userinfo_url)
                             .headers(headers.clone())
                             .bearer_auth(token.as_str())
                   }).await
                     .map_err(|e| {
                         observe_failure(provider, Operation::UserInfo, FailureKind::Transport);
                         Error::HttpClient(e)
                     })?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
                                         observe_failure(&config.provider,
                                                         Operation::UserInfo,
                                                         FailureKind::Transport);
                                         Error::HttpClient(e)
                                     })?;
        debug_response(&config.provider, "userinfo", status, &body);

        if status.is_success() {
            let user = match serde_json::from_str::<User>(&body) {
                Ok(msg) => msg,
                Err(e) => {
                    observe_failure(&config.provider, Operation::UserInfo, FailureKind::Parse);
                    return Err(Error::Serialization(e));
                }
            };

            Ok(OAuth2User { id:       user.sub,
                            username: user.preferred_username,
                            email:    user.email, })
        } else {
            observe_http_failure(&config.provider, Operation::UserInfo, status);
            Err(Error::HttpResponse(status, body))
        }
    }
}

#[async_trait]
impl OAuth2Provider for Okta {
    async fn authenticate(&self,
                          config: &OAuth2Cfg,
                          client: &HttpClient,
                          code: &str)
                          -> Result<(String, OAuth2User)> {
        observe_request(&config.provider, Operation::Token);
        let body = encode(&[("client_id", config.client_id.as_str()),
                            ("client_secret", config.client_secret.as_str()),
                            ("grant_type", "authorization_code"),
                            ("code", code),
                            ("redirect_uri", config.redirect_url.as_str())]);

        let header_values = vec![ACCEPT_APPLICATION_JSON.clone(),
                                 CONTENT_TYPE_FORM_URL_ENCODED.clone(),];
        let headers = header_values.into_iter().collect::<HeaderMap<_>>();

        let provider = config.provider.as_str();
        let token_url = config.token_url.as_str();
        let resp = send_with_retry(config, "token", || {
                       client.post(token_url)
                             .headers(headers.clone())
                             .body(body.clone())
                   }).await
                     .map_err(|e| {
                         observe_failure(provider, Operation::Token, FailureKind::Transport);
                         Error::HttpClient(e)
                     })?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
                                         observe_failure(&config.provider,
                                                         Operation::Token,
                                                         FailureKind::Transport);
                                         Error::HttpClient(e)
                                     })?;
        debug_response(&config.provider, "token", status, &body);

        let token = if status.is_success() {
            match serde_json::from_str::<AuthOk>(&body) {
                Ok(msg) => msg.access_token,
                Err(e) => {
                    observe_failure(&config.provider, Operation::Token, FailureKind::Parse);
                    return Err(Error::Serialization(e));
                }
            }
        } else {
            observe_http_failure(&config.provider, Operation::Token, status);
            return Err(Error::HttpResponse(status, body));
        };

        let user = self.user(config, client, &token).await?;
        Ok((token, user))
    }
}
