use crate::{config::GitHubCfg,
            error::{HubError,
                    HubResult},
            jwt::{self,
                  Algorithm},
            metrics::Counter,
            types::*};
use builder_core::{http_client::{HttpClient,
                                 ACCEPT_GITHUB_JSON,
                                 CONTENT_TYPE_APPLICATION_JSON,
                                 USER_AGENT_BLDR},
                   metrics::CounterMetric};
use reqwest::{header::HeaderMap,
              StatusCode};

use std::{collections::HashMap,
          path::Path,
          time::{Duration,
                 SystemTime,
                 UNIX_EPOCH}};
use tokio::time::sleep;

pub type TokenString = String;
pub type InstallationId = u32;

/// Bundle up a Github token with the Github App installation ID used
/// to create it.
///
/// Consumers will treat this as an opaque type; its main utility is
/// in carrying the installation ID around so we can generate metrics
/// on a per-installation basis.
pub struct AppToken {
    inner_token: TokenString,

    // Leave this here in anticipation of using it for tagged metrics
    #[allow(dead_code)]
    installation_id: InstallationId,
}

impl AppToken {
    // Not public, because you should only get these from
    // `GitHubClient::app_installation_token`
    fn new(inner_token: TokenString, installation_id: InstallationId) -> Self {
        AppToken { inner_token,
                   installation_id }
    }

    // Only providing this for builder-worker's benefit... it
    // currently needs a token for cloning via libgit2; once that's
    // gone, just delete this function.
    /// Retrieve the actual token content for use in HTTP calls.
    pub fn inner_token(&self) -> &str { self.inner_token.as_ref() }
}

#[derive(Clone)]
pub struct GitHubClient {
    inner:              HttpClient,
    pub api_url:        String,
    app_id:             u32,
    app_private_key:    String,
    pub webhook_secret: String,
    resilience:         GitHubResilience,
}

#[derive(Clone, Copy, Debug)]
struct GitHubResilience {
    request_timeout: Duration,
    retry_backoff:   Duration,
    retry_attempts:  usize,
}

impl GitHubResilience {
    fn from_config(config: &GitHubCfg) -> Self {
        GitHubResilience { request_timeout: Duration::from_millis(config.request_timeout_ms),
                           retry_backoff:   Duration::from_millis(config.retry_backoff_ms),
                           retry_attempts:  config.retry_attempts, }
    }

    fn max_attempts(self) -> usize { self.retry_attempts + 1 }
}

impl GitHubClient {
    pub fn new(config: GitHubCfg) -> HubResult<Self> {
        let header_values = vec![USER_AGENT_BLDR.clone(),
                                 ACCEPT_GITHUB_JSON.clone(),
                                 CONTENT_TYPE_APPLICATION_JSON.clone(),];
        let headers = header_values.into_iter().collect::<HeaderMap<_>>();
        let resilience = GitHubResilience::from_config(&config);

        Ok(GitHubClient { inner:           HttpClient::new(&config.api_url, headers)?,
                          api_url:         config.api_url,
                          app_id:          config.app_id,
                          app_private_key: config.app_private_key,
                          webhook_secret:  config.webhook_secret,
                          resilience, })
    }

    pub async fn app(&self) -> HubResult<App> {
        let app_token = generate_app_token(&self.app_private_key, &self.app_id)?;
        let url_path = format!("{}/app", self.api_url);

        let rep = self.inner
                      .get(&url_path)
                      .bearer_auth(&app_token)
                      .send()
                      .await
                      .map_err(HubError::HttpClient)?;

        let status = rep.status();
        let body = rep.text().await?;
        debug!("GitHub response body, {}", body);

        if status != StatusCode::OK {
            let err: HashMap<String, String> = serde_json::from_str(&body)?;
            return Err(HubError::ApiError(status, err));
        }

        let contents = serde_json::from_str::<App>(&body)?;
        Ok(contents)
    }

