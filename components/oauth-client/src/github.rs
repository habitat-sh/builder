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

use serde_json;

use reqwest::header::HeaderMap;

use builder_core::http_client::{HttpClient,
                                ACCEPT_APPLICATION_JSON,
                                ACCEPT_GITHUB_JSON};

use crate::{config::OAuth2Cfg,
            error::{Error,
                    Result},
            types::*};

pub struct GitHub;

#[derive(Deserialize)]
struct AuthOk {
    pub access_token: String,
}

#[derive(Deserialize)]
struct User {
    pub id:    u32,
    pub login: String,
    pub email: Option<String>,
}

impl GitHub {
    fn user(&self, config: &OAuth2Cfg, client: &HttpClient, token: &str) -> Result<OAuth2User> {
        let header_values = vec![ACCEPT_GITHUB_JSON.clone(),];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        let mut resp = client.get(&config.userinfo_url)
                             .headers(headers)
                             .bearer_auth(token)
                             .send()
                             .map_err(Error::HttpClient)?;

        let status = resp.status();
        let body = resp.text().map_err(Error::HttpClient)?;
        debug!("GitHub response body: {}", body);

        if status.is_success() {
            let user = match serde_json::from_str::<User>(&body) {
                Ok(msg) => msg,
                Err(e) => return Err(Error::Serialization(e)),
            };

            Ok(OAuth2User { id:       user.id.to_string(),
                            username: user.login,
                            email:    user.email, })
        } else {
            Err(Error::HttpResponse(status, body))
        }
    }
}

impl OAuth2Provider for GitHub {
    fn authenticate(&self,
                    config: &OAuth2Cfg,
                    client: &HttpClient,
                    code: &str)
                    -> Result<(String, OAuth2User)> {
        let url = format!("{}?client_id={}&client_secret={}&code={}",
                          config.token_url, config.client_id, config.client_secret, code);

        let header_values = vec![ACCEPT_APPLICATION_JSON.clone(),];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        let mut resp = client.post(&url)
                             .headers(headers)
                             .send()
                             .map_err(Error::HttpClient)?;

        let status = resp.status();
        let body = resp.text().map_err(Error::HttpClient)?;
        debug!("GitHub response body: {}", body);

        let token = if status.is_success() {
            match serde_json::from_str::<AuthOk>(&body) {
                Ok(msg) => msg.access_token,
                Err(e) => return Err(Error::Serialization(e)),
            }
        } else {
            return Err(Error::HttpResponse(status, body));
        };

        let user = self.user(config, client, &token)?;
        Ok((token, user))
    }
}
