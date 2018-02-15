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

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthOk {
    pub access_token: String,
    pub scopes: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub token_type: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ApiErr {
    #[serde(rename = "type")]
    pub _type: String,
    pub error: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserOk {
    pub user: User,
    pub repositories: Vec<Repository>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub display_name: String,
    pub is_staff: bool,
    pub avatar: String,
    pub resource_uri: String,
    pub is_team: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Repository {
    pub scm: String,
    pub has_wiki: bool,
    pub last_updated: String,
    pub no_forks: bool,
    pub created_on: String,
    pub owner: String,
    pub logo: String,
    pub email_mailinglist: String,
    pub is_mq: bool,
    pub size: u64,
    pub read_only: bool,
    pub fork_of: Option<Box<Repository>>,
    pub mq_of: Option<Box<Repository>>,
    pub state: String,
    pub utc_created_on: String,
    pub website: String,
    pub description: String,
    pub has_issues: bool,
    pub is_fork: bool,
    pub slug: String,
    pub is_private: bool,
    pub name: String,
    pub language: String,
    pub utc_last_updated: String,
    pub no_public_forks: bool,
    pub creator: Option<String>,
    pub resource_uri: String,
}
