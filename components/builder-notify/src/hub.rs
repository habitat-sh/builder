use crate::webhook::Webhook;

/// A hub is a registry of hooks
#[derive(Default)]
pub struct Hub {
    hooks: Vec<Webhook>,
}

impl Hub {
    pub fn new() -> Hub { Hub { ..Default::default() } }

    /// add a hook to list of hooks
    pub fn add(&mut self, hook: Webhook) { self.hooks.push(hook); }

    /// handle hook delivery
    pub async fn handle(&self, event_data:String) {
        for hook in &self.hooks {
            let result = hook.deliver(&event_data).await;
            match result {
                Ok(_) => debug!("Successfully delivered event!"),
                Err(err) => debug!("Error {:?}", err),
            }
        }
    }
}
