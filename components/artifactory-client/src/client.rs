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

use std::{collections::HashMap,
          path::PathBuf};

use reqwest::{header::{HeaderMap,
                       HeaderName,
                       HeaderValue},
              Body,
              Response};

use crate::{config::ArtifactoryCfg,
            error::{ArtifactoryError,
                    ArtifactoryResult}};

use crate::hab_core::package::{PackageArchive,
                               PackageIdent,
                               PackageTarget};

use builder_core::http_client::{HttpClient,
                                USER_AGENT_BLDR};
use tokio::io::AsyncWriteExt;
const X_JFROG_ART_API: &str = "x-jfrog-art-api";

#[derive(Clone)]
pub struct ArtifactoryClient {
    inner:       HttpClient,
    pub api_url: String,
    pub api_key: String,
    pub repo:    String,
}

impl ArtifactoryClient {
    pub fn new(config: ArtifactoryCfg) -> ArtifactoryResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT_BLDR.0.clone(), USER_AGENT_BLDR.1.clone());
        headers.insert(HeaderName::from_static(X_JFROG_ART_API),
                       HeaderValue::from_str(&config.api_key).expect("Invalid API key value"));

        Ok(ArtifactoryClient { inner:   HttpClient::new(&config.api_url, headers)?,
                               api_url: config.api_url,
                               api_key: config.api_key,
                               repo:    config.repo, })
    }

    pub async fn upload(&self,
                        source_path: &PathBuf,
                        ident: &PackageIdent,
                        target: PackageTarget)
                        -> ArtifactoryResult<Response> {
        debug!("ArtifactoryClient upload request for file path: {:?}",
               source_path);

        let url = self.url_path_for(ident, target);
        debug!("ArtifactoryClient upload url = {}", url);

        let body: Body = tokio::fs::read(source_path).await
                                                     .map_err(ArtifactoryError::IO)?
                                                     .into();

        let resp = match self.inner
                             .put(&url)
                             .body(body)
                             .send()
                             .await
                             .map_err(ArtifactoryError::HttpClient)
        {
            Ok(resp) => resp,
            Err(err) => {
                error!("ArtifactoryClient upload failed, err={}", err);
                return Err(err);
            }
        };

        debug!("Artifactory response status: {:?}", resp.status());

        if resp.status().is_success() {
            Ok(resp)
        } else {
            error!("Artifactory upload non-success status: {:?}", resp.status());
            Err(ArtifactoryError::ApiError(resp.status(), HashMap::new()))
        }
    }

    pub async fn download(&self,
                          destination_path: &PathBuf,
                          ident: &PackageIdent,
                          target: PackageTarget)
                          -> ArtifactoryResult<PackageArchive> {
        debug!("ArtifactoryClient download request for {} ({}) to destination path: {:?}",
               ident, target, destination_path);

        let url = self.url_path_for(ident, target);
        debug!("ArtifactoryClient download url = {}", url);

        let resp = match self.inner
                             .get(&url)
                             .send()
                             .await
                             .map_err(ArtifactoryError::HttpClient)
        {
            Ok(resp) => resp,
            Err(err) => {
                error!("ArtifactoryClient download failed, err={}", err);
                return Err(err);
            }
        };

        debug!("Artifactory response status: {:?}", resp.status());

        if resp.status().is_success() {
            let mut file = tokio::fs::File::create(destination_path).await
                                                                    .map_err(ArtifactoryError::IO)?;
            file.write_all(&resp.bytes().await?).await?;
            Ok(PackageArchive::new(destination_path))
        } else {
            error!("Artifactory download non-success status: {:?}",
                   resp.status());
            Err(ArtifactoryError::ApiError(resp.status(), HashMap::new()))
        }
    }

    fn url_path_for(&self, ident: &PackageIdent, target: PackageTarget) -> String {
        let hart_name = ident.archive_name_with_target(target)
                             .expect("ident is fully qualified");

        let url = format!("{}/artifactory/{}/{}/{}/{}",
                          self.api_url,
                          self.repo,
                          ident.iter().collect::<Vec<&str>>().join("/"),
                          target.iter().collect::<Vec<&str>>().join("/"),
                          hart_name);

        url
    }
}
