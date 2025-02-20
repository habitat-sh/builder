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

use std::collections::HashMap;

use reqwest::{header::HeaderMap,
              Response,
              StatusCode};

use crate::{error::{Error,
                    Result},
            hab_core::{package::{self,
                                 Identifiable,
                                 PackageTarget},
                       ChannelIdent},
            http_client::{ACCEPT_APPLICATION_JSON,
                          USER_AGENT_BLDR}};

use crate::http_client::HttpClient;

#[derive(Clone, Deserialize)]
pub struct PackageIdent {
    pub origin:  String,
    pub name:    String,
    pub version: String,
    pub release: String,
}

#[allow(clippy::from_over_into)]
impl Into<package::PackageIdent> for PackageIdent {
    fn into(self) -> package::PackageIdent {
        package::PackageIdent { origin:  self.origin,
                                name:    self.name,
                                version: Some(self.version),
                                release: Some(self.release), }
    }
}

#[derive(Clone, Deserialize)]
pub struct Package {
    pub ident:       PackageIdent,
    pub checksum:    String,
    pub manifest:    String,
    pub target:      String,
    pub deps:        Vec<PackageIdent>,
    pub tdeps:       Vec<PackageIdent>,
    pub build_deps:  Vec<PackageIdent>,
    pub build_tdeps: Vec<PackageIdent>,
    pub exposes:     Vec<u32>,
    pub config:      String,
}

#[derive(Clone)]
pub struct ApiClient {
    inner:   HttpClient,
    pub url: String,
}

impl ApiClient {
    pub fn new(url: &str) -> Result<Self> {
        let header_values = vec![USER_AGENT_BLDR.clone(), ACCEPT_APPLICATION_JSON.clone()];
        let headers = header_values.into_iter().collect::<HeaderMap<_>>();

        Ok(ApiClient { inner: HttpClient::new(url, headers)?,
                       url:   url.to_owned(), })
    }

    pub async fn create_channel(&self,
                                origin: &str,
                                channel: &ChannelIdent,
                                token: &str)
                                -> Result<()> {
        let url_path = format!("{}/v1/depot/channels/{}/{}", self.url, origin, channel);
        debug!("Creating channel, path: {:?}", url_path);

        let resp = self.inner
                       .post(&url_path)
                       .bearer_auth(token)
                       .send()
                       .await
                       .map_err(Error::HttpClient)?;

        match resp.status() {
            StatusCode::CREATED | StatusCode::CONFLICT => (), // Conflict means channel already
            // created - return Ok
            _ => return Err(err_from_response(resp).await),
        }

        Ok(())
    }

    // TODO: make channel type hab_core::ChannelIdent
    pub async fn promote_package<I>(&self,
                                    (ident, target): (&I, PackageTarget),
                                    channel: &ChannelIdent,
                                    token: &str)
                                    -> Result<()>
        where I: Identifiable
    {
        let url_path = format!("{}/v1/{}",
                               self.url,
                               channel_package_promote(channel, ident));
        debug!("Promoting package {}, target {}", ident, target);

        let mut qparams: HashMap<&str, &str> = HashMap::new();
        qparams.insert("target", &target);

        let resp = self.inner
                       .put(&url_path)
                       .query(&qparams)
                       .bearer_auth(token)
                       .send()
                       .await
                       .map_err(Error::HttpClient)?;

        if resp.status() != StatusCode::OK {
            return Err(err_from_response(resp).await);
        };

        Ok(())
    }
}

fn channel_package_promote<I>(channel: &ChannelIdent, package: &I) -> String
    where I: Identifiable
{
    format!("depot/channels/{}/{}/pkgs/{}/{}/{}/promote",
            package.origin(),
            channel,
            package.name(),
            package.version().unwrap(),
            package.release().unwrap())
}

async fn err_from_response(response: Response) -> Error {
    let status = response.status();
    let body = response.text().await.expect("Unable to read response body");
    Error::ApiError(status, body)
}
