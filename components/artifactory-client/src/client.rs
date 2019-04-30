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
          fs::File,
          path::PathBuf};

use env_proxy;
use reqwest::{header::{Headers,
                       UserAgent},
              Client,
              Response};
use url::Url;

use crate::{config::ArtifactoryCfg,
            error::{ArtifactoryError,
                    ArtifactoryResult}};

use crate::hab_core::package::{PackageArchive,
                               PackageIdent,
                               PackageTarget};

const USER_AGENT: &str = "Habitat-Builder";
header! { (XJFrogArtApi, "X-JFrog-Art-Api") => [String] }

#[derive(Clone)]
pub struct ArtifactoryClient {
    inner:       Client,
    pub api_url: String,
    pub api_key: String,
    pub repo:    String,
}

impl ArtifactoryClient {
    pub fn new(config: ArtifactoryCfg) -> Self {
        let mut headers = Headers::new();
        headers.set(UserAgent::new(USER_AGENT));
        headers.set(XJFrogArtApi(config.api_key.to_owned()));

        let mut client = Client::builder();
        client.default_headers(headers);

        let url = Url::parse(&config.api_url).expect("valid Artifactory url must be configured");
        debug!("ArtifactoryClient checking proxy for url: {:?}", url);

        if let Some(proxy_url) = env_proxy::for_url(&url).to_string() {
            if url.scheme() == "http" {
                debug!("Setting http_proxy to {}", proxy_url);
                match reqwest::Proxy::http(&proxy_url) {
                    Ok(p) => {
                        client.proxy(p);
                    }
                    Err(e) => warn!("Invalid proxy, err: {:?}", e),
                }
            }

            if url.scheme() == "https" {
                debug!("Setting https proxy to {}", proxy_url);
                match reqwest::Proxy::https(&proxy_url) {
                    Ok(p) => {
                        client.proxy(p);
                    }
                    Err(e) => warn!("Invalid proxy, err: {:?}", e),
                }
            }
        } else {
            debug!("No proxy configured for url: {:?}", url);
        }

        ArtifactoryClient { inner:   client.build().unwrap(),
                            api_url: config.api_url,
                            api_key: config.api_key,
                            repo:    config.repo, }
    }

    pub fn upload(&self,
                  source_path: &PathBuf,
                  ident: &PackageIdent,
                  target: PackageTarget)
                  -> ArtifactoryResult<Response> {
        debug!("ArtifactoryClient upload request for file path: {:?}",
               source_path);

        let url = self.url_path_for(ident, target);
        debug!("ArtifactoryClient upload url = {}", url);

        let file = File::open(source_path).map_err(ArtifactoryError::IO)?;

        let resp = match self.inner
                             .put(&url)
                             .body(file)
                             .send()
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

    pub fn download(&self,
                    destination_path: &PathBuf,
                    ident: &PackageIdent,
                    target: PackageTarget)
                    -> ArtifactoryResult<PackageArchive> {
        debug!("ArtifactoryClient download request for {} ({}) to destination path: {:?}",
               ident, target, destination_path);

        let url = self.url_path_for(ident, target);
        debug!("ArtifactoryClient download url = {}", url);

        let mut resp = match self.inner
                                 .get(&url)
                                 .send()
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
            let mut file = File::create(destination_path).map_err(ArtifactoryError::IO)?;
            std::io::copy(&mut resp, &mut file)?;
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
