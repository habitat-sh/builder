extern crate features;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

pub mod cli;
pub mod client;
pub mod config;
pub mod error;
pub mod hook;
pub mod hub;
pub mod server;

use crate::{client::WebhookClient,
            config::Config,
            hook::Webhook,
            hub::Hub};

pub fn get_hub(config: &Config) -> Hub {
    debug!("NotifyConfig {:?}", config);
    let mut hub = Hub::new();
    let webhooks = &config.hub.webhooks;
    for webhook in webhooks {
        hub.add(Webhook { endpoint: webhook.endpoint.clone(),
                          client:   WebhookClient::new().unwrap(), });
    }

    hub
}
