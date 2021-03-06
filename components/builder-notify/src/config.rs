use crate::error::Error;
use builder_core::config::ConfigFile;
use habitat_builder_events::connection::EventConfig;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub eventbus: EventConfig,
}

impl ConfigFile for Config {
    type Error = Error;
}
