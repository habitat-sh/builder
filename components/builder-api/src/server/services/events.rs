use std::convert::TryFrom;

use crate::config::KafkaCfg;

use cloudevents::Event;
use cloudevents_sdk_rdkafka::{FutureRecordExt,
                              MessageRecord};

use rdkafka::{config::ClientConfig,
              error::KafkaError,
              producer::{FutureProducer,
                         FutureRecord}};

pub const KAFKA_DEFAULT_TOPIC_NAME: &str = "bldr_event_init";

pub struct KafkaProducer(pub FutureProducer);

impl TryFrom<&KafkaCfg> for KafkaProducer {
    type Error = KafkaError;

    fn try_from(config: &KafkaCfg) -> Result<Self, KafkaError> {
        let bootstrap_list = config.bootstrap_nodes.join(",");
        match ClientConfig::new().set("bootstrap.servers", &bootstrap_list)
                                 .set("message.timeout.ms", &config.message_timeout)
                                 .set("client.id", &config.client_id)
                                 .set("sasl.username", &config.api_key)
                                 .set("sasl.password", &config.api_secret_key)
                                 .set("security.protocol", "SASL_SSL")
                                 .set("sasl.mechanisms", "PLAIN")
                                 .create()
        {
            Ok(iprodr) => Ok(KafkaProducer(iprodr)),
            Err(err) => {
                error!("Error initializing kafka producer, will retry: {:?}", err);
                Err(err)
            }
        }
    }
}

pub async fn produce(msgkey: i64, event: Event, conn: &FutureProducer, topic_name: &str) {
    let message_record =
        MessageRecord::from_event(event).expect("error while serializing the event");

    if let Err(err) = conn.send(FutureRecord::to(topic_name).message_record(&message_record)
                                                            .key(&format!("Key {}", msgkey)),
                                0)
                          .await
    {
        error!("Event producer failed to send message to event bus: {:?}",
               err)
    };
}
