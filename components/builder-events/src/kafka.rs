use std::convert::TryFrom;

use crate::{cloud_events::CloudEvent,
            connection::{EventBusCfg,
                         EventBusConn}};
use cloudevents::AttributesReader;
use cloudevents_sdk_rdkafka::{FutureRecordExt,
                              MessageRecord};

use rdkafka::{config::ClientConfig,
              error::KafkaError,
              producer::{FutureProducer,
                         FutureRecord}};

pub const KAFKA_DEFAULT_TOPIC_NAME: &str = "bldr_events";

pub struct KafkaProducer {
    pub inner: FutureProducer,
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

#[allow(clippy::wildcard_in_or_patterns)]
pub async fn produce(msgkey: i64, event: CloudEvent, conn: &EventBusConn) {
    let topic = match event.inner.get_type() {
        "PackageChannelMotion" | _ => KAFKA_DEFAULT_TOPIC_NAME,
    };
    let message_record =
        MessageRecord::from_event(event.inner).expect("error while serializing the event");

    match conn.kafka.as_ref() {
        Some(c) => {
            if let Err(err) = c.inner
                               .send(FutureRecord::to(topic).message_record(&message_record)
                                                            .key(&format!("Key {}", msgkey)),
                                     0)
                               .await
            {
                error!("Event producer failed to send message to event bus: {:?}",
                       err)
            };
        }
        None => {
            error!("Uninitialized eventbus producer connection handle!");
        }
    }
}
