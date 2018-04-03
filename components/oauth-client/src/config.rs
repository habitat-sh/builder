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

#[derive(Clone, Debug)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub authorize_url: String,
    pub token_url: String,
    pub user_url: String,
}

impl Config {
    pub fn new<A, B, C, D, E>(
        client_id: A,
        client_secret: B,
        authorize_url: C,
        token_url: D,
        user_url: E,
    ) -> Self
    where
        A: Into<String>,
        B: Into<String>,
        C: Into<String>,
        D: Into<String>,
        E: Into<String>,
    {
        Config {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            authorize_url: authorize_url.into(),
            token_url: token_url.into(),
            user_url: user_url.into(),
        }

    }
}
