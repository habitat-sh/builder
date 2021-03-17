#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

pub mod cli;
pub mod config;
pub mod error;
pub mod hook;
pub mod hub;
pub mod server;

use crate::config::Config;
// use crate::hook::Hook;
use crate::hub::Hub;

pub fn get_hub(config: &Config) -> Hub {
    debug!("NotifyConfig {:?}", config);
    let hub = Hub::new();
    // Add the hooks

    hub
}
