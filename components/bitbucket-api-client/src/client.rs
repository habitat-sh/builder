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

use std::io::Read;

use hab_http::ApiClient;
use oauth_common::{OAuthError, OAuthResult, OAuthUserToken};
use oauth_common::types::*;
use hyper::header::{Authorization, Basic, Bearer, ContentType};
use serde_json;

use config::BitbucketCfg;
use types::*;

#[derive(Clone)]
pub struct BitbucketClient {
    pub config: BitbucketCfg,
}

impl BitbucketClient {
    pub fn new(config: BitbucketCfg) -> Self {
        BitbucketClient { config: config }
    }
}

impl OAuthClient for BitbucketClient {
    // This function takes the code received from the Oauth dance and exchanges
    // it for an access token
    fn authenticate(&self, code: &str) -> OAuthResult<String> {
        // TODO JB: make the version here dynamic
        let client = ApiClient::new(&self.config.web_url, "habitat", "0.54.0", None)
            .map_err(OAuthError::ApiClient)?;
        let params = format!("grant_type=authorization_code&code={}", code);
        let req = client
            .post("site/oauth2/access_token")
            .header(Authorization(Basic {
                username: self.config.client_id.clone(),
                password: Some(self.config.client_secret.clone()),
            }))
            .header(ContentType::form_url_encoded())
            .body(&params);
        let mut resp = req.send().map_err(OAuthError::HttpClient)?;
        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        if resp.status.is_success() {
            debug!("Bitbucket response body, {}", body);
            match serde_json::from_str::<AuthOk>(&body) {
                Ok(msg) => Ok(msg.access_token),
                Err(e) => {
                    return Err(OAuthError::Serialization(e));
                }
            }
        } else {
            Err(OAuthError::HttpResponse(resp.status, body))
        }
    }

    // This function uses a valid access token to retrieve details about a user. All we really care
    // about is username and email address
    fn user(&self, token: &OAuthUserToken) -> OAuthResult<OAuthUser> {
        // TODO JB: make the version here dynamic
        let client = ApiClient::new(&self.config.api_url, "habitat", "0.54.0", None)
            .map_err(OAuthError::ApiClient)?;
        let mut req = client.get("1.0/user");
        req = req.header(Authorization(Bearer { token: token.to_string() }));
        let mut resp = req.send().map_err(OAuthError::HttpClient)?;
        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        if resp.status.is_success() {
            debug!("Bitbucket response body, {}", body);
            match serde_json::from_str::<UserOk>(&body) {
                Ok(msg) => {
                    Ok(OAuthUser {
                        id: msg.user.username.clone(),
                        username: msg.user.username,
                        email: None,
                    })
                }
                Err(e) => {
                    return Err(OAuthError::Serialization(e));
                }
            }
        } else {
            Err(OAuthError::HttpResponse(resp.status, body))
        }
    }

    fn box_clone(&self) -> Box<OAuthClient> {
        Box::new((*self).clone())
    }

    fn provider(&self) -> String {
        String::from("bitbucket")
    }
}
