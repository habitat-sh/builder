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

use reqwest;

use crate::{config::OAuth2Cfg,
            error::Result};

pub struct OAuth2User {
    pub id:       String,
    pub username: String,
    pub email:    Option<String>,
}

pub trait OAuth2Provider: Sync + Send {
    fn authenticate(&self,
                    config: &OAuth2Cfg,
                    client: &reqwest::Client,
                    code: &str)
                    -> Result<(String, OAuth2User)>;
}
