use self::DispatchStatus::*;
use crate::connection::EventPublisher;
use cloudevents::{event::Event,
                  EventBuilder,
                  EventBuilderV10};
use habitat_core::os::net::fqdn;
use serde::Serialize;
use serde_json::json;
use std::fmt;
use url::Url;
use uuid::Uuid;

lazy_static! {
    pub static ref SOURCE_URL: Url = localhost_url().expect("URL from hostname");
}

fn localhost_url() -> Result<Url, url::ParseError> {
    let host = fqdn().unwrap_or_else(|| "localhost".to_string());
    Url::parse(&format!("https://{}", host))
}

/// AffinityKey allows for guaranteed message ordering or "message affinity" when the underlying
/// message bus type supports this. For instance, with Kafka, the AffinityKey is a hash used to
/// route all messages with the same key to the same topic partition.
#[derive(Debug, Deserialize)]
pub enum AffinityKey {
    // message affinity is not needed (potentially random delivery order)
    NoAffinity,
    Key(String),
}

/// DeliveryTicket contains routing information about how to deliver the message such as the
/// destination and message affinity key.
#[derive(Debug)]
pub struct DeliveryTicket {
    // the Topic or Queue name
    pub destination:  String,
    pub affinity_key: AffinityKey,
}

/// DispatchStatus tracks the delivery status of the Event.
#[derive(Debug)]
pub enum DispatchStatus {
    Delivered,
    Undelivered(DeliveryTicket),
}

/// An "event" expressing an action occurrence and its context in Builder. BuilderEvents are routed
/// from an event producer (the source) to interested event consumers.
///
/// BuilderEvents will contain two types of information: the Event Data (the `inner` field)
/// representing the Occurrence along with Context metadata providing contextual information about
/// the Occurrence. Additionally, BuilderEvent contains an `tracking` field used to track the
/// delivery status and/or routing information for delivery of the Event message.
#[derive(Debug)]
pub struct BuilderEvent {
    inner:    Event,
    tracking: DispatchStatus,
}

/// The type of "event" Occurrence representing a BuilderEvent action that occurred and can be
/// subscribed to by interested consumers. Internally, we may use the EventType to help determine
/// which Topic or Queue a message is sent to.
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
    pub fn new<D>(event_type: EventType,
                  affinity_key: AffinityKey,
                  destination: String,
                  payload: D)
                  -> Self
        where D: Serialize
    {
        let data = json!(payload);
        // EventBuilder returns a CloudEvent which is a vendor-neutral specification for defining
        // the format of event data. See: https://github.com/cloudevents/spec/blob/v1.0/spec.md
        let event =
            EventBuilderV10::new().id(Uuid::new_v4().to_string())
                                  .ty(event_type.to_string())
                                  .source(SOURCE_URL.to_string())
                                  .data("application/json", data)
                                  .build()
                                  .expect("This should always work because we control all the \
                                           inputs");
        BuilderEvent { inner:    event,
                       tracking: Undelivered(DeliveryTicket { destination,
                                                              affinity_key }), }
    }

    // Function to return owned tuple consisting of the private fields in BuilderEvent
    // This allows us to keep the fields private in case we want to change their
    // types.
    pub fn fields(self) -> (Event, DispatchStatus) { (self.inner, self.tracking) }

    // Tells the configured EventBus to send the BuilderEvent message
    pub async fn publish(self, bus: &Option<Box<dyn EventPublisher>>) {
        // If the EventBus feature is enabled, we send the message, otherwise it is a no-op.
        if let Some(b) = bus.as_ref() {
            b.publish(self).await;
        }
    }
}

impl From<Event> for BuilderEvent {
    // Return a BuilderEvent from a consumed Event
    fn from(event: Event) -> Self {
        BuilderEvent { inner:    event,
                       tracking: Delivered, }
    }
}
