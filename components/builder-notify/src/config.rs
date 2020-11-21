use habitat_builder_events::connection::EventConfig;
use habitat_core::config::ConfigFile;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub eventbus: EventConfig,
}

impl ConfigFile for Config {
    type Error = habitat_core::Error;
}
