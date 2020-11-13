use crate::{connection::EventBusProducer,
            event::{BuilderEvent,
                    EventType,
                    RoutingKey}};
use async_trait::async_trait;
use cloudevents::AttributesReader;
use cloudevents_sdk_rdkafka::{FutureRecordExt,
                              MessageRecord};
use std::{convert::TryFrom,
          time::Duration};
use url::Url;

use rdkafka::{config::ClientConfig,
              error::KafkaError,
              producer::{FutureProducer,
                         FutureRecord}};

pub const KAFKA_DEFAULT_TOPIC_NAME: &str = "builder_events";

pub struct KafkaProducer {
    inner: FutureProducer,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct KafkaConfig {
    pub api_key:                String,
    pub api_secret_key:         String,
    pub bootstrap_nodes:        Vec<String>,
    pub client_id:              Url,
    #[serde(with = "deserialize_into_duration")]
    pub connection_retry_delay: Duration,
    pub message_timeout_ms:     u64,
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

impl Default for KafkaConfig {
    // Reasonable defaults that allow a new KafkaProducer to be created.
    // We need to implement Default due to the Deserialize trait on the struct in order to read
    // from the toml config.
    fn default() -> Self {
        KafkaConfig { api_key:                "api key".to_string(),
                      api_secret_key:         "secret key".to_string(),
                      bootstrap_nodes:        vec!["localhost:9092".to_string()],
                      client_id:              "http://localhost".parse().expect("CLIENT_ID URL"),
                      connection_retry_delay: Duration::from_secs(3),
                      message_timeout_ms:     3000, }
    }
}

impl TryFrom<&KafkaConfig> for KafkaProducer {
    type Error = KafkaError;

    fn try_from(config: &KafkaConfig) -> Result<Self, KafkaError> {
        let bootstrap_list = config.bootstrap_nodes.join(",");
        match ClientConfig::new().set("bootstrap.servers", &bootstrap_list)
                                 .set("message.timeout.ms", &config.message_timeout_ms.to_string())
                                 .set("client.id", &config.client_id.as_str())
                                 .set("sasl.username", &config.api_key)
                                 .set("sasl.password", &config.api_secret_key)
                                 .set("security.protocol", "SASL_SSL")
                                 .set("sasl.mechanisms", "PLAIN")
                                 .create()
        {
            Ok(p) => Ok(KafkaProducer { inner: p }),
            Err(err) => {
                error!("Error initializing kafka producer: {:?}", err);
                Err(err)
            }
        }
    }
}

// The EventBusProducer trait object binding for Kafka
#[async_trait]
#[allow(clippy::wildcard_in_or_patterns)]
impl EventBusProducer for KafkaProducer {
    async fn send(&self, builder_event: BuilderEvent) {
        let (event, routing_key) = builder_event.fields();
        let topic = match event.get_type() {
            EventType::PACKAGECHANNELMOTION | _ => KAFKA_DEFAULT_TOPIC_NAME,
        };
        let message_record =
            MessageRecord::from_event(event).expect("error while serializing the event");
        let future_record = {
            let r = FutureRecord::to(topic).message_record(&message_record);
            if let RoutingKey::Key(key) = &routing_key {
                r.key(key)
            } else {
                r
            }
        };
        if let Err(err) = self.inner.send(future_record, 0).await {
            error!("Event producer failed to send message to event bus: {:?}",
                   err)
        };
    }
}
