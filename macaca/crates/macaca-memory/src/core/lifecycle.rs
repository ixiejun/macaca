use chrono::{DateTime, Utc};
use macaca_proto::MemoryId;
use serde::{Deserialize, Serialize};

use super::scope::MemoryScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLifecycleEventKind {
    Created,
    Updated,
    Deleted,
    Promoted,
    Flushed,
    Compacted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLifecycleEvent {
    pub kind: MemoryLifecycleEventKind,
    pub scope: MemoryScope,
    pub memory_id: Option<MemoryId>,
    pub occurred_at: DateTime<Utc>,
}

impl MemoryLifecycleEvent {
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
