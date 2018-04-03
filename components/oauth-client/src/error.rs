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

use reqwest;

#[derive(Debug)]
pub enum Error {
    HttpClient(reqwest::Error),
    HttpResponse(reqwest::StatusCode, String),
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::HttpClient(ref e) => format!("{}", e),
            Error::HttpResponse(ref code, ref response) => {
                format!(
                    "Received a non-200 response, status={}, response={}",
                    code,
                    response
                )
            }
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::HttpClient(ref err) => err.description(),
            Error::HttpResponse(_, _) => "Non-200 HTTP response.",
        }
    }
}

// impl From<io::Error> for OAuthError {
//     fn from(err: io::Error) -> Self {
//         OAuthError::IO(err)
//     }
// }
