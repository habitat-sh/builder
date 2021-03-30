use crate::webhook_client::WebhookClient;
use std::result::Result;

/// A Webhook
#[derive(Default)]
pub struct Webhook {
    pub endpoint: String,
    pub client:   WebhookClient,
}

impl Webhook {
    pub fn new(endpoint: &str) -> Webhook {
        let client = WebhookClient::new(endpoint).unwrap();
        Webhook { endpoint: endpoint.to_owned(),
                  client }
    }

    pub async fn deliver(&self,
                         event_data: &str)
                         -> Result<(), Box<dyn std::error::Error + 'static>> {
        let response = self.client.push(event_data).await;
        match response {
            Ok(_) => Ok(()),
            Err(err) => Err(Box::new(err)),
        }
    }
}
