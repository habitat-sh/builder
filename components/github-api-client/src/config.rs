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
pub const DEFAULT_GITHUB_API_URL: &str = "https://api.github.com";
/// Default github application id created in the habitat-sh org
pub const DEFAULT_GITHUB_APP_ID: u32 = 5629;
/// Webhook secret token
pub const DEV_GITHUB_WEBHOOK_SECRET: &str = "58d4afaf5e5617ab0f8c39e505605e78a054d003";
/// Per-attempt timeout for retryable GitHub GET requests
pub const DEFAULT_GITHUB_REQUEST_TIMEOUT_MS: u64 = 2_000;
/// Delay between retry attempts for retryable GitHub GET requests
pub const DEFAULT_GITHUB_RETRY_BACKOFF_MS: u64 = 250;
/// Number of retry attempts for retryable GitHub GET requests
pub const DEFAULT_GITHUB_RETRY_ATTEMPTS: usize = 2;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GitHubCfg {
    /// URL to GitHub API
    pub api_url:         String,
    /// Path to GitHub App private key
    pub app_private_key: String,
    /// App Id used for builder integration
    pub app_id:          u32,
    /// Secret key for validating payloads sent by a GitHub WebHook
    pub webhook_secret:  String,
    /// Per-attempt timeout for retryable GitHub GET requests
    pub request_timeout_ms: u64,
    /// Delay between retry attempts for retryable GitHub GET requests
    pub retry_backoff_ms: u64,
    /// Number of retry attempts for retryable GitHub GET requests
    pub retry_attempts: usize,
}

impl Default for GitHubCfg {
    fn default() -> Self {
        GitHubCfg { api_url:         DEFAULT_GITHUB_API_URL.to_string(),
                    app_private_key: "/src/.secrets/builder-github-app.pem".to_string(),
                    app_id:          DEFAULT_GITHUB_APP_ID,
                    webhook_secret:  DEV_GITHUB_WEBHOOK_SECRET.to_string(),
                    request_timeout_ms: DEFAULT_GITHUB_REQUEST_TIMEOUT_MS,
                    retry_backoff_ms: DEFAULT_GITHUB_RETRY_BACKOFF_MS,
                    retry_attempts: DEFAULT_GITHUB_RETRY_ATTEMPTS, }
    }
}
