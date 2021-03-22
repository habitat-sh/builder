use crate::hook::Hook;

/// A hub is a registry of hooks
#[derive(Default)]
pub struct Hub {
    hooks: Vec<Box<dyn Hook>>,
}

impl Hub {
    pub fn new() -> Hub { Hub { ..Default::default() } }

    /// add a hook to list of hooks
    pub fn add<H>(&mut self, hook: H)
        where H: Hook + 'static
    {
        self.hooks.push(Box::new(hook));
    }

    /// handle hook delivery
    pub async fn handle(&self, event_data: &str) {
        for hook in &self.hooks {
            let result = hook.deliver(&event_data).await;
            match result {
                Ok(_) => debug!("Successfully delivered event!"),
                Err(err) => debug!("Error {:?}", err),
            }
        }
    }
}
