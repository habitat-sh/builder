use crate::{error::Error,
            event::BuilderEvent,
            kafka::KafkaProducer};
use async_trait::async_trait;
use habitat_core::util;
use std::{convert::TryFrom,
          fmt,
          result,
          str::FromStr,
          time::Duration};
use url::Url;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum Provider {
    Kafka,
}

impl Provider {
    pub const KAFKA: &'static str = "kafka";
}

impl FromStr for Provider {
    type Err = Error;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            Provider::KAFKA => Ok(Provider::Kafka),
            _ => Err(Error::BadProvider(value.to_string())),
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self) }
}

impl Default for Provider {
    fn default() -> Provider { Provider::Kafka }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct EventBusCfg {
    pub api_key:                String,
    pub api_secret_key:         String,
    pub bootstrap_nodes:        Vec<String>,
    pub client_id:              Url,
    #[serde(with = "deserialize_into_duration")]
    pub connection_retry_delay: Duration,
    pub message_timeout_ms:     u64,
    #[serde(with = "util::serde::string")]
    pub provider:               Provider,
}

impl Default for EventBusCfg {
    fn default() -> Self {
        EventBusCfg { api_key:                "API_KEY".to_string(),
                      api_secret_key:         "API_SECRET_KEY".to_string(),
                      bootstrap_nodes:        vec!["localhost:9092".to_string()],
                      client_id:              "http://localhost".parse().expect("CLIENT_ID URL"),
                      connection_retry_delay: Duration::from_secs(3),
                      message_timeout_ms:     3000,
                      provider:               Provider::default(), }
    }
}

mod deserialize_into_duration {
    use serde::{self,
                Deserialize,
                Deserializer};
    use std::time::Duration;
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
        where D: Deserializer<'de>
    {
        let s = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(s))
    }
}

pub fn event_producer(config: &EventBusCfg) -> Result<Box<dyn EventBusProducer>, Error> {
    match config.provider {
        Provider::Kafka => {
            match KafkaProducer::try_from(&config.clone()) {
                Ok(client) => {
                    info!("EventBusClient (Kafka) ready to go.");
                    Ok(Box::new(client))
                }
                Err(e) => {
                    warn!("Unable to load EventBusClient (Kafka): {}", e);
                    Err(e.into())
                }
            }
        }
    }
}

#[async_trait]
pub trait EventBusProducer {
    async fn send(&self, event: BuilderEvent);
}
