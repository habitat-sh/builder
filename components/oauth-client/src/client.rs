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

use crate::{config::OAuth2Cfg,
            error::Result,
            types::*};

use env_proxy;
use reqwest::{self,
              header};
use url::Url;

use crate::{a2::A2,
            active_directory::ActiveDirectory,
            azure_ad::AzureAD,
            bitbucket::Bitbucket,
            github::GitHub,
            gitlab::GitLab,
            metrics::Counter,
            okta::Okta};

use builder_core::metrics::CounterMetric;

pub struct OAuth2Client {
    inner:        reqwest::Client,
    pub config:   OAuth2Cfg,
    pub provider: Box<OAuth2Provider>,
}

impl OAuth2Client {
    pub fn new(config: OAuth2Cfg) -> Self {
        let mut headers = header::Headers::new();
        headers.set(header::UserAgent::new("oauth-client"));
        let mut client = reqwest::Client::builder();
        client.default_headers(headers);

        let url = Url::parse(&config.token_url).expect("valid oauth url must be configured");
        trace!("Checking proxy for url: {:?}", url);

        if let Some(proxy_url) = env_proxy::for_url(&url).to_string() {
            if url.scheme() == "http" {
                trace!("Setting http_proxy to {}", proxy_url);
                match reqwest::Proxy::http(&proxy_url) {
                    Ok(p) => {
                        client.proxy(p);
                    }
                    Err(e) => warn!("Invalid proxy, err: {:?}", e),
                }
            }

            if url.scheme() == "https" {
                trace!("Setting https proxy to {}", proxy_url);
                match reqwest::Proxy::https(&proxy_url) {
                    Ok(p) => {
                        client.proxy(p);
                    }
                    Err(e) => warn!("Invalid proxy, err: {:?}", e),
                }
            }
        } else {
            trace!("No proxy configured for url: {:?}", url);
        }

        let provider: Box<OAuth2Provider> = match &config.provider[..] {
            "active-directory" => Box::new(ActiveDirectory),
            "azure-ad" => Box::new(AzureAD),
            "github" => Box::new(GitHub),
            "gitlab" => Box::new(GitLab),
            "bitbucket" => Box::new(Bitbucket),
            "okta" => Box::new(Okta),
            "chef-automate" => Box::new(A2),
            _ => panic!("Unknown OAuth provider: {}", config.provider),
        };

        OAuth2Client { inner: client.build().unwrap(),
                       config,
                       provider }
    }

    pub fn authenticate(&self, code: &str) -> Result<(String, OAuth2User)> {
        Counter::Authenticate(self.config.provider.clone()).increment();
        debug!("Authenticate called, config: {:?}", self.config);
        self.provider.authenticate(&self.config, &self.inner, code)
    }
}
