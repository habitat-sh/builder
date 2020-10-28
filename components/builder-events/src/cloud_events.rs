use crate::{connection::{EventBusConn,
                         Provider},
            error::Error,
            kafka::produce};
use cloudevents::{event::Event,
                  EventBuilder,
                  EventBuilderV10};
use habitat_core::os::net::fqdn;
use std::fmt;
use url::Url;
use uuid::Uuid;

pub struct CloudEvent(pub Event);

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub enum EventType {
    PackageChannelMotion,
}

impl EventType {
    pub const PACKAGECHANNELMOTION: &'static str = "PackageChannelMotion";
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match *self {
            EventType::PackageChannelMotion => EventType::PACKAGECHANNELMOTION,
        };
        write!(f, "{}", value)
    }
}

impl CloudEvent {
    pub fn new(event_type: EventType, json_data: serde_json::Value) -> Result<Self, Error> {
        let host = fqdn().unwrap_or_else(|| "localhost".to_string());
        let source_url = Url::parse(&format!("https://{}", host))?;

        let event = EventBuilderV10::new().id(format!("{}", Uuid::new_v4()))
                                          .ty(format!("{}", event_type))
                                          .source(source_url)
                                          .data("application/json", json_data)
                                          .build()?;
        Ok(CloudEvent(event))
    }

    pub async fn maybe_emit(event_bus: &Option<EventBusConn>,
                            event_key: i64,
                            event_type: EventType,
                            event_body: serde_json::Value) {
        // We check if the EventBus feature is enabled and a Some variant
        // If so, we use the underlying connection to emit an event. No-op otherwise.
        if event_bus.as_ref().is_some() {
            let bus_conn = event_bus.as_ref().expect("EventBus Connection");
            match bus_conn.provider_in_use {
                Provider::Kafka => {
                    match CloudEvent::new(event_type, event_body) {
                        Ok(cloudevent) => produce(event_key, cloudevent, bus_conn).await,
                        Err(err) => {
                            error!("Could not generate cloudevent from promotion event: {:?}",
                                   err)
                        }
                    };
                }
            }
        }
    }
}
