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

use std::path::Path;

use std::{collections::HashMap,
          io::Read,
          time::{Duration,
                 SystemTime,
                 UNIX_EPOCH}};

use reqwest::{header::HeaderMap,
              StatusCode};

use builder_core::{http_client::{HttpClient,
                                 ACCEPT_GITHUB_JSON,
                                 CONTENT_TYPE_APPLICATION_JSON,
                                 USER_AGENT_BLDR},
                   metrics::CounterMetric};

use serde_json;

use crate::jwt::{self,
                 Algorithm};

use crate::{config::GitHubCfg,
            error::{HubError,
                    HubResult},
            metrics::Counter,
            types::*};

pub type TokenString = String;
pub type InstallationId = u32;

/// Bundle up a Github token with the Github App installation ID used
/// to create it.
///
/// Consumers will treat this as an opaque type; its main utility is
/// in carrying the installation ID around so we can generate metrics
/// on a per-installation basis.
pub struct AppToken {
    inner_token: TokenString,

    // Leave this here in anticipation of using it for tagged metrics
    #[allow(dead_code)]
    installation_id: InstallationId,
}

impl AppToken {
    // Not public, because you should only get these from
    // `GitHubClient::app_installation_token`
    fn new(inner_token: TokenString, installation_id: InstallationId) -> Self {
        AppToken { inner_token,
                   installation_id }
    }

    // Only providing this for builder-worker's benefit... it
    // currently needs a token for cloning via libgit2; once that's
    // gone, just delete this function.
    /// Retrieve the actual token content for use in HTTP calls.
    pub fn inner_token(&self) -> &str { self.inner_token.as_ref() }
}

#[derive(Clone)]
pub struct GitHubClient {
    inner:              HttpClient,
    pub api_url:        String,
    app_id:             u32,
    app_private_key:    String,
    pub webhook_secret: String,
}

impl GitHubClient {
    pub fn new(config: GitHubCfg) -> HubResult<Self> {
        let header_values = vec![USER_AGENT_BLDR.clone(),
                                 ACCEPT_GITHUB_JSON.clone(),
                                 CONTENT_TYPE_APPLICATION_JSON.clone()];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        Ok(GitHubClient { inner:           HttpClient::new(&config.api_url, headers)?,
                          api_url:         config.api_url,
                          app_id:          config.app_id,
                          app_private_key: config.app_private_key,
                          webhook_secret:  config.webhook_secret, })
    }

    pub fn app(&self) -> HubResult<App> {
        let app_token = generate_app_token(&self.app_private_key, &self.app_id)?;
        let url_path = format!("{}/app", self.api_url);

        let mut rep = self.inner
                          .get(&url_path)
                          .bearer_auth(&app_token)
                          .send()
                          .map_err(HubError::HttpClient)?;

        let mut body = String::new();
        rep.read_to_string(&mut body)?;
        debug!("GitHub response body, {}", body);

        if rep.status() != StatusCode::OK {
            let err: HashMap<String, String> = serde_json::from_str(&body)?;
            return Err(HubError::ApiError(rep.status(), err));
        }

        let contents = serde_json::from_str::<App>(&body)?;
        Ok(contents)
    }

    pub fn app_installation_token(&self, install_id: u32) -> HubResult<AppToken> {
        let app_token = generate_app_token(&self.app_private_key, &self.app_id)?;

        let url_path = format!("{}/installations/{}/access_tokens",
                               self.api_url, install_id);
        debug!("app_installation_token posting to url path {:?}", url_path);
        Counter::InstallationToken.increment();

        let mut rep = self.inner
                          .post(&url_path)
                          .bearer_auth(&app_token)
                          .send()
                          .map_err(HubError::HttpClient)?;

        let mut body = String::new();
        rep.read_to_string(&mut body)?;
        debug!("GitHub response body, {}", body);
        match serde_json::from_str::<AppInstallationToken>(&body) {
            Ok(msg) => Ok(AppToken::new(msg.token, install_id)),
            Err(_) => {
                let err = serde_json::from_str::<AppAuthErr>(&body)?;
                Err(HubError::AppAuth(err))
            }
        }
    }

