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

use error::OAuthResult;

// duplicated from github-api-client, feels bad, but taking a dependency
// on github-api-client to pull this in feels worse.
pub type TokenString = String;

#[derive(Clone, Debug)]
pub struct OAuthUserToken(TokenString);

impl OAuthUserToken {
    pub fn new(token: TokenString) -> Self {
        OAuthUserToken(token)
    }
}

impl fmt::Display for OAuthUserToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(dead_code)]
pub struct OAuthUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
}

pub trait OAuthClient: Send + Sync {
    fn authenticate(&self, &str) -> OAuthResult<String>;
    fn user(&self, &OAuthUserToken) -> OAuthResult<OAuthUser>;
    fn box_clone(&self) -> Box<OAuthClient>;
    fn provider(&self) -> String;
}

// This little nugget of joy is required because Rust doesn't let you
// clone trait objects by default (because the Clone trait returns Self).
// For more reading on this subject, run: rustc --explain 0038
impl Clone for Box<OAuthClient> {
    fn clone(&self) -> Box<OAuthClient> {
        self.box_clone()
    }
}
