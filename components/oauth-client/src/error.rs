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

use std::fmt;

#[derive(Debug)]
pub enum Error {
    BuilderCore(builder_core::Error),
    HttpClient(reqwest::Error),
    HttpResponse(reqwest::StatusCode, String),
    Serialization(serde_json::Error),
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::BuilderCore(ref e) => format!("{}", e),
            Error::HttpClient(ref e) => format!("{}", e),
            Error::HttpResponse(ref code, ref response) => {
                format!("Received a non-200 response, status={}, response={}",
                        code, response)
            }
            Error::Serialization(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl From<builder_core::Error> for Error {
    fn from(err: builder_core::Error) -> Error { Error::BuilderCore(err) }
}
