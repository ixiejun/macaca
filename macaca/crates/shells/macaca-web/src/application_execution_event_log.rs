//! Application execution EventLog observer port.
//!
//! Application execution stream routes need durable replay rows and append
//! notifications, but they should not depend on the concrete host persistence
//! type. This module keeps a small Observer/Adapter port around the host-owned
//! EventLog so HTTP/SSE code can stay focused on transport framing.

use async_trait::async_trait;
use macaca_proto::{EventEntry, EventLogQuery};
use tokio::sync::broadcast;

/// Provider-neutral read/subscribe port for persisted application events.
#[async_trait]
pub(crate) trait ApplicationExecutionEventLog: Send + Sync {
    /// Query durable EventLog rows through the service-owned index.
    async fn query_indexed(&self, query: EventLogQuery) -> Vec<EventEntry>;

    /// Subscribe to append notifications keyed by session id and latest sequence.
    fn subscribe(&self) -> broadcast::Receiver<(String, u64)>;
}
