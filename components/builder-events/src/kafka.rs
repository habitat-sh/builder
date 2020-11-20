use crate::{connection::{EventConsumer,
                         EventPublisher},
            error::Error,
            event::{BuilderEvent,
                    EventType,
                    RoutingKey}};
use async_trait::async_trait;
use cloudevents::AttributesReader;
use cloudevents_sdk_rdkafka::{FutureRecordExt,
                              MessageRecord};
use serde::{Deserialize,
            Serialize};
use std::{convert::TryFrom,
          time::Duration};
use url::Url;

use rdkafka::{config::ClientConfig,
              consumer::{BaseConsumer,
                         Consumer},
              message::{Message,
                        OwnedMessage},
              producer::{FutureProducer,
                         FutureRecord}};

pub const KAFKA_DEFAULT_TOPIC_NAME: &str = "builder_events";

pub struct KafkaConsumer {
    inner: BaseConsumer,
}

pub struct KafkaProducer {
    inner: FutureProducer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct KafkaConfig {
    pub api_key:            String,
    pub api_secret_key:     String,
    pub auto_commit:        bool,
    pub bootstrap_nodes:    Vec<String>,
    pub client_id:          Url,
    pub group_id:           String,
    pub message_timeout_ms: u64,
    pub partition_eof:      bool,
    pub session_timeout_ms: u64,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        KafkaConfig { api_key:            "api key".to_string(),
                      api_secret_key:     "secret key".to_string(),
                      auto_commit:        true,
                      bootstrap_nodes:    vec!["localhost:9092".to_string()],
                      client_id:          "http://localhost".parse().expect("CLIENT_ID URL"),
                      group_id:           "bldr_consumer group".to_string(),
                      message_timeout_ms: 6000,
                      partition_eof:      false,
                      session_timeout_ms: 6000, }
    }
}

impl TryFrom<&KafkaConfig> for KafkaConsumer {
    type Error = Error;

    fn try_from(config: &KafkaConfig) -> Result<Self, Error> {
        let bootstrap_list = config.bootstrap_nodes.join(",");
        match ClientConfig::new().set("group.id", &config.group_id)
                                 .set("bootstrap.servers", &bootstrap_list)
                                 .set("enable.partition.eof", &config.partition_eof.to_string())
                                 .set("sasl.username", &config.api_key)
                                 .set("sasl.password", &config.api_secret_key)
                                 .set("sasl.mechanisms", "PLAIN")
                                 .set("security.protocol", "SASL_SSL")
                                 .set("session.timeout.ms", &config.session_timeout_ms.to_string())
                                 .set("enable.auto.commit", "true")
                                 .set("auto.offset.reset", "earliest")
                                 .create()
        {
            Ok(c) => Ok(KafkaConsumer { inner: c }),
            Err(err) => {
                error!("Error initializing kafka consumer: {:?}", err);
                Err(Error::EventError(Box::new(err)))
            }
        }
    }
}

impl TryFrom<&KafkaConfig> for KafkaProducer {
    type Error = Error;

    fn try_from(config: &KafkaConfig) -> Result<Self, Error> {
        let bootstrap_list = config.bootstrap_nodes.join(",");
        match ClientConfig::new().set("bootstrap.servers", &bootstrap_list)
                                 .set("message.timeout.ms", &config.message_timeout_ms.to_string())
                                 .set("client.id", &config.client_id.as_str())
                                 .set("sasl.username", &config.api_key)
                                 .set("sasl.password", &config.api_secret_key)
                                 .set("sasl.mechanisms", "PLAIN")
                                 .set("security.protocol", "SASL_SSL")
                                 .create()
        {
            Ok(p) => Ok(KafkaProducer { inner: p }),
            Err(err) => {
                error!("Error initializing kafka producer: {:?}", err);
                Err(Error::EventError(Box::new(err)))
            }
        }
    }
}

fn extract_message_payload(msg: &OwnedMessage) -> String {
    match msg.payload_view::<str>() {
        Some(Ok(payload)) => payload.to_string(),
        Some(Err(e)) => {
            let msg = "Message payload is not a string".to_owned();
            error!("{}: {}", msg, e);
            msg
        }
        None => "No payload".to_owned(),
    }
}

// The EventConsumer trait object binding for Kafka
#[allow(clippy::wildcard_in_or_patterns)]
impl EventConsumer for KafkaConsumer {
    /// Subscribe to a vec of topics
    fn subscribe(&self, topic_names: &[&str]) -> Result<(), Error> {
        info!("KafkaConsumer subscribing to {:?}", topic_names);
        self.inner
            .subscribe(topic_names)
            .map_err(|e| Error::EventError(Box::new(e)))
    }

    /// Poll for a new message
    fn poll(&self) -> Option<String> {
        if let Some(polled) = self.inner.poll(Duration::from_secs(0)) {
            match polled {
                Ok(borrowed_message) => {
                    let owned_message = borrowed_message.detach();
                    Some(extract_message_payload(&owned_message))
                }
                Err(err) => {
                    error!("Unable to extract borrowed message: {}", err);
                    None
                }
            }
        } else {
            None
        }
    }
}

// The EventPublisher trait object binding for Kafka
#[async_trait]
#[allow(clippy::wildcard_in_or_patterns)]
impl EventPublisher for KafkaProducer {
    /// Publish messages onto the topic
    async fn publish(&self, builder_event: BuilderEvent) {
        trace!("KafkaProducer publishing event {:?}", builder_event);
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
