use crate::{
    config::GitHubCfg,
    error::{HubError, HubResult},
    jwt::{self, Algorithm},
    metrics::Counter,
    types::*,
};
use builder_core::{
    http_client::{HttpClient, ACCEPT_GITHUB_JSON, CONTENT_TYPE_APPLICATION_JSON, USER_AGENT_BLDR},
    metrics::CounterMetric,
};
use reqwest::{header::HeaderMap, StatusCode};

use std::{
    collections::HashMap,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

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
        AppToken {
            inner_token,
            installation_id,
        }
    }

    // Only providing this for builder-worker's benefit... it
    // currently needs a token for cloning via libgit2; once that's
    // gone, just delete this function.
    /// Retrieve the actual token content for use in HTTP calls.
    pub fn inner_token(&self) -> &str {
        self.inner_token.as_ref()
    }
}

#[derive(Clone)]
pub struct GitHubClient {
    inner: HttpClient,
    pub api_url: String,
    app_id: u32,
    app_private_key: String,
    pub webhook_secret: String,
}

impl GitHubClient {
    pub fn new(config: GitHubCfg) -> HubResult<Self> {
        let header_values = vec![
            USER_AGENT_BLDR.clone(),
            ACCEPT_GITHUB_JSON.clone(),
            CONTENT_TYPE_APPLICATION_JSON.clone(),
        ];
        let headers = header_values.into_iter().collect::<HeaderMap<_>>();

        Ok(GitHubClient {
            inner: HttpClient::new(&config.api_url, headers)?,
            api_url: config.api_url,
            app_id: config.app_id,
            app_private_key: config.app_private_key,
            webhook_secret: config.webhook_secret,
        })
    }