    pub async fn app_installation_token(&self, install_id: u32) -> HubResult<AppToken> {
        let app_token = generate_app_token(&self.app_private_key, &self.app_id)?;

        let url_path = format!("{}/app/installations/{}/access_tokens",
                               self.api_url, install_id);
        debug!("app_installation_token posting to url path {:?}", url_path);
        Counter::InstallationToken.increment();

        let rep = self.inner
                      .post(&url_path)
                      .bearer_auth(&app_token)
                      .send()
                      .await
                      .map_err(HubError::HttpClient)?;

        let body = rep.text().await?;
        debug!("GitHub response body, {}", body);
        match serde_json::from_str::<AppInstallationToken>(&body) {
            Ok(msg) => Ok(AppToken::new(msg.token, install_id)),
            Err(_) => {
                let err = serde_json::from_str::<AppAuthErr>(&body)?;
                Err(HubError::AppAuth(err))
            }
        }
    }

    /// Returns the contents of a file or directory in a repository.
    pub async fn contents(&self,
                          token: &AppToken,
                          repo: u32,
                          path: &str)
                          -> HubResult<Option<Contents>> {
        let url_path = format!("{}/repositories/{}/contents/{}", self.api_url, repo, path);

        Counter::Contents.increment();

        let rep = self.inner
                      .get(&url_path)
                      .bearer_auth(&token.inner_token)
                      .send()
                      .await
                      .map_err(HubError::HttpClient)?;

        let status = rep.status();
        let body = rep.text().await?;
        debug!("GitHub response body, {}", body);
        match status {
            StatusCode::NOT_FOUND => return Ok(None),
            StatusCode::OK => (),
            status => {
                let err: HashMap<String, String> = serde_json::from_str(&body)?;
                return Err(HubError::ApiError(status, err));
            }
        }
        let mut contents: Contents = serde_json::from_str(&body)?;
        // We need to strip line feeds as the Github API has started to return
        // base64 content with line feeds.
        if contents.encoding == "base64" {
            contents.content = contents.content.replace('\n', "");
        }
        Ok(Some(contents))
    }

    /// Returns the directory listing of a path in a repository.
    pub async fn directory(&self,
                           token: &AppToken,
                           repo: u32,
                           path: &str)
                           -> HubResult<Option<Vec<DirectoryEntry>>> {
        let url_path = format!("{}/repositories/{}/contents/{}", self.api_url, repo, path);

        Counter::Contents.increment();
        let rep = self.inner
                      .get(&url_path)
                      .bearer_auth(&token.inner_token)
                      .send()
                      .await
                      .map_err(HubError::HttpClient)?;

        let status = rep.status();
        let body = rep.text().await?;
        debug!("GitHub response body, {}", body);
        match status {
            StatusCode::NOT_FOUND => return Ok(None),
            StatusCode::OK => (),
            status => {
                let err: HashMap<String, String> = serde_json::from_str(&body)?;
                return Err(HubError::ApiError(status, err));
            }
        }
        let directory: Vec<DirectoryEntry> = serde_json::from_str(&body)?;
        Ok(Some(directory))
    }

    pub async fn repo(&self, token: &AppToken, repo: u32) -> HubResult<Option<Repository>> {
        let url_path = format!("{}/repositories/{}", self.api_url, repo);
        Counter::Repo.increment();

        let rep = self.send_get_with_resilience("repo",
                                                || self.inner
                                                        .get(&url_path)
                                                        .bearer_auth(&token.inner_token))
                      .await?;

        let status = rep.status();
        let body = rep.text().await?;
        debug!("GitHub response body, {}", body);
        match status {
            StatusCode::NOT_FOUND => return Ok(None),
            StatusCode::OK => (),
            status => {
                let err: HashMap<String, String> = serde_json::from_str(&body)?;
                return Err(HubError::ApiError(status, err));
            }
        }
        let value = serde_json::from_str(&body)?;
        Ok(Some(value))
    }

    // The main purpose of this is just to verify HTTP communication with GH.
    // There's nothing special about this endpoint, only that it doesn't require
    // auth and the response body seemed small. We don't even care what the
    // response is. For our purposes, just receiving a response is enough.
    pub async fn meta(&self) -> HubResult<()> {
        let url_path = format!("{}/meta", self.api_url);

        let rep = self.send_get_with_resilience("meta", || self.inner.get(&url_path))
                      .await?;

        let status = rep.status();
        let body = rep.text().await?;
        debug!("GitHub response body, {}", body);

        if status != StatusCode::OK {
            let err: HashMap<String, String> = serde_json::from_str(&body)?;
            return Err(HubError::ApiError(status, err));
        }

        Ok(())
    }

