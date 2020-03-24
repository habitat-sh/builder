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

use reqwest::{blocking::Body,
              header::HeaderMap};

use builder_core::http_client::{HttpClient,
                                ACCEPT_APPLICATION_JSON,
                                CONTENT_TYPE_FORM_URL_ENCODED};

use crate::{config::OAuth2Cfg,
            error::{Error,
                    Result},
            types::*};

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
    fn user(&self, config: &OAuth2Cfg, client: &HttpClient, token: &str) -> Result<OAuth2User> {
        let header_values = vec![ACCEPT_APPLICATION_JSON.clone(),];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        let resp = client.get(&config.userinfo_url)
                         .headers(headers)
                         .bearer_auth(token)
                         .send()
                         .map_err(Error::HttpClient)?;

        let status = resp.status();
        let body = resp.text().map_err(Error::HttpClient)?;
        debug!("Okta response body: {}", body);

        if status.is_success() {
            let user = match serde_json::from_str::<User>(&body) {
                Ok(msg) => msg,
                Err(e) => return Err(Error::Serialization(e)),
            };

            Ok(OAuth2User { id:       user.sub,
                            username: user.preferred_username,
                            email:    user.email, })
        } else {
            Err(Error::HttpResponse(status, body))
        }
    }
}

impl OAuth2Provider for Okta {
    fn authenticate(&self,
                    config: &OAuth2Cfg,
                    client: &HttpClient,
                    code: &str)
                    -> Result<(String, OAuth2User)> {
        let url = config.token_url.to_string();
        let body = format!("client_id={}&client_secret={}&grant_type=authorization_code&code={}&\
                            redirect_uri={}",
                           config.client_id, config.client_secret, code, config.redirect_url);

        let header_values = vec![ACCEPT_APPLICATION_JSON.clone(),
                                 CONTENT_TYPE_FORM_URL_ENCODED.clone()];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        let body: Body = body.into();

        let resp = client.post(&url)
                         .headers(headers)
                         .body(body)
                         .send()
                         .map_err(Error::HttpClient)?;

        let status = resp.status();
        let body = resp.text().map_err(Error::HttpClient)?;
        debug!("Okta response body: {}", body);

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