    /// Returns the contents of a file or directory in a repository.
    pub fn contents(&self, token: &AppToken, repo: u32, path: &str) -> HubResult<Option<Contents>> {
        let url_path = format!("{}/repositories/{}/contents/{}", self.api_url, repo, path);

        Counter::Contents.increment();

        let mut rep = self.inner
                          .get(&url_path)
                          .bearer_auth(&token.inner_token)
                          .send()
                          .map_err(HubError::HttpClient)?;

        let mut body = String::new();
        rep.read_to_string(&mut body)?;
        debug!("GitHub response body, {}", body);
        match rep.status() {
            StatusCode::NOT_FOUND => return Ok(None),
            StatusCode::OK => (),
            status => {
                let err: HashMap<String, String> = serde_json::from_str(&body)?;
                return Err(HubError::ApiError(status, err));
            }
        }
        let mut contents: Contents = serde_json::from_str(&body)?;
        // We need to strip line feeds as the Github API has started to return
        // base64 content with line feeds.
        if contents.encoding == "base64" {
            contents.content = contents.content.replace("\n", "");
        }
        Ok(Some(contents))
    }

    /// Returns the directory listing of a path in a repository.
    pub fn directory(&self,
                     token: &AppToken,
                     repo: u32,
                     path: &str)
                     -> HubResult<Option<Vec<DirectoryEntry>>> {
        let url_path = format!("{}/repositories/{}/contents/{}", self.api_url, repo, path);

        Counter::Contents.increment();
        let mut rep = self.inner
                          .get(&url_path)
                          .bearer_auth(&token.inner_token)
                          .send()
                          .map_err(HubError::HttpClient)?;

        let mut body = String::new();
        rep.read_to_string(&mut body)?;
        debug!("GitHub response body, {}", body);
        match rep.status() {
            StatusCode::NOT_FOUND => return Ok(None),
            StatusCode::OK => (),
            status => {
                let err: HashMap<String, String> = serde_json::from_str(&body)?;
                return Err(HubError::ApiError(status, err));
            }
        }
        let directory: Vec<DirectoryEntry> = serde_json::from_str(&body)?;
        Ok(Some(directory))
    }

    pub fn repo(&self, token: &AppToken, repo: u32) -> HubResult<Option<Repository>> {
        let url_path = format!("{}/repositories/{}", self.api_url, repo);
        Counter::Repo.increment();

        let mut rep = self.inner
                          .get(&url_path)
                          .bearer_auth(&token.inner_token)
                          .send()
                          .map_err(HubError::HttpClient)?;

        let mut body = String::new();
        rep.read_to_string(&mut body)?;
        debug!("GitHub response body, {}", body);
        match rep.status() {
            StatusCode::NOT_FOUND => return Ok(None),
            StatusCode::OK => (),
            status => {
                let err: HashMap<String, String> = serde_json::from_str(&body)?;
                return Err(HubError::ApiError(status, err));
            }
        }
        let value = serde_json::from_str(&body)?;
        Ok(Some(value))
    }

    // The main purpose of this is just to verify HTTP communication with GH.
    // There's nothing special about this endpoint, only that it doesn't require
    // auth and the response body seemed small. We don't even care what the
    // response is. For our purposes, just receiving a response is enough.
    pub fn meta(&self) -> HubResult<()> {
        let url_path = format!("{}/meta", self.api_url);

        let mut rep = self.inner
                          .get(&url_path)
                          .send()
                          .map_err(HubError::HttpClient)?;

        let mut body = String::new();
        rep.read_to_string(&mut body)?;
        debug!("GitHub response body, {}", body);

        if rep.status() != StatusCode::OK {
            let err: HashMap<String, String> = serde_json::from_str(&body)?;
            return Err(HubError::ApiError(rep.status(), err));
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct RepositoryList {
    pub total_count:  u32,
    pub repositories: Vec<Repository>,
}

fn generate_app_token<T, U>(key_path: T, app_id: &U) -> HubResult<String>
    where T: AsRef<Path>,
          U: ToString
{
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let expiration = now + Duration::from_secs(10 * 10);
    let payload = json!({
        "iat" : now.as_secs(),
        "exp" : expiration.as_secs(),
        "iss" : app_id.to_string()});
    debug!("Payload = {:?}", payload);

    let header = json!({});
    let res = jwt::encode(header,
                          &key_path.as_ref().to_path_buf(),
                          &payload,
                          Algorithm::RS256).map_err(HubError::JWT);

    if let Ok(ref t) = res {
        debug!("Encoded JWT token = {}", t);
    };

    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use std::env;

    #[test]
    fn use_a_proxy_from_the_env() {
        let proxy = env::var_os("HTTPS_PROXY");

        if proxy.is_some() {
            let p = proxy.unwrap();
            let pp = p.to_string_lossy();

            if !pp.is_empty() {
                let cfg = config::GitHubCfg::default();
                let client = GitHubClient::new(cfg).unwrap();
                assert_eq!(client.meta().unwrap(), ());
            }
        }
    }
}
