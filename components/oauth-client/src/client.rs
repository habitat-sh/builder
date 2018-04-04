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
use std::collections::HashMap;
use reqwest::{self, header};
use config::OAuth2Cfg;
use error::{Result, Error};
use types::*;

use builder_core::metrics::CounterMetric;
use metrics::Counter;
use github::GitHub;

pub struct OAuth2Client {
    inner: reqwest::Client,
    pub config: OAuth2Cfg,
    pub provider: Box<OAuth2Provider>,
}

impl OAuth2Client {
    pub fn new(config: OAuth2Cfg) -> Self {
        println!("**** NEW OAUTH2CLIENT, Config: {:?}", config);

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

        let provider = Box::new(GitHub);

        OAuth2Client {
            inner: client.build().unwrap(),
            config: config,
            provider: provider,
        }
    }

    pub fn authenticate(&self, code: &str, state: &str) -> Result<(String, OAuth2User)> {
        Counter::Authenticate(self.config.provider.clone()).increment();

        println!("**** AUTHENTICATE CALLED, Config: {:?}", self.config);
        self.provider.authenticate(&self.config, &self.inner, code)
    }
}
