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

/// URL to GitHub API endpoint
pub const DEFAULT_GITHUB_API_URL: &'static str = "https://api.github.com";
/// Default github application id created in the habitat-sh org
pub const DEFAULT_GITHUB_APP_ID: u32 = 5629;
/// Webhook secret token
pub const DEV_GITHUB_WEBHOOK_SECRET: &'static str = "58d4afaf5e5617ab0f8c39e505605e78a054d003";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GitHubCfg {
    /// URL to GitHub API
    pub api_url: String,
    /// Path to GitHub App private key
    pub app_private_key: String,
    /// App Id used for builder integration
    pub app_id: u32,
    /// Secret key for validating payloads sent by a GitHub WebHook
    pub webhook_secret: String,
}

impl Default for GitHubCfg {
    fn default() -> Self {
        GitHubCfg {
            api_url: DEFAULT_GITHUB_API_URL.to_string(),
            app_private_key: "/src/.secrets/builder-github-app.pem".to_string(),
            app_id: DEFAULT_GITHUB_APP_ID,
            webhook_secret: DEV_GITHUB_WEBHOOK_SECRET.to_string(),
        }
    }
}
