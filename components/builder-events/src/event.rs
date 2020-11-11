use crate::error::Error;
use cloudevents::{event::Event,
                  EventBuilder,
                  EventBuilderV10};
use habitat_core::os::net::fqdn;
use std::fmt;
use url::Url;
use uuid::Uuid;

#[derive(Clone)]
pub struct BuilderEvent {
    inner:       Event,
    routing_key: Option<String>,
}

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

impl BuilderEvent {
    pub fn new(event_type: EventType,
               routing_key: Option<String>,
               payload: serde_json::Value)
               -> Result<Self, Error> {
        let host = fqdn().unwrap_or_else(|| "localhost".to_string());
        let source_url = Url::parse(&format!("https://{}", host))?;

        let event = EventBuilderV10::new().id(format!("{}", Uuid::new_v4()))
                                          .ty(format!("{}", event_type))
                                          .source(source_url)
                                          .data("application/json", payload)
                                          .build()?;
        Ok(BuilderEvent { inner: event,
                          routing_key })
    }

    pub fn into_inner(self) -> Event { self.inner }

    pub fn routing_key(self) -> Option<String> { self.routing_key }
}

#[macro_export]
macro_rules! publish_event {
    ($opt_bus:expr, $event_type:expr, $json:expr) => {
        publish_event!($opt_bus, $event_type, $json, with_routing_key: None)
    };
    ($opt_bus:expr, $event_type:expr, $json:expr,with_routing_key: $routing_key:expr) => {
        if let Some(bus) = $opt_bus.as_ref() {
            match BuilderEvent::new($event_type, $routing_key, $json) {
                Ok(event) => bus.send(event).await,
                Err(err) => {
                    debug!("Failed to create EventBus event for {}: {}",
                           $event_type, err)
                }
            }
        }
    };
}
