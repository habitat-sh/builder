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
use reqwest::header::{Accept, Authorization, Bearer, Headers, qitem};
use reqwest::mime;
use serde_json;

use config::OAuth2Cfg;
use error::{Error, Result};
use types::*;

pub struct GitHub;

#[derive(Deserialize)]
pub struct AuthOk {
    pub access_token: String,
    pub scope: String,
    pub token_type: String,
}

#[derive(Default, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u32,
    pub avatar_url: String,
    pub gravatar_id: String,
    pub url: String,
    pub html_url: String,
    pub followers_url: String,
    pub following_url: String,
    pub gists_url: String,
    pub starred_url: String,
    pub subscriptions_url: String,
    pub organizations_url: String,
    pub repos_url: String,
    pub events_url: String,
    pub received_events_url: String,
    pub name: Option<String>,
    pub company: Option<String>,
    pub blog: Option<String>,
    pub location: Option<String>,
    pub email: Option<String>,
    pub hireable: Option<bool>,
    pub bio: Option<String>,
    pub public_repos: Option<u32>,
    pub public_gists: Option<u32>,
    pub followers: Option<u32>,
    pub following: Option<u32>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl GitHub {
    fn user(&self, config: &OAuth2Cfg, client: &Client, token: &str) -> Result<OAuth2User> {
        let mut headers = Headers::new();
        headers.set(Accept(vec![
            qitem(mime::APPLICATION_JSON),
            qitem("application/vnd.github.v3+json".parse().unwrap()),
            qitem(
                "application/vnd.github.machine-man-preview+json"
                    .parse()
                    .unwrap()
            ),
        ]));
        headers.set(Authorization(Bearer { token: token.to_string() }));

        let mut resp = client
            .get(&config.userinfo_url)
            .headers(headers)
            .send()
            .map_err(Error::HttpClient)?;

        if resp.status().is_success() {
            let user: User = resp.json().map_err(Error::HttpClient)?;
            Ok(OAuth2User {
                id: user.id.to_string(),
                username: user.login,
                email: user.email,
            })
        } else {
            let body = resp.text().map_err(Error::HttpClient)?;
            Err(Error::HttpResponse(resp.status(), body))
        }
    }
}



impl OAuth2Provider for GitHub {
    fn authenticate(
        &self,
        config: &OAuth2Cfg,
        client: &Client,
        code: &str,
    ) -> Result<(String, OAuth2User)> {
        let url = format!(
            "{}?client_id={}&client_secret={}&code={}",
            config.token_url,
            config.client_id,
            config.client_secret,
            code
        );

        let mut headers = Headers::new();
        headers.set(Accept(vec![qitem(mime::APPLICATION_JSON)]));

        let mut resp = client.post(&url).headers(headers).send().map_err(
            Error::HttpClient,
        )?;

        let token = if resp.status().is_success() {
            let auth_ok: AuthOk = resp.json().map_err(Error::HttpClient)?;
            auth_ok.access_token
        } else {
            let body = resp.text().map_err(Error::HttpClient)?;
            return Err(Error::HttpResponse(resp.status(), body));
        };

        let user = self.user(config, client, &token)?;
        Ok((token, user))
    }
}
