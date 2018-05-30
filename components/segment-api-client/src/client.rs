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

use hab_http::ApiClient;
use hyper::client::{IntoUrl, Response};
use hyper::header::{qitem, Accept, Authorization, Basic, ContentType, Headers, UserAgent};
use hyper::mime::{Mime, SubLevel, TopLevel};
use serde_json;

use config::SegmentCfg;
use error::{SegmentError, SegmentResult};

const USER_AGENT: &'static str = "Habitat-Builder";

#[derive(Clone, Debug)]
pub struct SegmentClient {
    pub url: String,
    pub write_key: String,
}

impl SegmentClient {
    pub fn new(config: SegmentCfg) -> Self {
        SegmentClient {
            url: config.url,
            write_key: config.write_key,
        }
    }

    pub fn identify(&self, user_id: &str) -> SegmentResult<Response> {
        let json = json!({ "userId": user_id });

        self.http_post(
            "identify",
            &self.write_key,
            serde_json::to_string(&json).unwrap(),
        )
    }

    pub fn track(&self, user_id: &str, event: &str) -> SegmentResult<Response> {
        let json = json!({
            "userId": user_id,
            "event": event
        });

        self.http_post(
            "track",
            &self.write_key,
            serde_json::to_string(&json).unwrap(),
        )
    }

    fn http_post<U>(&self, path: &str, token: U, body: String) -> SegmentResult<Response>
    where
        U: ToString,
    {
        let client = http_client(&self.url)?;
        let url_path = format!("v1/{}", path);
        let req = client
            .post(&url_path)
            .body(&body)
            .headers(configure_headers(token));

        req.send().map_err(SegmentError::HttpClient)
    }
}

fn http_client<T>(url: T) -> SegmentResult<ApiClient>
where
    T: IntoUrl,
{
    ApiClient::new(url, "bldr", "0.0.0", None).map_err(SegmentError::ApiClient)
}

fn configure_headers<U>(token: U) -> Headers
where
    U: ToString,
{
    let mut headers = Headers::new();
    headers.set(Accept(vec![qitem(Mime(
        TopLevel::Application,
        SubLevel::Json,
        vec![],
    ))]));
    headers.set(ContentType(Mime(
        TopLevel::Application,
        SubLevel::Json,
        vec![],
    )));
    headers.set(UserAgent(USER_AGENT.to_string()));
    headers.set(Authorization(Basic {
        username: token.to_string(),
        password: None,
    }));

    headers
}
