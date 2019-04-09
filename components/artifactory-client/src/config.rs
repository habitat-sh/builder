// Copyright (c) 2019 Chef Software Inc. and/or applicable contributors
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
pub const DEFAULT_ARTIFACTORY_API_URL: &str = "http://localhost:8081";

/// Default repository name
pub const DEFAULT_ARTIFACTORY_REPO: &str = "habitat-artifact-store";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ArtifactoryCfg {
    /// URL to Artifactory API
    pub api_url: String,
    /// Artifactory API key
    pub api_key: String,
    // Repo name
    pub repo: String,
}

impl Default for ArtifactoryCfg {
    fn default() -> Self {
        ArtifactoryCfg { api_url: DEFAULT_ARTIFACTORY_API_URL.to_string(),
                         api_key: "".to_string(),
                         repo:    DEFAULT_ARTIFACTORY_REPO.to_string(), }
    }
}
