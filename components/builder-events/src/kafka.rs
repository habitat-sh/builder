use crate::{connection::{EventBusCfg,
                         EventBusProvider},
            event::{BuilderEvent,
                    EventType}};
use async_trait::async_trait;
use cloudevents::AttributesReader;
use cloudevents_sdk_rdkafka::{FutureRecordExt,
                              MessageRecord};
use std::convert::TryFrom;

use rdkafka::{config::ClientConfig,
              error::KafkaError,
              producer::{FutureProducer,
                         FutureRecord}};

pub const KAFKA_DEFAULT_TOPIC_NAME: &str = "builder_events";

pub struct KafkaProducer {
    inner: FutureProducer,
}

impl KafkaProducer {
    pub fn into_inner(self) -> FutureProducer { self.inner }
}

impl TryFrom<&EventBusCfg> for KafkaProducer {
    type Error = KafkaError;

    fn try_from(config: &EventBusCfg) -> Result<Self, KafkaError> {
        let bootstrap_list = config.bootstrap_nodes.join(",");
        match ClientConfig::new().set("bootstrap.servers", &bootstrap_list)
                                 .set("message.timeout.ms", &config.message_timeout_ms.to_string())
                                 .set("client.id", &config.client_id)
                                 .set("sasl.username", &config.api_key)
                                 .set("sasl.password", &config.api_secret_key)
                                 .set("security.protocol", "SASL_SSL")
                                 .set("sasl.mechanisms", "PLAIN")
                                 .create()
        {
            Ok(p) => Ok(KafkaProducer { inner: p }),
            Err(err) => {
                error!("Error initializing kafka producer, will retry: {:?}", err);
                Err(err)
            }
        }
    }
}

#[async_trait]
#[allow(clippy::wildcard_in_or_patterns)]
impl EventBusProvider for KafkaProducer {
    async fn publish(&self, event: BuilderEvent) {
        let topic = match event.clone().into_inner().get_type() {
            EventType::PACKAGECHANNELMOTION | _ => KAFKA_DEFAULT_TOPIC_NAME,
        };
        let routing_key = event.clone().routing_key();
        let message_record = MessageRecord::from_event(event.into_inner()).expect("error while \
                                                                                   serializing \
                                                                                   the event");

        if let Err(err) = self.inner
                              .send(FutureRecord::to(topic).message_record(&message_record)
                                                           .key(&format!("Key {}",
                                                                         routing_key
                                                                           .unwrap_or_default())),
                                    0)
                              .await
        {
            error!("Event producer failed to send message to event bus: {:?}",
                   err)
        };
    }
}
