use common_eventbus::{Event, EventBus};
use common_eventbus::{EventBusError, EventHandler};
use std::sync::Arc;

pub struct NoopEventBus;

impl Default for NoopEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl NoopEventBus {
    pub fn new() -> Self {
        NoopEventBus
    }
}

#[async_trait::async_trait]
impl EventBus<Event> for NoopEventBus {
    async fn subscribe(&self, _handler: Arc<dyn EventHandler<Event>>) {
        // Noop
    }

    async fn publish(&self, _event: Event) -> Result<(), EventBusError> {
        // Noop
        Ok(())
    }
}
