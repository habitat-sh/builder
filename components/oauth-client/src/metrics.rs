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

//! Centralized definition of all OAuth client metrics that we
//! wish to track.

use builder_core::metrics::{self,
                            CounterMetric};
use reqwest::StatusCode;
use std::borrow::Cow;

pub enum Counter {
    Authenticate(String),
    Token(String),
    UserInfo(String),
    Failure {
        provider:  String,
        operation: Operation,
        kind:      FailureKind,
    },
}

#[derive(Clone, Copy)]
pub enum Operation {
    Token,
    UserInfo,
}

#[derive(Clone, Copy)]
pub enum FailureKind {
    Http4xx,
    Http5xx,
    HttpOther,
    Parse,
    Transport,
}

impl Operation {
    fn as_str(self) -> &'static str {
        match self {
            Operation::Token => "token",
            Operation::UserInfo => "userinfo",
        }
    }
}

impl FailureKind {
    fn as_str(self) -> &'static str {
        match self {
            FailureKind::Http4xx => "http-4xx",
            FailureKind::Http5xx => "http-5xx",
            FailureKind::HttpOther => "http-other",
            FailureKind::Parse => "parse",
            FailureKind::Transport => "transport",
        }
    }
}

impl metrics::CounterMetric for Counter {}

impl metrics::Metric for Counter {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Counter::Authenticate(ref provider) => format!("{}.authenticate", provider).into(),
            Counter::Token(ref provider) => format!("{}.token", provider).into(),
            Counter::UserInfo(ref provider) => format!("{}.userinfo", provider).into(),
            Counter::Failure { ref provider,
                               operation,
                               kind, } => {
                format!("{}.{}.failure.{}",
                        provider,
                        operation.as_str(),
                        kind.as_str()).into()
            }
        }
    }
}

pub fn observe_request(provider: &str, operation: Operation) {
    match operation {
        Operation::Token => Counter::Token(provider.to_string()).increment(),
        Operation::UserInfo => Counter::UserInfo(provider.to_string()).increment(),
    }
}

pub fn observe_http_failure(provider: &str, operation: Operation, status: StatusCode) {
    observe_failure(provider, operation, http_failure_kind(status));
}

pub fn observe_failure(provider: &str, operation: Operation, kind: FailureKind) {
    Counter::Failure { provider: provider.to_string(),
                       operation,
                       kind }.increment();
}

fn http_failure_kind(status: StatusCode) -> FailureKind {
    if status.is_client_error() {
        FailureKind::Http4xx
    } else if status.is_server_error() {
        FailureKind::Http5xx
    } else {
        FailureKind::HttpOther
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use builder_core::metrics::Metric;

    #[test]
    fn authenticate_metric_keeps_existing_name() {
        assert_eq!(Counter::Authenticate("github".to_string()).id(),
                   "github.authenticate");
    }

    #[test]
    fn token_and_userinfo_metrics_use_provider_prefix() {
        assert_eq!(Counter::Token("github".to_string()).id(), "github.token");
        assert_eq!(Counter::UserInfo("github".to_string()).id(),
                   "github.userinfo");
    }

    #[test]
    fn failure_metrics_include_operation_and_kind() {
        assert_eq!(Counter::Failure { provider:  "github".to_string(),
                                      operation: Operation::Token,
                                      kind:      FailureKind::Http4xx, }.id(),
                   "github.token.failure.http-4xx");
    }

    #[test]
    fn http_failure_kind_maps_status_classes() {
        assert!(matches!(http_failure_kind(StatusCode::BAD_REQUEST),
                         FailureKind::Http4xx));
        assert!(matches!(http_failure_kind(StatusCode::BAD_GATEWAY),
                         FailureKind::Http5xx));
        assert!(matches!(http_failure_kind(StatusCode::MULTIPLE_CHOICES),
                         FailureKind::HttpOther));
    }
}
