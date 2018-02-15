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

use std::error;
use std::fmt;
use std::io;

use hab_http;
use hyper;
use serde_json;

#[derive(Debug)]
pub enum OAuthError {
    ApiClient(hab_http::Error),
    HttpClient(hyper::Error),
    HttpClientParse(hyper::error::ParseError),
    HttpResponse(hyper::status::StatusCode, String),
    Hub(String), // making this a String on purpose to avoid a circular dependency on the github-api-client crate
    IO(io::Error),
    Serialization(serde_json::Error),
}

pub type OAuthResult<T> = Result<T, OAuthError>;

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            OAuthError::ApiClient(ref e) => format!("{}", e),
            OAuthError::HttpClient(ref e) => format!("{}", e),
            OAuthError::HttpClientParse(ref e) => format!("{}", e),
            OAuthError::HttpResponse(ref code, ref response) => {
                format!(
                    "Received a non-200 response, status={}, response={}",
                    code,
                    response
                )
            }
            OAuthError::Hub(ref e) => format!("{}", e),
            OAuthError::IO(ref e) => format!("{}", e),
            OAuthError::Serialization(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for OAuthError {
    fn description(&self) -> &str {
        match *self {
            OAuthError::ApiClient(ref err) => err.description(),
            OAuthError::HttpClient(ref err) => err.description(),
            OAuthError::HttpClientParse(ref err) => err.description(),
            OAuthError::HttpResponse(_, _) => "Non-200 HTTP response.",
            OAuthError::Hub(_) => "Error communicating with GitHub",
            OAuthError::IO(ref err) => err.description(),
            OAuthError::Serialization(ref err) => err.description(),
        }
    }
}

impl From<io::Error> for OAuthError {
    fn from(err: io::Error) -> Self {
        OAuthError::IO(err)
    }
}
