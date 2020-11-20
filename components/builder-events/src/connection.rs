use crate::{error::Error,
            event::BuilderEvent,
            kafka::{KafkaConfig,
                    KafkaConsumer,
                    KafkaProducer}};
use async_trait::async_trait;
use habitat_core::util;
use serde::{Deserialize,
            Serialize};
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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct EventBusConfig {
    #[serde(with = "util::serde::string")]
    pub provider: Provider,
    #[serde(flatten)]
    pub kafka:    KafkaConfig,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        EventBusConfig { provider: Provider::default(),
                         kafka:    KafkaConfig::default(), }
    }
}

/// Creates a new Consumer instance of a trait object for the selected message bus provider.
/// The Consumer consumes messages from the bus.
pub fn create_consumer(config: &EventBusConfig) -> Result<Box<dyn EventBusConsumer>, Error> {
    match config.provider {
        Provider::Kafka => {
            match KafkaConsumer::try_from(&config.kafka.clone()) {
                Ok(consumer) => {
                    info!("Kafka EventBusConsumer ready to go.");
                    Ok(Box::new(consumer))
                }
                Err(err) => {
                    error!("Unable to load Kafka EventBusConsumer: {}", err);
                    Err(Error::EventBusError(Box::new(err)))
                }
            }
        }
        Provider::None => {
            let msg = "No EventBus provider specified!";
            error!("{}", msg);
            Err(Error::BadProvider(msg.to_string()))
        }
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
        Provider::None => {
            let msg = "No EventBus provider specified!";
            error!("{}", msg);
            Err(Error::BadProvider(msg.to_string()))
        }
    }
}

/// Trait object abstraction representing the configured EventBus Producer. The Producer is
/// responsible for publishing messages onto the message bus.
pub trait EventBusConsumer {
    fn subscribe(&self, queues: Vec<&str>) -> Result<(), Error>;
    fn poll(&self) -> Option<String>;
}

/// Trait object abstraction representing the configured EventBus Consumer. The Consumer is
/// responsible for publishing messages onto the message bus.
#[async_trait]
pub trait EventBusProducer {
    async fn publish(&self, event: BuilderEvent);
}
