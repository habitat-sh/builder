use crate::{error::Error,
            event::BuilderEvent,
            kafka::{KafkaConfig,
                    KafkaConsumer,
                    KafkaPublisher}};
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
pub struct EventConfig {
    #[serde(with = "util::serde::string")]
    pub provider: Provider,
    #[serde(flatten)]
    pub kafka:    KafkaConfig,
}

impl Default for EventConfig {
    fn default() -> Self {
        EventConfig { provider: Provider::default(),
                      kafka:    KafkaConfig::default(), }
    }
}

/// Creates a new Consumer instance of a trait object for the selected message bus provider.
/// The Consumer consumes messages from the bus.
pub fn create_consumer(config: &EventConfig) -> Result<Box<dyn EventConsumer>, Error> {
    match config.provider {
        Provider::Kafka => {
            match KafkaConsumer::try_from(&config.kafka.clone()) {
                Ok(consumer) => {
                    info!("Kafka EventConsumer ready to go.");
                    Ok(Box::new(consumer))
                }
                Err(err) => {
                    error!("Unable to load Kafka EventConsumer: {}", err);
                    Err(Error::EventError(Box::new(err)))
                }
            }
        }
        Provider::None => {
            let msg = "No EventBus Provider specified!";
            error!("{}", msg);
            Err(Error::BadProvider(msg.to_string()))
        }
    }
}

/// Creates a new Producer instance of a trait object for the selected message bus provider.
/// The Producer publishes messages onto the bus.
pub fn create_producer(config: &EventConfig) -> Result<Box<dyn EventPublisher>, Error> {
    match config.provider {
        Provider::Kafka => {
            match KafkaPublisher::try_from(&config.kafka.clone()) {
                Ok(producer) => {
                    info!("Kafka EventPublisher ready to go.");
                    Ok(Box::new(producer))
                }
                Err(err) => {
                    error!("Unable to load Kafka EventPublisher: {}", err);
                    Err(Error::EventError(Box::new(err)))
                }
            }
        }
        Provider::None => {
            let msg = "No EventBus Provider specified!";
            error!("{}", msg);
            Err(Error::BadProvider(msg.to_string()))
        }
    }
}

/// Trait abstraction representing the configured Event Consumer. The Consumer is
/// responsible for reading messages off of the message bus.
#[async_trait]
pub trait EventConsumer {
    /// Subscribe to list of queues or topics
    fn subscribe(&self, queues: &[&str]) -> Result<(), Error>;
    /// Stream the topic(s) for new messages. When a message exists, its payload will be returned.
    /// This is a synchronous call.
    async fn stream(&self);
}

/// Trait abstraction representing the configured Event Publisher. The Publisher is
/// responsible for publishing messages onto the message bus.
#[async_trait]
pub trait EventPublisher {
    /// Send events onto the configured message bus
    async fn publish(&self, event: BuilderEvent);
}
