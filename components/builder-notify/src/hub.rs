use crate::webhook::Webhook;
use futures::{prelude::*,
              stream::FuturesUnordered};

/// A hub is a registry of hooks
#[derive(Clone, Default)]
pub struct Hub {
    hooks: Vec<Webhook>,
}

impl Hub {
    pub fn new() -> Hub { Hub { ..Default::default() } }

    /// add a hook to list of hooks
    pub fn add(&mut self, hook: Webhook) { self.hooks.push(hook); }

    /// handle hook delivery
    pub async fn handle(&self, event_data: String) {
        let hooks = self.hooks.clone();
        let futures = hooks.iter()
                           .map(move |hook| {
                               let data = event_data.clone();
                               hook.deliver(data)
                           })
                           .collect::<FuturesUnordered<_>>();

        let results = futures.collect::<Vec<_>>().await;
        for result in results {
            match result {
                Ok(_) => debug!("Delivery Success"),
                Err(e) => debug!("Delivery Error: {}", e),
            }
        }
    }
}
