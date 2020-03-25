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
          error,
          fmt,
          io};

use builder_core;
use reqwest;

pub type ArtifactoryResult<T> = Result<T, ArtifactoryError>;

#[derive(Debug)]
pub enum ArtifactoryError {
    HttpClient(reqwest::Error),
    ApiError(reqwest::StatusCode, HashMap<String, String>),
    BuilderCore(builder_core::Error),
    IO(io::Error),
}

impl fmt::Display for ArtifactoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            ArtifactoryError::HttpClient(ref e) => format!("{}", e),
            ArtifactoryError::ApiError(ref code, ref response) => {
                format!("Received a non-200 response, status={}, response={:?}",
                        code, response)
            }
            ArtifactoryError::BuilderCore(ref e) => format!("{}", e),
            ArtifactoryError::IO(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for ArtifactoryError {
    fn description(&self) -> &str {
        match *self {
            ArtifactoryError::HttpClient(ref err) => err.description(),
            ArtifactoryError::ApiError(..) => "Response returned a non-200 status code.",
            ArtifactoryError::BuilderCore(ref err) => err.description(),
            ArtifactoryError::IO(ref err) => err.description(),
        }
    }
}

impl From<io::Error> for ArtifactoryError {
    fn from(err: io::Error) -> Self { ArtifactoryError::IO(err) }
}

impl From<builder_core::Error> for ArtifactoryError {
    fn from(err: builder_core::Error) -> Self { ArtifactoryError::BuilderCore(err) }
}
impl From<reqwest::Error> for ArtifactoryError {
    fn from(err: reqwest::Error) -> Self { ArtifactoryError::HttpClient(err) }
}
