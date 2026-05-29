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

use reqwest::StatusCode;

pub(crate) fn debug_response(provider: &str, operation: &str, status: StatusCode, body: &str) {
    debug!("{} {} response: {}",
           provider,
           operation,
           response_summary(status, body));
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
}
