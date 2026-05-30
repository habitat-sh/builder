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

use std::time::Duration;

use reqwest::StatusCode;

pub(crate) fn debug_authenticate_start(provider: &str) {
    debug!("{} authenticate start", provider);
}

pub(crate) fn debug_response(provider: &str, operation: &str, status: StatusCode, body: &str) {
    debug!("{} {} response: {}",
           provider,
           operation,
           response_summary(status, body));
}

pub(crate) fn debug_retry_attempt(provider: &str,
                                  operation: &str,
                                  attempt: u32,
                                  delay: Duration,
                                  error: &reqwest::Error) {
    debug!("{} {} retry attempt={} delay_ms={} err={:?}",
           provider,
           operation,
           attempt,
           delay.as_millis(),
           error);
}

pub(crate) fn redacted_body(body: &str) -> String { format!("<redacted {} bytes>", body.len()) }

fn response_summary(status: StatusCode, body: &str) -> String {
    format!("status={}, body={}", status, redacted_body(body))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn redacted_body_reports_only_length() {
        assert_eq!(redacted_body("access_token=secret"), "<redacted 19 bytes>");
    }

    #[test]
    fn response_summary_uses_redacted_body() {
        assert_eq!(response_summary(StatusCode::BAD_REQUEST, "{\"error\":\"invalid\"}"),
                   "status=400 Bad Request, body=<redacted 19 bytes>");
    }

    #[test]
    fn debug_authenticate_start_uses_provider_key() { debug_authenticate_start("chef-automate"); }

    #[test]
    fn debug_retry_attempt_uses_provider_key() {
        let err = reqwest::Client::new().get("http://").build().unwrap_err();
        debug_retry_attempt("github", "token", 2, Duration::from_millis(250), &err);
    }
}
