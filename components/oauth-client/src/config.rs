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

/// URL to GitHub User endpoint
pub const DEFAULT_GITHUB_USERINFO_URL: &str = "https://api.github.com/user";
/// URL to GitHub Token endpoint
pub const DEFAULT_GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// Default Client ID providing a value in development environments only.
///
/// See https://developer.github.com/apps
pub const DEV_GITHUB_CLIENT_ID: &str = "Iv1.732260b62f84db15";

/// Default Client Secret providing a value in development environments only.
///
/// See https://developer.github.com/apps
pub const DEV_GITHUB_CLIENT_SECRET: &str = "fc7654ed8c65ccfe014cd339a55e3538f935027a";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct OAuth2Cfg {
    pub provider: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub redirect_url: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Default for OAuth2Cfg {
    fn default() -> Self {
        OAuth2Cfg {
            provider: "github".to_string(),
            token_url: DEFAULT_GITHUB_TOKEN_URL.to_string(),
            userinfo_url: DEFAULT_GITHUB_USERINFO_URL.to_string(),
            redirect_url: "http://localhost/".to_string(),
            client_id: DEV_GITHUB_CLIENT_ID.to_string(),
            client_secret: DEV_GITHUB_CLIENT_SECRET.to_string(),
        }
    }
}
