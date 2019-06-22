use std::ops::Deref;

use reqwest::{header::{HeaderMap,
                       HeaderName,
                       HeaderValue,
                       ACCEPT,
                       CONTENT_TYPE,
                       USER_AGENT},
              Client,
              Proxy};

use url::Url;

const BLDR_USER_AGENT: &str = "Habitat-Builder";
const APPLICATION_JSON: &str = "application/json";
const FORM_URL_ENCODED: &str = "application/x-www-form-urlencoded";
const GITHUB_JSON: &str = "application/json, application/vnd.github.v3+json, \
                           application/vnd.github.machine-man-preview+json";
const X_FILENAME: &str = "x-filename";

lazy_static::lazy_static! {
    pub static ref USER_AGENT_BLDR: (HeaderName, HeaderValue) = (USER_AGENT, HeaderValue::from_static(BLDR_USER_AGENT));
    pub static ref ACCEPT_APPLICATION_JSON: (HeaderName, HeaderValue) = (ACCEPT, HeaderValue::from_static(APPLICATION_JSON));
    pub static ref ACCEPT_GITHUB_JSON: (HeaderName, HeaderValue) = (ACCEPT, HeaderValue::from_static(GITHUB_JSON));
    pub static ref CONTENT_TYPE_APPLICATION_JSON: (HeaderName, HeaderValue) = (CONTENT_TYPE, HeaderValue::from_static(APPLICATION_JSON));
    pub static ref CONTENT_TYPE_FORM_URL_ENCODED: (HeaderName, HeaderValue) = (CONTENT_TYPE, HeaderValue::from_static(FORM_URL_ENCODED));
    pub static ref XFILENAME: HeaderName = HeaderName::from_static(X_FILENAME);
}

#[derive(Clone)]
pub struct HttpClient(Client);

impl HttpClient {
    pub fn new(url: &str, headers: HeaderMap) -> Self {
        let mut client = Client::builder();

        trace!("HttpClient: checking proxy for url: {:?}", url);
        let url = Url::parse(url).expect("valid client url must be configured");

        if let Some(proxy_url) = env_proxy::for_url(&url).to_string() {
            if url.scheme() == "http" {
                trace!("Setting http_proxy to {}", proxy_url);
                match Proxy::http(&proxy_url) {
                    Ok(p) => {
                        client = client.proxy(p);
                    }
                    Err(e) => warn!("Invalid proxy, err: {:?}", e),
                }
            }

            if url.scheme() == "https" {
                trace!("Setting https proxy to {}", proxy_url);
                match Proxy::https(&proxy_url) {
                    Ok(p) => {
                        client = client.proxy(p);
                    }
                    Err(e) => warn!("Invalid proxy, err: {:?}", e),
                }
            }
        } else {
            trace!("No proxy configured for url: {:?}", url);
        }

        HttpClient(client.default_headers(headers).build().unwrap())
    }
}

impl Deref for HttpClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target { &self.0 }
}
