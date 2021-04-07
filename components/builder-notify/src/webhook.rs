use crate::webhook_client::WebhookClient;
use std::result::Result;

/// A Webhook
#[derive(Clone, Default)]
pub struct Webhook {
    pub endpoint: String,
    pub client:   WebhookClient,
}

impl Webhook {
    pub fn new(endpoint: &str, auth_header: &str) -> Webhook {
        let client = WebhookClient::new(endpoint, auth_header).unwrap();
        Webhook { endpoint: endpoint.to_owned(),
                  client }
    }

    pub async fn deliver(&self,
                         event_data: String)
                         -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let response = self.client.push(&event_data).await;
        match response {
            Ok(_) => Ok(()),
            Err(_err) => Err("Could not deliver!".into()),
        }
    }
}
