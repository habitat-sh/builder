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

use std::env;

use reqwest::{self, header};

use config::Config;
use error::Result;
use types::*;

#[derive(Clone, Debug)]
pub struct Client {
    inner: reqwest::Client,
    provider: Box<OAuthProvider>,
    pub config: Config,
}

impl Client {
    pub fn new(config: Config, provider: Box<OAuthProvider>) -> Self {
        let mut headers = header::Headers::new();
        headers.set(header::UserAgent::new("oauth-client"));
        let mut client = reqwest::Client::builder();
        client.default_headers(headers);

        if let Ok(url) = env::var("HTTP_PROXY") {
            if let Ok(p) = reqwest::Proxy::http(&url) {
                client.proxy(p);
            } else {
                println!(
                    "Attempted to set a client proxy to {}, but that failed",
                    url,
                )
            }
        }

        if let Ok(url) = env::var("HTTPS_PROXY") {
            if let Ok(p) = reqwest::Proxy::https(&url) {
                client.proxy(p);
            } else {
                println!(
                    "Attempted to set a client proxy to {}, but that failed",
                    url,
                )
            }
        }

        Client {
            inner: client.build().unwrap(),
            provider: provider,
            config: config,
        }
    }

    pub fn authenticate(&self, code: &str) -> Result<String> {
        self.provider.authenticate(&self.inner, code)
    }

    pub fn user(&self, token: &str) -> Result<User> {
        self.provider.user(&self.inner, token)
    }

    pub fn provider(&self) -> String {
        self.provider.name.clone()
    }
}
