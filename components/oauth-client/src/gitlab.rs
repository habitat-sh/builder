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

use reqwest::Client;
use reqwest::header::{qitem, Accept, Authorization, Bearer, Headers};
use reqwest::mime;
use serde_json;

use config::OAuth2Cfg;
use error::{Error, Result};
use types::*;

pub struct GitLab;

#[derive(Deserialize)]
struct AuthOk {
    pub access_token: String,
}

#[derive(Deserialize)]
struct User {
    pub sub: String,
    pub nickname: String,
    pub email: Option<String>,
}

impl GitLab {
    fn user(&self, config: &OAuth2Cfg, client: &Client, token: &str) -> Result<OAuth2User> {
        let mut headers = Headers::new();
        headers.set(Accept(vec![qitem(mime::APPLICATION_JSON)]));
        headers.set(Authorization(Bearer {
            token: token.to_string(),
        }));

        let mut resp = client
            .get(&config.userinfo_url)
            .headers(headers)
            .send()
            .map_err(Error::HttpClient)?;

        let body = resp.text().map_err(Error::HttpClient)?;
        debug!("GitLab response body: {}", body);

        if resp.status().is_success() {
            let user = match serde_json::from_str::<User>(&body) {
                Ok(msg) => msg,
                Err(e) => return Err(Error::Serialization(e)),
            };

            Ok(OAuth2User {
                id: user.sub,
                username: user.nickname,
                email: user.email,
            })
        } else {
            Err(Error::HttpResponse(resp.status(), body))
        }
    }
}

impl OAuth2Provider for GitLab {
    fn authenticate(
        &self,
        config: &OAuth2Cfg,
        client: &Client,
        code: &str,
    ) -> Result<(String, OAuth2User)> {
        let url = format!(
            "{}?client_id={}&client_secret={}&grant_type=authorization_code&code={}&redirect_uri={}",
            config.token_url, config.client_id, config.client_secret, code, config.redirect_url
        );

        let mut headers = Headers::new();
        headers.set(Accept(vec![qitem(mime::APPLICATION_JSON)]));

        let mut resp = client
            .post(&url)
            .headers(headers)
            .send()
            .map_err(Error::HttpClient)?;

        let body = resp.text().map_err(Error::HttpClient)?;
        debug!("GitLab response body: {}", body);

        let token = if resp.status().is_success() {
            match serde_json::from_str::<AuthOk>(&body) {
                Ok(msg) => msg.access_token,
                Err(e) => return Err(Error::Serialization(e)),
            }
        } else {
            return Err(Error::HttpResponse(resp.status(), body));
        };

        let user = self.user(config, client, &token)?;
        Ok((token, user))
    }
}
