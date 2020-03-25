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

use std::{collections::HashMap,
          error,
          fmt,
          io};

use base64;
use builder_core;
use reqwest;
use serde_json;

use crate::{jwt,
            types};

pub type HubResult<T> = Result<T, HubError>;

#[derive(Debug)]
pub enum HubError {
    ApiError(reqwest::StatusCode, HashMap<String, String>),
    AppAuth(types::AppAuthErr),
    BuilderCore(builder_core::Error),
    ContentDecode(base64::DecodeError),
    HttpClient(reqwest::Error),
    IO(io::Error),
    JWT(jwt::Error),
    Serialization(serde_json::Error),
}

impl fmt::Display for HubError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            HubError::ApiError(ref code, ref response) => {
                format!("Received a non-200 response, status={}, response={:?}",
                        code, response)
            }
            HubError::AppAuth(ref e) => format!("GitHub App Authentication error, {}", e),
            HubError::BuilderCore(ref e) => format!("{}", e),
            HubError::ContentDecode(ref e) => format!("{}", e),
            HubError::HttpClient(ref e) => format!("{}", e),
            HubError::IO(ref e) => format!("{}", e),
            HubError::JWT(ref e) => format!("JWT generation error {:?}", e),
            HubError::Serialization(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for HubError {
    fn description(&self) -> &str {
        match *self {
            HubError::ApiError(..) => "Response returned a non-200 status code.",
            HubError::AppAuth(_) => "GitHub App authorization error.",
            HubError::BuilderCore(ref err) => err.description(),
            HubError::ContentDecode(ref err) => err.description(),
            HubError::HttpClient(ref err) => err.description(),
            HubError::IO(ref err) => err.description(),
            HubError::JWT(_) => "Unable to generate JWT token",
            HubError::Serialization(ref err) => err.description(),
        }
    }
}

impl From<io::Error> for HubError {
    fn from(err: io::Error) -> Self { HubError::IO(err) }
}

impl From<serde_json::Error> for HubError {
    fn from(err: serde_json::Error) -> Self { HubError::Serialization(err) }
}

impl From<builder_core::Error> for HubError {
    fn from(err: builder_core::Error) -> Self { HubError::BuilderCore(err) }
}

impl From<reqwest::Error> for HubError {
    fn from(err: reqwest::Error) -> Self { HubError::HttpClient(err) }
}