    /// Retry idempotent GitHub GET calls on transient transport failures and retryable
    /// response statuses. Do not use this helper for POST/PUT/DELETE flows.
    async fn send_get_with_resilience<F>(&self,
                                         operation: &str,
                                         build_request: F)
                                         -> HubResult<reqwest::Response>
        where F: Fn() -> reqwest::RequestBuilder
    {
        let max_attempts = self.resilience.max_attempts();

        for attempt in 1..=max_attempts {
            match build_request().timeout(self.resilience.request_timeout)
                                 .send()
                                 .await
            {
                Ok(response) => {
                    if attempt < max_attempts {
                        if let Some(delay) = retry_delay_for_response(&response,
                                                                      self.resilience
                                                                          .retry_backoff)
                        {
                            warn!("GitHub {} received retryable status {}; retrying in {:?} \
                                   (attempt {}/{})",
                                  operation,
                                  response.status(),
                                  delay,
                                  attempt,
                                  max_attempts);
                            sleep(delay).await;
                            continue;
                        }
                    }

                    return Ok(response);
                }
                Err(err) => {
                    if attempt < max_attempts && is_retryable_transport_error(&err) {
                        warn!("GitHub {} failed with retryable transport error {}; retrying in \
                               {:?} (attempt {}/{})",
                              operation,
                              err,
                              self.resilience.retry_backoff,
                              attempt,
                              max_attempts);
                        sleep(self.resilience.retry_backoff).await;
                        continue;
                    }

                    return Err(HubError::HttpClient(err));
                }
            }
        }

        unreachable!("retry loop must return a response or error")
    }
}

fn is_retryable_transport_error(err: &reqwest::Error) -> bool { err.is_connect() || err.is_timeout() }

fn retry_delay_for_response(response: &reqwest::Response, fallback: Duration) -> Option<Duration> {
    match response.status() {
        StatusCode::TOO_MANY_REQUESTS => retry_after_delay(response.headers()).or(Some(fallback)),
        StatusCode::REQUEST_TIMEOUT
        | StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => Some(fallback),
        _ => None,
    }
}

fn retry_after_delay(headers: &HeaderMap) -> Option<Duration> {
    headers.get("retry-after")
           .and_then(|value| value.to_str().ok())
           .and_then(|value| value.parse::<u64>().ok())
           .map(Duration::from_secs)
           .or_else(|| {
               headers.get("x-ratelimit-reset")
                      .and_then(|value| value.to_str().ok())
                      .and_then(|value| value.parse::<u64>().ok())
                      .map(rate_limit_reset_delay)
           })
}

fn rate_limit_reset_delay(reset_at_epoch_seconds: u64) -> Duration {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    Duration::from_secs(reset_at_epoch_seconds.saturating_sub(now))
}

