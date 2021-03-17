use cloudevents::event::Event;
use std::fmt::Debug;

/// Handles hook deliveries
pub trait Hook: Debug {
    /// Implementations are expected to deliver
    fn deliver(&self, event: &Event);
}

/// A Webhook
#[derive(Debug, Default)]
pub struct Webhook {
    pub endpoint: String,
}

impl Webhook {
    pub fn new(endpoint: String) -> Webhook { Webhook { endpoint } }
}

impl Hook for Webhook {
    fn deliver(&self, event: &Event) {
        debug!("Hook:: Delivering Event {:?}", event);
    }
}
