// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
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

use reqwest::{header::HeaderMap,
              Body};

use builder_core::http_client::{HttpClient,
                                ACCEPT_APPLICATION_JSON,
                                CONTENT_TYPE_APPLICATION_JSON,
                                USER_AGENT_BLDR};

use crate::{config::SegmentCfg,
            error::SegmentResult};

#[derive(Clone)]
pub struct SegmentClient {
    inner:         HttpClient,
    pub url:       String,
    pub write_key: String,
    pub enabled:   bool,
}

impl SegmentClient {
    pub fn new(config: SegmentCfg) -> SegmentResult<Self> {
        let header_values = vec![USER_AGENT_BLDR.clone(),
                                 ACCEPT_APPLICATION_JSON.clone(),
                                 CONTENT_TYPE_APPLICATION_JSON.clone()];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        Ok(SegmentClient { inner:     HttpClient::new(&config.url, headers)?,
                           url:       config.url,
                           write_key: config.write_key,
                           enabled:   config.enabled, })
    }

    pub fn identify(&self, user_id: &str) {
        if self.enabled {
            let json = json!({ "userId": user_id });
            let sbody = serde_json::to_string(&json).unwrap();
            let body: Body = sbody.into();

            if let Err(err) = self.inner
                                  .post(&self.url_path_for("identity"))
                                  .body(body)
                                  .basic_auth("", Some(&self.write_key))
                                  .send()
            {
                debug!("Error identifying a user in segment, {}", err);
            }
        }
    }

    pub fn track(&self, user_id: &str, event: &str) {
        if self.enabled {
            let json = json!({
                "userId": user_id,
                "event": event
            });

            let sbody = serde_json::to_string(&json).unwrap();
            let body: Body = sbody.into();

            if let Err(err) = self.inner
                                  .post(&self.url_path_for("track"))
                                  .body(body)
                                  .basic_auth("", Some(&self.write_key))
                                  .send()
            {
                debug!("Error tracking event in segment, {}", err);
            }
        }
    }

    fn url_path_for(&self, path: &str) -> String { format!("{}/v1/{}", &self.url, path) }
}
