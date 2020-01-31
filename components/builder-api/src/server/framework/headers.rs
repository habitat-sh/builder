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

pub const NO_CACHE: &str = "private, no-cache, no-store";
pub const CACHE: &str = "public, max-age=31536000"; // ONE_YEAR_IN_SECONDS

pub const APPLICATION_JSON: &str = "application/json";

pub const XFILENAME: &str = "x-filename"; // must be lowercase

pub enum Cache {
    NoCache,
    MaxAgeDefault,
    MaxAge(usize),
}

impl Default for Cache {
    fn default() -> Self { Cache::MaxAgeDefault }
}

impl ToString for Cache {
    fn to_string(&self) -> String {
        match self {
            Cache::NoCache => NO_CACHE.to_string(),
            Cache::MaxAgeDefault => CACHE.to_string(),
            Cache::MaxAge(secs) => format!("public, max-age={}", secs),
        }
    }
}

pub const XGITHUBEVENT: &str = "X-GitHub-Event";
pub const XHUBSIGNATURE: &str = "X-Hub-Signature";
