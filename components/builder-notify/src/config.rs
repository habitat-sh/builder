use crate::{bldr_events::connection::EventConfig,
            hab_core::config::ConfigFile};
use std::{error,
          fmt};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub eventbus: EventConfig,
}

#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", *self) }
}

impl error::Error for ConfigError {}

impl ConfigFile for Config {
    type Error = ConfigError;
}

impl From<habitat_core::Error> for ConfigError {
    fn from(err: habitat_core::Error) -> ConfigError { ConfigError(format!("{:?}", err)) }
}
