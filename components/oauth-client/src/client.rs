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

use crate::{a2::A2,
            active_directory::ActiveDirectory,
            azure_ad::AzureAD,
            bitbucket::Bitbucket,
            config::OAuth2Cfg,
            error::Result,
            github::GitHub,
            gitlab::GitLab,
            metrics::Counter,
            okta::Okta,
            types::*};
use builder_core::{http_client::{HttpClient,
                                 USER_AGENT_BLDR},
                   metrics::CounterMetric};
use reqwest::header::HeaderMap;
use std::iter::FromIterator;

pub struct OAuth2Client {
    inner:        HttpClient,
    pub config:   OAuth2Cfg,
    pub provider: Box<dyn OAuth2Provider>,
}

impl OAuth2Client {
    pub fn new(config: OAuth2Cfg) -> Result<Self> {
        let header_values = vec![USER_AGENT_BLDR.clone(),];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        let client = HttpClient::new(&config.token_url, headers)?;

        let provider: Box<dyn OAuth2Provider> = match &config.provider[..] {
            "active-directory" => Box::new(ActiveDirectory),
            "azure-ad" => Box::new(AzureAD),
            "github" => Box::new(GitHub),
            "gitlab" => Box::new(GitLab),
            "bitbucket" => Box::new(Bitbucket),
            "okta" => Box::new(Okta),
            "chef-automate" => Box::new(A2),
            _ => panic!("Unknown OAuth provider: {}", config.provider),
        };

        Ok(OAuth2Client { inner: client,
                          config,
                          provider })
    }

    pub async fn authenticate(&self, code: &str) -> Result<(String, OAuth2User)> {
        Counter::Authenticate(self.config.provider.clone()).increment();
        debug!("Authenticate called, config: {:?}", self.config);
        self.provider
            .authenticate(&self.config, &self.inner, code)
            .await
    }
}
