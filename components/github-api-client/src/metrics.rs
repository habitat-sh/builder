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

//! Centralized definition of all Github API client metrics that we
//! wish to track.

use std::borrow::Cow;
use builder_core::metrics;

pub type Endpoint = &'static str;

pub enum Counter {
    Authenticate,
    InstallationToken,
    // Github App-mediated API calls
    Api(Endpoint),
    // Github API calls, but using a user's personal Github token
    UserApi(Endpoint),
}

impl metrics::CounterMetric for Counter {}

impl metrics::Metric for Counter {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Counter::Authenticate => "github.authenticate".into(),
            Counter::InstallationToken => "github.installation-token".into(),
            Counter::Api(ref endpoint) => {
                format!("github.api.{}", endpoint).into()
            }
            Counter::UserApi(ref endpoint) => {
                format!("github.user-api.{}", endpoint).into()
            }
        }
    }
}
