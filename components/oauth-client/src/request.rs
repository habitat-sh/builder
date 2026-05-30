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

use std::{future::Future,
          time::Duration};

use reqwest::{RequestBuilder,
              Response};

use crate::{config::OAuth2Cfg,
            logging::debug_retry_attempt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RequestPolicy {
    timeout:      Duration,
    retry_count:  u32,
    backoff_base: Duration,
}

impl RequestPolicy {
    pub(crate) fn from_config(config: &OAuth2Cfg) -> Self {
        Self { timeout:      Duration::from_millis(config.request_timeout_ms),
               retry_count:  config.request_retry_count,
               backoff_base: Duration::from_millis(config.request_backoff_base_ms), }
    }

    pub(crate) fn backoff_delay(&self, attempt: u32) -> Duration {
        let multiplier = 1u32.checked_shl(attempt.saturating_sub(1))
                             .unwrap_or(u32::MAX);
        self.backoff_base.saturating_mul(multiplier)
    }
}

pub(crate) async fn send_with_retry<F>(config: &OAuth2Cfg,
                                       operation_name: &str,
                                       request: F)
                                       -> reqwest::Result<Response>
    where F: Fn() -> RequestBuilder
{
    let policy = RequestPolicy::from_config(config);

    execute_with_backoff(policy,
                         should_retry_reqwest_error,
                         tokio::time::sleep,
                         || request().timeout(policy.timeout).send(),
                         |attempt, delay, err| {
                             debug_retry_attempt(&config.provider,
                                                 operation_name,
                                                 attempt,
                                                 delay,
                                                 err);
                         }).await
}

async fn execute_with_backoff<T, E, A, AFut, S, SFut, P, O>(policy: RequestPolicy,
                                                            should_retry: P,
                                                            mut sleep: S,
                                                            mut action: A,
                                                            mut on_retry: O)
                                                            -> Result<T, E>
    where A: FnMut() -> AFut,
          AFut: Future<Output = Result<T, E>>,
          S: FnMut(Duration) -> SFut,
          SFut: Future<Output = ()>,
          P: Fn(&E) -> bool,
          O: FnMut(u32, Duration, &E)
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        match action().await {
            Ok(value) => return Ok(value),
            Err(err) if attempt <= policy.retry_count && should_retry(&err) => {
                let delay = policy.backoff_delay(attempt);
                on_retry(attempt, delay, &err);
                sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }
}

fn should_retry_reqwest_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request() || err.is_body()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{Arc,
                    Mutex};

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum FakeError {
        Retryable,
        Terminal,
    }

    #[test]
    fn policy_uses_config_tuning() {
        let policy = RequestPolicy::from_config(&OAuth2Cfg { request_timeout_ms: 4_000,
                                                             request_retry_count: 3,
                                                             request_backoff_base_ms: 125,
                                                             ..OAuth2Cfg::default() });

        assert_eq!(policy.timeout, Duration::from_millis(4_000));
        assert_eq!(policy.retry_count, 3);
        assert_eq!(policy.backoff_base, Duration::from_millis(125));
    }

    #[test]
    fn backoff_delay_doubles_per_retry_attempt() {
        let policy = RequestPolicy { timeout:      Duration::from_secs(1),
                                     retry_count:  2,
                                     backoff_base: Duration::from_millis(250), };

        assert_eq!(policy.backoff_delay(1), Duration::from_millis(250));
        assert_eq!(policy.backoff_delay(2), Duration::from_millis(500));
    }

    #[tokio::test]
    async fn retries_retryable_failures_until_success() {
        let policy = RequestPolicy { timeout:      Duration::from_secs(1),
                                     retry_count:  2,
                                     backoff_base: Duration::from_millis(10), };
        let attempts = Arc::new(Mutex::new(0));
        let sleeps = Arc::new(Mutex::new(Vec::new()));

        let result = execute_with_backoff(policy,
                                          |err| matches!(err, FakeError::Retryable),
                                          {
                                              let sleeps = Arc::clone(&sleeps);
                                              move |delay| {
                                                  let sleeps = Arc::clone(&sleeps);
                                                  async move {
                                                      sleeps.lock().unwrap().push(delay);
                                                  }
                                              }
                                          },
                                          {
                                              let attempts = Arc::clone(&attempts);
                                              move || {
                                                  let attempts = Arc::clone(&attempts);
                                                  async move {
                                                      let mut attempts = attempts.lock().unwrap();
                                                      *attempts += 1;
                                                      if *attempts < 3 {
                                                          Err(FakeError::Retryable)
                                                      } else {
                                                          Ok("ok")
                                                      }
                                                  }
                                              }
                                          },
                                          |_, _, _| {}).await;

        assert_eq!(result, Ok("ok"));
        assert_eq!(*attempts.lock().unwrap(), 3);
        assert_eq!(*sleeps.lock().unwrap(),
                   vec![Duration::from_millis(10), Duration::from_millis(20)]);
    }

    #[tokio::test]
    async fn does_not_retry_terminal_failures() {
        let policy = RequestPolicy { timeout:      Duration::from_secs(1),
                                     retry_count:  3,
                                     backoff_base: Duration::from_millis(10), };
        let attempts = Arc::new(Mutex::new(0));

        let result = execute_with_backoff(policy,
                                          |err| matches!(err, FakeError::Retryable),
                                          |_| async {},
                                          {
                                              let attempts = Arc::clone(&attempts);
                                              move || {
                                                  let attempts = Arc::clone(&attempts);
                                                  async move {
                                                      *attempts.lock().unwrap() += 1;
                                                      Err::<(), _>(FakeError::Terminal)
                                                  }
                                              }
                                          },
                                          |_, _, _| {}).await;

        assert_eq!(result, Err(FakeError::Terminal));
        assert_eq!(*attempts.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn stops_after_retry_budget_is_exhausted() {
        let policy = RequestPolicy { timeout:      Duration::from_secs(1),
                                     retry_count:  2,
                                     backoff_base: Duration::from_millis(10), };
        let attempts = Arc::new(Mutex::new(0));

        let result = execute_with_backoff(policy,
                                          |err| matches!(err, FakeError::Retryable),
                                          |_| async {},
                                          {
                                              let attempts = Arc::clone(&attempts);
                                              move || {
                                                  let attempts = Arc::clone(&attempts);
                                                  async move {
                                                      *attempts.lock().unwrap() += 1;
                                                      Err::<(), _>(FakeError::Retryable)
                                                  }
                                              }
                                          },
                                          |_, _, _| {}).await;

        assert_eq!(result, Err(FakeError::Retryable));
        assert_eq!(*attempts.lock().unwrap(), 3);
    }
}
