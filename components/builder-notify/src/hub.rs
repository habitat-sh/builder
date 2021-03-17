use crate::hook::Hook;
use cloudevents::event::Event;

/// A hub is a registry of hooks
#[derive(Debug, Default)]
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
    pub fn handle(&self, event: &Event) {
        debug!("Hub:: Delivering Event {:?}", event);
        for hook in &self.hooks {
            hook.deliver(&event);
        }
    }
}
