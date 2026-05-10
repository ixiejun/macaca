use chrono::{DateTime, Utc};
use macaca_proto::MemoryId;
use serde::{Deserialize, Serialize};

use super::scope::MemoryScope;

/// High-level lifecycle phases a provider may emit for a memory entry.
///
/// The enum is intentionally broader than the current builtin adapters so
/// future providers can report governance, promotion, flush, or compaction
/// behavior through one common DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLifecycleEventKind {
    Created,
    Updated,
    Deleted,
    Promoted,
    Flushed,
    Compacted,
}

/// Immutable event record describing a change in memory state.
///
/// Lifecycle events are designed for observability and future hooks rather than
/// being part of the write/search hot path. The scope is embedded directly so
/// downstream consumers can reason about which isolation domain changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLifecycleEvent {
    pub kind: MemoryLifecycleEventKind,
    pub scope: MemoryScope,
    pub memory_id: Option<MemoryId>,
    pub occurred_at: DateTime<Utc>,
}

impl MemoryLifecycleEvent {
    /// Create a lifecycle event stamped with the current UTC time.
    pub fn new(
        kind: MemoryLifecycleEventKind,
        scope: MemoryScope,
        memory_id: Option<MemoryId>,
    ) -> Self {
        Self {
            kind,
            scope,
            memory_id,
            occurred_at: Utc::now(),
        }
    }
}
