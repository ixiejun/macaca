//! Task service event sink (Observer pattern).
//!
//! Structured events flow to SSE bridges, audit logs, or in-memory test collectors.

use std::sync::RwLock;

use async_trait::async_trait;

use crate::events::TaskServiceEvent;

/// Sink boundary for task service events.
#[async_trait]
pub trait TaskServiceEventSink: Send + Sync {
    /// Publish one structured event.
    async fn publish(&self, event: TaskServiceEvent);
}

/// In-memory event sink used by tests and local runtime wiring.
#[derive(Default)]
pub struct InMemoryTaskServiceEventSink {
    events: RwLock<Vec<TaskServiceEvent>>,
}

impl InMemoryTaskServiceEventSink {
    /// Create an empty in-memory sink for tests and local wiring.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a cloned list of published events.
    pub fn snapshot(&self) -> Vec<TaskServiceEvent> {
        self.events.read().unwrap().clone()
    }
}

#[async_trait]
impl TaskServiceEventSink for InMemoryTaskServiceEventSink {
    async fn publish(&self, event: TaskServiceEvent) {
        self.events.write().unwrap().push(event);
    }
}
