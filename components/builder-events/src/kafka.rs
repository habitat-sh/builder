use crate::{connection::{EventConsumer,
                         EventPublisher},
            error::Error,
            event::{AffinityKey,
                    BuilderEvent,
                    DispatchStatus::*}};
use async_trait::async_trait;
use cloudevents_sdk_rdkafka::{FutureRecordExt,
                              MessageExt,
                              MessageRecord};
use rdkafka::{config::ClientConfig,
              consumer::{Consumer,
                         StreamConsumer},
              producer::{FutureProducer,
                         FutureRecord}};
use serde::{Deserialize,
            Serialize};
use std::convert::TryFrom;
use tokio::stream::StreamExt;
use url::Url;

pub const KAFKA_DEFAULT_TOPIC_NAME: &str = "builder-events";

pub struct KafkaConsumer {
    inner: StreamConsumer,
}

pub struct KafkaPublisher {
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
                                //  .set("sasl.username", &config.api_key)
                                //  .set("sasl.password", &config.api_secret_key)
                                //  .set("sasl.mechanisms", "PLAIN")
                                //  .set("security.protocol", "PLAINTEXT")
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

impl TryFrom<&KafkaConfig> for KafkaPublisher {
    type Error = Error;

    fn try_from(config: &KafkaConfig) -> Result<Self, Error> {
        let bootstrap_list = config.bootstrap_nodes.join(",");
        match ClientConfig::new().set("bootstrap.servers", &bootstrap_list)
                                 .set("message.timeout.ms", &config.message_timeout_ms.to_string())
                                 .set("client.id", &config.client_id.as_str())
                                //  .set("sasl.username", &config.api_key)
                                //  .set("sasl.password", &config.api_secret_key)
                                //  .set("sasl.mechanisms", "PLAIN")
                                //  .set("security.protocol", "PLAINTEXT")
                                 .create()
        {
            Ok(p) => Ok(KafkaPublisher { inner: p }),
            Err(err) => {
                error!("Error initializing kafka producer: {:?}", err);
                Err(Error::EventError(Box::new(err)))
            }
        }
    }
}

// The EventConsumer trait object binding for Kafka
#[async_trait]
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
    async fn stream(&self) {
        let mut msg_stream = self.inner.start();
        while let Some(message) = msg_stream.next().await {
            match message {
                Ok(m) => {
                    match m.to_event() {
                        Ok(event) => {
                            debug!("Event {:?}", event);
                            let data: serde_json::Value = event.try_get_data().unwrap().unwrap();
                            debug!("EventData {:?}", data);
                        }
                        Err(err) => {
                            debug!("Error in EventData: {:?}", err);
                        }
                    }
                }
                Err(err) => {
                    let err_ext: Box<dyn std::error::Error> =
                        format!("Unable to extract message: {}", err).into();
                    debug!("Error in EventData: {:?}", err_ext);
                }
            };
        }
    }
}

// The EventPublisher trait object binding for Kafka
#[async_trait]
#[allow(clippy::wildcard_in_or_patterns)]
impl EventPublisher for KafkaPublisher {
    /// Publish messages onto the topic
    async fn publish(&self, builder_event: BuilderEvent) {
        let (event, tracking) = builder_event.fields();
        let message_record =
            MessageRecord::from_event(event).expect("error while serializing the event");
        match tracking {
            Undelivered(tag) => {
                let future_record = {
                    let r = FutureRecord::to(&tag.destination).message_record(&message_record);
                    if let AffinityKey::Key(key) = &tag.affinity_key {
                        r.key(key)
                    } else {
                        r
                    }
                };
                match self.inner.send(future_record, 0).await {
                    Err(err) => {
                        error!("KafkaPublisher failed to send message to a Broker: {:?}",
                               err)
                    }
                    Ok(_) => {
                        trace!("KafkaPublisher published event to topic: {}",
                               tag.destination)
                    }
                };
            }
            Delivered => error!("KafkaPublisher will not publish an already delivered Event."),
        }
    }
}
