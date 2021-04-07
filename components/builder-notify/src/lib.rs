extern crate features;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

pub mod cli;
pub mod config;
pub mod error;
pub mod hub;
pub mod server;
pub mod webhook;
pub mod webhook_client;

use crate::{config::Config,
            hub::Hub,
            webhook::Webhook};

pub fn get_hub(config: &Config) -> Hub {
    debug!("NotifyConfig {:?}", config);
    let mut hub = Hub::new();
    let webhooks = &config.hub.webhooks;
    for webhook in webhooks {
        hub.add(Webhook::new(&webhook.endpoint, &webhook.auth_header));
    }

    hub
}
