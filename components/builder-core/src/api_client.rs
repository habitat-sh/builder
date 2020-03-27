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

use std::{collections::HashMap,
          fs::{self,
               File},
          io::Write,
          iter::FromIterator,
          path::{Path,
                 PathBuf}};

use rand::{distributions::Alphanumeric,
           thread_rng,
           Rng};

use reqwest::{header::HeaderMap,
              Body,
              Response,
              StatusCode};

use serde_json;

use crate::{error::{Error,
                    Result},
            hab_core::{package::{self,
                                 Identifiable,
                                 PackageArchive,
                                 PackageTarget},
                       ChannelIdent}};

use crate::http_client::{HttpClient,
                         ACCEPT_APPLICATION_JSON,
                         USER_AGENT_BLDR,
                         XFILENAME};

#[derive(Clone, Deserialize)]
pub struct PackageIdent {
    pub origin:  String,
    pub name:    String,
    pub version: String,
    pub release: String,
}

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
        let headers = HeaderMap::from_iter(header_values.into_iter());

        Ok(ApiClient { inner: HttpClient::new(url, headers)?,
                       url:   url.to_owned(), })
    }

    pub async fn show_package<I>(&self,
                                 package: &I,
                                 channel: &ChannelIdent,
                                 target: &str,
                                 token: Option<&str>)
                                 -> Result<Package>
        where I: Identifiable
    {
        let mut url = channel_package_path(channel, package);

        if !package.fully_qualified() {
            url.push_str("/latest");
        }

        let url_path = format!("{}/v1/{}", self.url, url);
        let mut query = HashMap::new();
        query.insert("target", target);

        let mut request = self.inner.get(&url_path).query(&query);

        if let Some(token) = token {
            request = request.bearer_auth(token);
        }

        let resp = request.send().await.map_err(Error::HttpClient)?;

        if resp.status() != StatusCode::OK {
            return Err(err_from_response(resp).await);
        }

        let body = resp.text().await?;
        debug!("Body: {:?}", body);

        let package: Package =
            serde_json::from_str::<Package>(&body).map_err(Error::Serialization)?;
        Ok(package)
    }

    pub async fn fetch_package<I, P>(&self,
                                     ident: &I,
                                     target: &str,
                                     dst_path: &P,
                                     token: Option<&str>)
                                     -> Result<PackageArchive>
        where P: AsRef<Path> + ?Sized,
              I: Identifiable
    {
        let url = &package_download(ident);

        let mut qparams = HashMap::new();
        qparams.insert("target", target);

        match self.download(url, &qparams, dst_path.as_ref(), token).await {
            Ok(file) => Ok(PackageArchive::new(file)),
            Err(e) => Err(e),
        }
    }

    async fn download(&self,
                      url: &str,
                      qparams: &HashMap<&str, &str>,
                      dst_path: &Path,
                      token: Option<&str>)
                      -> Result<PathBuf> {
        let url_path = format!("{}/v1/{}", self.url, url);

        let mut request = self.inner.get(&url_path).query(&qparams);

        if let Some(token) = token {
            request = request.bearer_auth(token);
        }

        let resp = request.send().await?;

        debug!("Response: {:?}", resp);

        if resp.status() != StatusCode::OK {
            return Err(err_from_response(resp).await);
        }

        fs::create_dir_all(&dst_path).map_err(Error::IO)?;

        let file_name = match resp.headers().get(XFILENAME.clone()) {
            Some(f) => f.to_str().expect("X-Filename header exists"),
            None => return Err(Error::BadResponse),
        };

        let tmp_file_path = dst_path.join(format!("{}.tmp-{}",
                                                  file_name,
                                                  thread_rng().sample_iter(&Alphanumeric)
                                                              .take(8)
                                                              .collect::<String>()));

        let dst_file_path = dst_path.join(file_name);

        debug!("Writing to {}", &tmp_file_path.display());
        let mut f = File::create(&tmp_file_path).map_err(Error::IO)?;
        f.write_all(&resp.bytes().await?).map_err(Error::IO)?;

        debug!("Moving {} to {}",
               &tmp_file_path.display(),
               &dst_file_path.display());
        fs::rename(&tmp_file_path, &dst_file_path).map_err(Error::IO)?;
        Ok(dst_file_path)
    }

    pub async fn x_put_package(&self, pa: &mut PackageArchive, token: &str) -> Result<()> {
        let checksum = pa.checksum()?;
        let ident = pa.ident()?;
        let target = pa.target()?;

        let url_path = format!("{}/v1/{}", self.url, package_path(&ident));

        let mut qparams: HashMap<&str, &str> = HashMap::new();
        qparams.insert("checksum", &checksum);
        qparams.insert("target", &target);
        qparams.insert("builder", "");

        debug!("Reading from {}", &pa.path.display());

        let body: Body = tokio::fs::read(&pa.path).await.map_err(Error::IO)?.into();

        let resp = self.inner
                       .post(&url_path)
                       .query(&qparams)
                       .body(body)
                       .bearer_auth(token)
                       .send()
                       .await
                       .map_err(Error::HttpClient)?;

        match resp.status() {
            StatusCode::CREATED | StatusCode::CONFLICT => (), // Conflict means package already
            // uploaded - return Ok
            _ => return Err(err_from_response(resp).await),
        }

        Ok(())
    }

    pub async fn fetch_origin_secret_key<P>(&self,
                                            origin: &str,
                                            token: &str,
                                            dst_path: P)
                                            -> Result<PathBuf>
        where P: AsRef<Path>
    {
        self.download(&origin_secret_keys_latest(origin),
                      &HashMap::new(),
                      dst_path.as_ref(),
                      Some(token))
            .await
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

fn channel_package_path<I>(channel: &ChannelIdent, package: &I) -> String
    where I: Identifiable
{
    let mut path = format!("depot/channels/{}/{}/pkgs/{}",
                           package.origin(),
                           channel,
                           package.name());
    if let Some(version) = package.version() {
        path.push_str("/");
        path.push_str(version);
        if let Some(release) = package.release() {
            path.push_str("/");
            path.push_str(release);
        }
    }
    path
}

fn package_download<I>(package: &I) -> String
    where I: Identifiable
{
    format!("{}/download", package_path(package))
}

fn package_path<I>(package: &I) -> String
    where I: Identifiable
{
    format!("depot/pkgs/{}", package)
}

fn origin_secret_keys_latest(origin: &str) -> String {
    format!("depot/origins/{}/secret_keys/latest", origin)
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
