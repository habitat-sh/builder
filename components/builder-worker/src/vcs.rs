// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

use std::path::Path;

use git2;
use github_api_client::{GitHubCfg,
                        GitHubClient};
use url::Url;

use crate::{bldr_core::{job::Job,
                        metrics::CounterMetric},
            metrics::Counter};

use crate::error::{Error,
                   Result};

pub struct VCS {
    pub vcs_type:        String,
    pub data:            String,
    pub github_client:   GitHubClient,
    pub installation_id: Option<u32>,
}

impl VCS {
    pub fn from_job(job: &Job, config: GitHubCfg) -> Result<Self> {
        match job.get_project().get_vcs_type() {
            "git" => {
                let installation_id: Option<u32> = {
                    if job.get_project().has_vcs_installation_id() {
                        Some(job.get_project().get_vcs_installation_id())
                    } else {
                        None
                    }
                };
                Self::new(String::from(job.get_project().get_vcs_type()),
                          String::from(job.get_project().get_vcs_data()),
                          config,
                          installation_id)
            }
            _ => panic!("unknown vcs associated with jobs project"),
        }
    }

    pub fn new(vcs_type: String,
               data: String,
               config: GitHubCfg,
               installation_id: Option<u32>)
               -> Result<Self> {
        Ok(VCS { vcs_type,
                 data,
                 github_client: GitHubClient::new(config)?,
                 installation_id })
    }

    pub async fn clone(&self, path: &Path) -> Result<()> {
        debug!("VCS clone called, installation id = {:?}, path = {:?}",
               self.installation_id, path);
        match self.vcs_type.as_ref() {
            "git" => {
                let token = match self.installation_id {
                    None => {
                        Counter::GitClone.increment();
                        None
                    }
                    Some(id) => {
                        // TODO (CM): grabbing just the token matter
                        // because the subsequent git2 clone call
                        // doesn't use our Github client... maybe we
                        // should pull it in?
                        debug!("VCS clone creating token");
                        let t = self.github_client
                                    .app_installation_token(id)
                                    .await
                                    .map_err(Error::GithubAppAuthErr)?;
                        Counter::GitAuthenticatedClone.increment();
                        debug!("VCS clone token created successfully");
                        Some(t.inner_token().to_string())
                    }
                };
                debug!("VCS clone starting repo clone");
                git2::Repository::clone(&(self.url(&token)?).as_str(), path).map_err(Error::Git)?;
                debug!("VCS clone repo clone succeeded!");
                Ok(())
            }
            _ => panic!("Unknown vcs type"),
        }
    }

    pub fn url(&self, token: &Option<String>) -> Result<Url> {
        debug!("VCS creating url, token = {:?}", token);
        let mut url = Url::parse(self.data.as_str()).map_err(Error::UrlParseError)?;
        if self.data.starts_with("https://") {
            if let Some(ref tok) = token {
                url.set_username("x-access-token")
                   .map_err(|_| Error::CannotAddCreds)?;
                url.set_password(Some(tok.as_str()))
                   .map_err(|_| Error::CannotAddCreds)?;
            }
        } else {
            return Err(Error::NotHTTPSCloneUrl(url));
        }
        debug!("VCS url = {:?}", url);
        Ok(url)
    }
}
