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
use std::env;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use reqwest::header::{qitem, Accept, Authorization, Bearer, Headers, UserAgent};
use reqwest::mime;
use reqwest::{Body, Client, Method, Proxy, Response, StatusCode};
use serde_json;

use crate::error::{Error, Result};
use crate::hab_core::{
    package::{self, Identifiable, PackageArchive},
    ChannelIdent,
};

header! { (XFileName, "X-Filename") => [String] }

const USER_AGENT: &str = "Habitat-Builder";

#[derive(Clone, Deserialize)]
pub struct PackageIdent {
    pub origin: String,
    pub name: String,
    pub version: String,
    pub release: String,
}

impl Into<package::PackageIdent> for PackageIdent {
    fn into(self) -> package::PackageIdent {
        package::PackageIdent {
            origin: self.origin,
            name: self.name,
            version: Some(self.version),
            release: Some(self.release),
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct Package {
    pub ident: PackageIdent,
    pub checksum: String,
    pub manifest: String,
    pub target: String,
    pub deps: Vec<PackageIdent>,
    pub tdeps: Vec<PackageIdent>,
    pub exposes: Vec<u32>,
    pub config: String,
}

#[derive(Clone)]
pub struct ApiClient {
    inner: Client,
    pub url: String,
}

impl ApiClient {
    pub fn new(url: &str) -> Self {
        let mut headers = Headers::new();
        headers.set(UserAgent::new(USER_AGENT));
        headers.set(Accept(vec![qitem(mime::APPLICATION_JSON)]));
        let mut client = Client::builder();
        client.default_headers(headers);

        if let Ok(url) = env::var("HTTP_PROXY") {
            debug!("Using HTTP_PROXY: {}", url);
            match Proxy::http(&url) {
                Ok(p) => {
                    client.proxy(p);
                }
                Err(e) => warn!("Invalid proxy url: {}, err: {:?}", url, e),
            }
        }

        if let Ok(url) = env::var("HTTPS_PROXY") {
            debug!("Using HTTPS_PROXY: {}", url);
            match Proxy::https(&url) {
                Ok(p) => {
                    client.proxy(p);
                }
                Err(e) => warn!("Invalid proxy url: {}, err: {:?}", url, e),
            }
        }

        ApiClient {
            inner: client.build().unwrap(),
            url: url.to_owned(),
        }
    }

    pub fn show_package<I>(
        &self,
        package: &I,
        channel: &ChannelIdent,
        target: &str,
        token: Option<&str>,
    ) -> Result<Package>
    where
        I: Identifiable,
    {
        let mut url = channel_package_path(channel, package);

        if !package.fully_qualified() {
            url.push_str("/latest");
        }

        let url_path = format!("{}/v1/{}", self.url, url);
        let mut query = HashMap::new();
        query.insert("target", target);
        let mut resp = self.http_call(Method::Get, &url_path, None, token, &query)?;

        let mut body = String::new();
        resp.read_to_string(&mut body).map_err(Error::IO)?;
        debug!("Body: {:?}", body);

        if resp.status() != StatusCode::Ok {
            return Err(err_from_response(resp));
        }

        let package: Package =
            serde_json::from_str::<Package>(&body).map_err(Error::Serialization)?;
        Ok(package)
    }

    pub fn fetch_package<I, P>(
        &self,
        ident: &I,
        target: &str,
        dst_path: &P,
        token: Option<&str>,
    ) -> Result<PackageArchive>
    where
        P: AsRef<Path> + ?Sized,
        I: Identifiable,
    {
        let url = &package_download(ident);

        let mut qparams = HashMap::new();
        qparams.insert("target", target);

        match self.download(url, &qparams, dst_path.as_ref(), token) {
            Ok(file) => Ok(PackageArchive::new(file)),
            Err(e) => Err(e),
        }
    }

    fn download(
        &self,
        url: &str,
        qparams: &HashMap<&str, &str>,
        dst_path: &Path,
        token: Option<&str>,
    ) -> Result<PathBuf> {
        let url_path = format!("{}/v1/{}", self.url, url);

        let mut resp = self.http_call(Method::Get, &url_path, None, token, &qparams)?;
        debug!("Response: {:?}", resp);

        if resp.status() != StatusCode::Ok {
            return Err(err_from_response(resp));
        }

        fs::create_dir_all(&dst_path).map_err(Error::IO)?;

        let file_name = match resp.headers().get::<XFileName>() {
            Some(f) => f.as_str().to_owned(),
            None => return Err(Error::BadResponse),
        };

        let tmp_file_path = dst_path.join(format!(
            "{}.tmp-{}",
            file_name,
            thread_rng()
                .sample_iter(&Alphanumeric)
                .take(8)
                .collect::<String>()
        ));

        let dst_file_path = dst_path.join(file_name);

        debug!("Writing to {}", &tmp_file_path.display());
        let mut f = File::create(&tmp_file_path).map_err(Error::IO)?;
        io::copy(&mut resp, &mut f).map_err(Error::IO)?;

        debug!(
            "Moving {} to {}",
            &tmp_file_path.display(),
            &dst_file_path.display()
        );
        fs::rename(&tmp_file_path, &dst_file_path).map_err(Error::IO)?;
        Ok(dst_file_path)
    }

    pub fn x_put_package(&self, pa: &mut PackageArchive, token: &str) -> Result<()> {
        let checksum = pa.checksum()?;
        let ident = pa.ident()?;
        let file = File::open(&pa.path).map_err(Error::IO)?;
        let file_size = file.metadata().map_err(Error::IO)?.len();

        let url_path = format!("{}/v1/{}", self.url, package_path(&ident));

        let mut qparams: HashMap<&str, &str> = HashMap::new();
        qparams.insert("checksum", &checksum);
        qparams.insert("builder", "");

        debug!("Reading from {}", &pa.path.display());

        let body = Body::sized(file, file_size);

        let resp = self.http_call(Method::Post, &url_path, Some(body), Some(token), &qparams)?;

        match resp.status() {
            StatusCode::Created | StatusCode::Conflict => (), // Conflict means package already uploaded - return Ok
            _ => return Err(err_from_response(resp)),
        }

        Ok(())
    }

    pub fn fetch_origin_secret_key<P>(
        &self,
        origin: &str,
        token: &str,
        dst_path: P,
    ) -> Result<PathBuf>
    where
        P: AsRef<Path>,
    {
        self.download(
            &origin_secret_keys_latest(origin),
            &HashMap::new(),
            dst_path.as_ref(),
            Some(token),
        )
    }

    pub fn create_channel(&self, origin: &str, channel: &ChannelIdent, token: &str) -> Result<()> {
        let url_path = format!("{}/v1/depot/channels/{}/{}", self.url, origin, channel);
        debug!("Creating channel, path: {:?}", url_path);

        let resp = self.http_call(Method::Post, &url_path, None, Some(token), &HashMap::new())?;

        match resp.status() {
            StatusCode::Created | StatusCode::Conflict => (), // Conflict means channel already created - return Ok
            _ => return Err(err_from_response(resp)),
        }

        Ok(())
    }

    // TODO: make channel type hab_core::ChannelIdent
    pub fn promote_package<I>(&self, ident: &I, channel: &ChannelIdent, token: &str) -> Result<()>
    where
        I: Identifiable,
    {
        let url_path = format!(
            "{}/v1/{}",
            self.url,
            channel_package_promote(channel, ident)
        );
        debug!("Promoting package {}", ident);

        let resp = self.http_call(Method::Put, &url_path, None, Some(token), &HashMap::new())?;

        if resp.status() != StatusCode::Ok {
            return Err(err_from_response(resp));
        };

        Ok(())
    }

    fn http_call<U>(
        &self,
        method: Method,
        url: &str,
        body: Option<Body>,
        token: Option<U>,
        qparams: &HashMap<&str, &str>,
    ) -> Result<Response>
    where
        U: ToString,
    {
        let mut headers = Headers::new();
        if let Some(t) = token {
            headers.set(Authorization(Bearer {
                token: t.to_string(),
            }))
        }

        if body.is_some() {
            self.inner
                .request(method, url)
                .headers(headers)
                .query(qparams)
                .body(body.unwrap())
                .send()
                .map_err(Error::HttpClient)
        } else {
            self.inner
                .request(method, url)
                .headers(headers)
                .query(qparams)
                .send()
                .map_err(Error::HttpClient)
        }
    }
}

fn channel_package_path<I>(channel: &ChannelIdent, package: &I) -> String
where
    I: Identifiable,
{
    let mut path = format!(
        "depot/channels/{}/{}/pkgs/{}",
        package.origin(),
        channel,
        package.name()
    );
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
where
    I: Identifiable,
{
    format!("{}/download", package_path(package))
}

fn package_path<I>(package: &I) -> String
where
    I: Identifiable,
{
    format!("depot/pkgs/{}", package)
}

fn origin_secret_keys_latest(origin: &str) -> String {
    format!("depot/origins/{}/secret_keys/latest", origin)
}

fn channel_package_promote<I>(channel: &ChannelIdent, package: &I) -> String
where
    I: Identifiable,
{
    format!(
        "depot/channels/{}/{}/pkgs/{}/{}/{}/promote",
        package.origin(),
        channel,
        package.name(),
        package.version().unwrap(),
        package.release().unwrap()
    )
}

fn err_from_response(mut response: Response) -> Error {
    let mut s = String::new();
    response.read_to_string(&mut s).map_err(Error::IO).unwrap();
    Error::ApiError(response.status(), s)
}
