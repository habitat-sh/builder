use crate::error::{Error,
                   Result};
use builder_core::http_client::{ACCEPT_APPLICATION_JSON,
                                USER_AGENT_BLDR};
use reqwest::{header::HeaderMap,
              Body,
              Client,
              Response};
use std::iter::FromIterator;

#[derive(Clone, Default)]
pub struct WebhookClient {
    inner: Client,
    url:   String,
}

impl WebhookClient {
    pub fn new(url: &str) -> Result<Self> {
        let header_values = vec![USER_AGENT_BLDR.clone(), ACCEPT_APPLICATION_JSON.clone()];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        let client = reqwest::Client::builder().default_headers(headers)
                                               .build()?;

        Ok(WebhookClient { inner: client,
                           url:   url.to_owned(), })
    }

    pub async fn push(&self, payload: &str) -> Result<Response> {
        debug!("WebhookClient push url = {}", self.url);

        let body: Body = payload.to_string().into();

        let resp = self.inner
                       .post(&self.url)
                       .body(body)
                       .send()
                       .await
                       .map_err(Error::WebhookClientUpload)?;

        debug!("WebhookClient response status: {:?}", resp.status());

        if resp.status().is_success() {
            Ok(resp)
        } else {
            Err(Error::WebhookPushError(resp.status(), resp.text().await?))
        }
    }
}