    pub async fn app(&self) -> HubResult<App> {
        let app_token = generate_app_token(&self.app_private_key, &self.app_id)?;
        let url_path = format!("{}/app", self.api_url);

        let rep = self
            .inner
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

        let url_path = format!(
            "{}/app/installations/{}/access_tokens",
            self.api_url, install_id
        );
        debug!("app_installation_token posting to url path {:?}", url_path);
        Counter::InstallationToken.increment();

        let rep = self
            .inner
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
    pub async fn contents(
        &self,
        token: &AppToken,
        repo: u32,
        path: &str,
    ) -> HubResult<Option<Contents>> {
        let url_path = format!("{}/repositories/{}/contents/{}", self.api_url, repo, path);

        Counter::Contents.increment();

        let rep = self
            .inner
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
    pub async fn directory(
        &self,
        token: &AppToken,
        repo: u32,
        path: &str,
    ) -> HubResult<Option<Vec<DirectoryEntry>>> {
        let url_path = format!("{}/repositories/{}/contents/{}", self.api_url, repo, path);

        Counter::Contents.increment();
        let rep = self
            .inner
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

        let rep = self
            .inner
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
        let value = serde_json::from_str(&body)?;
        Ok(Some(value))
    }

    // The main purpose of this is just to verify HTTP communication with GH.
    // There's nothing special about this endpoint, only that it doesn't require
    // auth and the response body seemed small. We don't even care what the
    // response is. For our purposes, just receiving a response is enough.
    pub async fn meta(&self) -> HubResult<()> {
        let url_path = format!("{}/meta", self.api_url);

        let rep = self
            .inner
            .get(&url_path)
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

        Ok(())
    }
}

fn generate_app_token<T, U>(key_path: T, app_id: &U) -> HubResult<String>
where
    T: AsRef<Path>,
    U: ToString,
{
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let expiration = now + Duration::from_secs(10 * 10);
    let payload = json!({
        "iat" : now.as_secs(),
        "exp" : expiration.as_secs(),
        "iss" : app_id.to_string()});
    debug!("Payload = {:?}", payload);

    let header = json!({});
    let res = jwt::encode(
        header,
        &key_path.as_ref().to_path_buf(),
        &payload,
        Algorithm::RS256,
    )
    .map_err(HubError::JWT);

    if let Ok(ref t) = res {
        debug!("Encoded JWT token = {}", t);
    };

    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use std::{
        env,
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    fn test_client(api_url: String) -> GitHubClient {
        let mut cfg = config::GitHubCfg::default();
        cfg.api_url = api_url;
        GitHubClient::new(cfg).unwrap()
    }

    fn test_token() -> AppToken {
        AppToken::new("opaque-token".to_string(), 42)
    }

    fn spawn_server(
        status_line: &str,
        body: &str,
        expected_request_line: &str,
        expected_auth: Option<&str>,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let status_line = status_line.to_string();
        let body = body.to_string();
        let expected_request_line = expected_request_line.to_string();
        let expected_auth = expected_auth.map(str::to_string);
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();

            let mut request = Vec::new();
            loop {
                let mut buffer = [0; 1024];
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            let request = String::from_utf8(request).unwrap();
            assert!(
                request.starts_with(&expected_request_line),
                "unexpected request line: {:?}",
                request
            );
            if let Some(auth) = expected_auth {
                assert!(
                    request.contains(&format!("authorization: Bearer {auth}\r\n")),
                    "missing authorization header: {:?}",
                    request
                );
            }

            write!(
                stream,
                "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: \
                    {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            stream.flush().unwrap();
        });

        (format!("http://{}", addr), handle)
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
    async fn meta_returns_ok_on_200() {
        let (api_url, server) = spawn_server("200 OK", "{}", "GET /meta HTTP/1.1", None);
        let client = test_client(api_url);

        client.meta().await.unwrap();

        server.join().unwrap();
    }

    #[tokio::test]
    async fn meta_returns_api_error_on_non_200() {
        let (api_url, server) = spawn_server(
            "500 Internal Server Error",
            r#"{"message":"boom"}"#,
            "GET /meta HTTP/1.1",
            None,
        );
        let client = test_client(api_url);

        let err = client.meta().await.err().unwrap();
        match err {
            HubError::ApiError(StatusCode::INTERNAL_SERVER_ERROR, response) => {
                assert_eq!(response.get("message"), Some(&"boom".to_string()));
            }
            other => panic!("unexpected error: {:?}", other),
        }

        server.join().unwrap();
    }

    #[tokio::test]
    async fn contents_strips_newlines_from_base64_payloads() {
        let (api_url, server) = spawn_server(
            "200 OK",
            r#"{"name":"plan.sh","path":"support/ci/plan.sh","sha":"abc123","size":8,"url":"https://example.test/url","html_url":"https://example.test/html","git_url":"https://example.test/git","download_url":"https://example.test/download","content":"aGVs\nbG8=","encoding":"base64"}"#,
            "GET /repositories/77/contents/support/ci/plan.sh HTTP/1.1",
            Some("opaque-token"),
        );
        let client = test_client(api_url);

        let contents = client
            .contents(&test_token(), 77, "support/ci/plan.sh")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(contents.content, "aGVsbG8=");

        server.join().unwrap();
    }

    #[tokio::test]
    async fn contents_returns_none_on_not_found() {
        let (api_url, server) = spawn_server(
            "404 Not Found",
            r#"{"message":"missing"}"#,
            "GET /repositories/88/contents/missing.txt HTTP/1.1",
            Some("opaque-token"),
        );
        let client = test_client(api_url);

        let contents = client
            .contents(&test_token(), 88, "missing.txt")
            .await
            .unwrap();
        assert!(contents.is_none());

        server.join().unwrap();
    }

    #[tokio::test]
    async fn directory_returns_entries_on_200() {
        let (api_url, server) = spawn_server(
            "200 OK",
            r#"[{"type":"file","name":"plan.sh","path":"support/ci/plan.sh","sha":"abc123","size":8,"url":"https://example.test/url","html_url":"https://example.test/html","git_url":"https://example.test/git","download_url":"https://example.test/download"}]"#,
            "GET /repositories/99/contents/support/ci HTTP/1.1",
            Some("opaque-token"),
        );
        let client = test_client(api_url);

        let directory = client
            .directory(&test_token(), 99, "support/ci")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(directory.len(), 1);
        assert_eq!(directory[0].name, "plan.sh");

        server.join().unwrap();
    }
}
