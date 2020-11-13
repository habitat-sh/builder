use crate::{error::Error,
            event::BuilderEvent,
            kafka::{KafkaConfig,
                    KafkaProducer}};
use async_trait::async_trait;
use habitat_core::util;
use std::{convert::TryFrom,
          fmt,
          result,
          str::FromStr};

/// Value used in configuration to define the underlying EventBus binding type to instantiate
/// at run-time.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum Provider {
    None,
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
    fn default() -> Provider { Provider::None }
}

/// Holds the configuration defining the EventBus provider bindings to load as well
/// as the specific configuration required for each message bus binding type.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct EventBusConfig {
    #[serde(with = "util::serde::string")]
    pub provider: Provider,
    pub kafka:    KafkaConfig,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        EventBusConfig { provider: Provider::default(),
                         kafka:    KafkaConfig::default(), }
    }
}

/// Creates a new Producer instance of a trait object for the selected message bus provider.
/// The Producer publishes messages onto the bus.
pub fn create_producer(config: &EventBusConfig) -> Result<Box<dyn EventBusProducer>, Error> {
    match config.provider {
        Provider::Kafka => {
            match KafkaProducer::try_from(&config.kafka.clone()) {
                Ok(producer) => {
                    info!("Kafka EventBusProducer ready to go.");
                    Ok(Box::new(producer))
                }
                Err(err) => {
                    error!("Unable to load Kafka EventBusProducer: {}", err);
                    Err(Error::EventBusError(Box::new(err)))
                }
            }
        }
        Provider::None => Err(Error::BadProvider("No EventBus provider specified!".to_string())),
    }
}

/// Trait object abstraction representing the configured EventBus Producer. The Producer is
/// responsible for publishing messages onto the message bus.
#[async_trait]
pub trait EventBusProducer {
    async fn send(&self, event: BuilderEvent);
}
