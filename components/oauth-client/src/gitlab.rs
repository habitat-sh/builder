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
                                ACCEPT_APPLICATION_JSON};

use crate::{config::OAuth2Cfg,
            error::{Error,
                    Result},
            types::*};
use async_trait::async_trait;

pub struct GitLab;

#[derive(Deserialize)]
struct AuthOk {
    pub access_token: String,
}

#[derive(Deserialize)]
struct User {
    pub sub:      String,
    pub nickname: String,
    pub email:    Option<String>,
}

impl GitLab {
    fn parse_user(body: &str) -> Result<OAuth2User> {
        let user: User = serde_json::from_str(body).map_err(Error::Serialization)?;
        Ok(OAuth2User { id:       user.sub,
                        username: user.nickname,
                        email:    user.email, })
    }

    async fn user(&self,
                  config: &OAuth2Cfg,
                  client: &HttpClient,
                  token: &str)
                  -> Result<OAuth2User> {
        let header_values = vec![ACCEPT_APPLICATION_JSON.clone()];
        let headers = header_values.into_iter().collect::<HeaderMap<_>>();

        let resp = client.get(&config.userinfo_url)
                         .headers(headers)
                         .bearer_auth(token)
                         .send()
                         .await
                         .map_err(Error::HttpClient)?;

        let status = resp.status();
        let body = resp.text().await.map_err(Error::HttpClient)?;
        debug!("GitLab response body: {}", body);

        if status.is_success() {
            Self::parse_user(&body)
        } else {
            Err(Error::HttpResponse(status, body))
        }
    }
}

#[async_trait]
impl OAuth2Provider for GitLab {
    async fn authenticate(&self,
                          config: &OAuth2Cfg,
                          client: &HttpClient,
                          code: &str)
                          -> Result<(String, OAuth2User)> {
        let url = format!("{}?client_id={}&client_secret={}&grant_type=authorization_code&\
                           code={}&redirect_uri={}",
                          config.token_url,
                          config.client_id,
                          config.client_secret,
                          code,
                          config.redirect_url);

        let header_values = vec![ACCEPT_APPLICATION_JSON.clone()];
        let headers = header_values.into_iter().collect::<HeaderMap<_>>();

        let resp = client.post(&url)
                         .headers(headers)
                         .send()
                         .await
                         .map_err(Error::HttpClient)?;

        let status = resp.status();
        let body = resp.text().await.map_err(Error::HttpClient)?;
        debug!("GitLab response body: {}", body);

        let token = if status.is_success() {
            match serde_json::from_str::<AuthOk>(&body) {
                Ok(msg) => msg.access_token,
                Err(e) => return Err(Error::Serialization(e)),
            }
        } else {
            return Err(Error::HttpResponse(status, body));
        };

        let user = self.user(config, client, &token).await?;
        Ok((token, user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_changes_preserve_subject() {
        for username in &["original", "renamed"] {
            let body = format!(r#"{{"sub":"42","nickname":"{}","email":"user@example.com"}}"#,
                               username);
            let user = GitLab::parse_user(&body).unwrap();
            assert_eq!(user.id, "42");
            assert_eq!(user.username, *username);
            assert_eq!(user.email.as_deref(), Some("user@example.com"));
        }
    }

    #[test]
    fn missing_subject_is_rejected() {
        assert!(matches!(GitLab::parse_user(r#"{"nickname":"original"}"#),
                         Err(Error::Serialization(_))));
    }
}
