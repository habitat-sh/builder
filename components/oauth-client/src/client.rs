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
use config::OAuth2Cfg;
use error::Result;
use types::*;

use builder_core::metrics::CounterMetric;
use metrics::Counter;
use azure_ad::AzureAD;
use github::GitHub;
use gitlab::GitLab;
use bitbucket::Bitbucket;
use okta::Okta;

pub struct OAuth2Client {
    inner: reqwest::Client,
    pub config: OAuth2Cfg,
    pub provider: Box<OAuth2Provider>,
}

impl OAuth2Client {
    pub fn new(config: OAuth2Cfg) -> Self {
        let mut headers = header::Headers::new();
        headers.set(header::UserAgent::new("oauth-client"));
        let mut client = reqwest::Client::builder();
        client.default_headers(headers);

        if let Ok(url) = env::var("HTTP_PROXY") {
            debug!("Using HTTP_PROXY: {}", url);
            match reqwest::Proxy::http(&url) {
                Ok(p) => {
                    client.proxy(p);
                }
                Err(e) => warn!("Invalid proxy url: {}, err: {:?}", url, e),
            }
        }

        if let Ok(url) = env::var("HTTPS_PROXY") {
            debug!("Using HTTPS_PROXY: {}", url);
            match reqwest::Proxy::https(&url) {
                Ok(p) => {
                    client.proxy(p);
                }
                Err(e) => warn!("Invalid proxy url: {}, err: {:?}", url, e),
            }
        }

        let provider: Box<OAuth2Provider> = match &config.provider[..] {
            "azure-ad" => Box::new(AzureAD),
            "github" => Box::new(GitHub),
            "gitlab" => Box::new(GitLab),
            "bitbucket" => Box::new(Bitbucket),
            "okta" => Box::new(Okta),
            _ => panic!("Unknown OAuth provider: {}", config.provider),
        };

        OAuth2Client {
            inner: client.build().unwrap(),
            config: config,
            provider: provider,
        }
    }

    pub fn authenticate(&self, code: &str) -> Result<(String, OAuth2User)> {
        Counter::Authenticate(self.config.provider.clone()).increment();
        debug!("Authenticate called, config: {:?}", self.config);
        self.provider.authenticate(&self.config, &self.inner, code)
    }
}
