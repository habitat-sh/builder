use crate::error::{Error,
                   Result};
use builder_core::http_client::{ACCEPT_APPLICATION_JSON,
                                USER_AGENT_BLDR};
use reqwest::{header::HeaderMap,
              Body,
              Client,
              Response};
use std::{collections::HashMap,
          iter::FromIterator};

#[derive(Clone, Default)]
pub struct WebhookClient {
    inner: Client,
}

impl WebhookClient {
    pub fn new() -> Result<Self> {
        let header_values = vec![USER_AGENT_BLDR.clone(), ACCEPT_APPLICATION_JSON.clone()];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        let client = reqwest::Client::builder().default_headers(headers)
                                               .build()?;

        Ok(WebhookClient { inner: client })
    }

    pub async fn push(&self, url: &str, payload: &str) -> Result<Response> {
        debug!("WebhookClient push url = {}", url);

        let body: Body = payload.to_string().into();

        let resp = match self.inner
                             .post(url)
                             .body(body)
                             .send()
                             .await
                             .map_err(Error::HttpClient)
        {
            Ok(resp) => resp,
            Err(err) => {
                error!("WebhookClient upload failed, err={}", err);
                return Err(err);
            }
        };

        debug!("WebhookClient response status: {:?}", resp.status());

        if resp.status().is_success() {
            Ok(resp)
        } else {
            error!("WebhookClient push non-success status: {:?}", resp.status());
            Err(Error::ApiError(resp.status(), HashMap::new()))
        }
    }
}