fn generate_app_token<T, U>(key_path: T, app_id: &U) -> HubResult<String>
    where T: AsRef<Path>,
          U: ToString
{
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let expiration = now + Duration::from_secs(10 * 10);
    let payload = json!({
        "iat" : now.as_secs(),
        "exp" : expiration.as_secs(),
        "iss" : app_id.to_string()});
    debug!("Payload = {:?}", payload);

    let header = json!({});
    let res = jwt::encode(header,
                          &key_path.as_ref().to_path_buf(),
                          &payload,
                          Algorithm::RS256).map_err(HubError::JWT);

    if let Ok(ref t) = res {
        debug!("Encoded JWT token = {}", t);
    };

    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use std::{collections::VecDeque,
              env,
              io::{Read,
                   Write},
              net::TcpListener,
              sync::{Arc,
                     Mutex},
              thread};

    #[derive(Clone)]
    struct ScriptedResponse {
        status_code: u16,
        body:        &'static str,
        delay:       Duration,
        headers:     Vec<(&'static str, &'static str)>,
    }

    fn spawn_scripted_server(responses: Vec<ScriptedResponse>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
        let total = responses.lock().unwrap().len();

        thread::spawn(move || {
            for _ in 0..total {
                let (mut stream, _) = listener.accept().unwrap();
                let scripted = responses.lock().unwrap().pop_front().unwrap();

                thread::spawn(move || {
                    let mut request_buffer = [0; 4096];
                    let _ = stream.read(&mut request_buffer);
                    thread::sleep(scripted.delay);

                    let reason_phrase = match scripted.status_code {
                        200 => "OK",
                        404 => "Not Found",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
                        503 => "Service Unavailable",
                        _ => "Error",
                    };

                    let mut response =
                        format!("HTTP/1.1 {} {}\r\nContent-Type: application/json\r\n\
                                 Content-Length: {}\r\nConnection: close\r\n",
                                scripted.status_code,
                                reason_phrase,
                                scripted.body.len());

                    for (name, value) in scripted.headers {
                        response.push_str(&format!("{}: {}\r\n", name, value));
                    }

                    response.push_str("\r\n");

                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(scripted.body.as_bytes());
                });
            }
        });

        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn use_a_proxy_from_the_env() {
        let proxy = env::var_os("HTTPS_PROXY");

        if let Some(p) = proxy {
            let pp = p.to_string_lossy();

            if !pp.is_empty() {
                let cfg = config::GitHubCfg::default();
                let client = GitHubClient::new(cfg).unwrap();
                assert!(client.meta().await.is_ok());
            }
        }
    }

    #[tokio::test]
    async fn meta_retries_after_server_error() {
        let api_url =
            spawn_scripted_server(vec![ScriptedResponse { status_code: 500,
                                                         body:        r#"{"message":"retry me"}"#,
                                                         delay:       Duration::from_millis(0),
                                                         headers:     vec![], },
                                      ScriptedResponse { status_code: 200,
                                                         body:        "{}",
                                                         delay:       Duration::from_millis(0),
                                                         headers:     vec![], },]);
        let cfg = config::GitHubCfg { api_url,
                                      request_timeout_ms: 100,
                                      retry_backoff_ms:   10,
                                      retry_attempts:     1,
                                      ..Default::default() };
        let client = GitHubClient::new(cfg).unwrap();

        assert!(client.meta().await.is_ok());
    }

    #[tokio::test]
    async fn meta_returns_api_error_after_retries_are_exhausted() {
        let api_url =
            spawn_scripted_server(vec![ScriptedResponse { status_code: 500,
                                                         body:        r#"{"message":"still broken"}"#,
                                                         delay:       Duration::from_millis(0),
                                                         headers:     vec![], },
                                      ScriptedResponse { status_code: 500,
                                                         body:        r#"{"message":"still broken"}"#,
                                                         delay:       Duration::from_millis(0),
                                                         headers:     vec![], },]);
        let cfg = config::GitHubCfg { api_url,
                                      request_timeout_ms: 100,
                                      retry_backoff_ms:   10,
                                      retry_attempts:     1,
                                      ..Default::default() };
        let client = GitHubClient::new(cfg).unwrap();

        match client.meta().await.unwrap_err() {
            HubError::ApiError(status, payload) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(payload.get("message"), Some(&"still broken".to_string()));
            }
            err => panic!("expected api error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn repo_retries_after_timeout() {
        let api_url = spawn_scripted_server(vec![ScriptedResponse {
                                                     status_code: 200,
                                                     body:        r#"{"id":1,"name":"slow"}"#,
                                                     delay:       Duration::from_millis(100),
                                                     headers:     vec![],
                                                 },
                                                 ScriptedResponse {
                                                     status_code: 200,
                                                     body:        r#"{"id":7,"name":"stable"}"#,
                                                     delay:       Duration::from_millis(0),
                                                     headers:     vec![],
                                                 }]);
        let cfg = config::GitHubCfg { api_url,
                                      request_timeout_ms: 30,
                                      retry_backoff_ms:   10,
                                      retry_attempts:     1,
                                      ..Default::default() };
        let client = GitHubClient::new(cfg).unwrap();
        let token = AppToken::new("test-token".to_string(), 42);

        let repo = client.repo(&token, 7).await.unwrap().unwrap();

        assert_eq!(repo.id, 7);
        assert_eq!(repo.name, "stable");
    }
}
