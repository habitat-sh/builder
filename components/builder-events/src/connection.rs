use crate::kafka::KafkaProducer;
use std::{convert::From,
          time::Duration};

#[derive(Clone, Debug, Deserialize)]
#[non_exhaustive]
pub enum Provider {
    Kafka,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct EventBusCfg {
    pub api_key:                String,
    pub api_secret_key:         String,
    pub bootstrap_nodes:        Vec<String>,
    pub client_id:              String,
    #[serde(with = "deserialize_into_duration")]
    pub connection_retry_delay: Duration,
    pub message_timeout_ms:     u64,
    pub provider:               Provider,
}

impl Default for EventBusCfg {
    fn default() -> Self {
        EventBusCfg { api_key:                String::from("CHANGEME"),
                      api_secret_key:         String::from("CHANGEMETOO"),
                      bootstrap_nodes:        vec![String::from("localhost:9092")],
                      client_id:              String::from("http://localhost"),
                      connection_retry_delay: Duration::from_secs(3),
                      message_timeout_ms:     3000,
                      provider:               Provider::Kafka, }
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

pub struct EventBusConn {
    pub kafka:           Option<KafkaProducer>,
    pub provider_in_use: Provider,
}

impl From<KafkaProducer> for EventBusConn {
    fn from(producer: KafkaProducer) -> Self {
        EventBusConn { kafka:           Some(producer),
                       provider_in_use: Provider::Kafka, }
    }
}
