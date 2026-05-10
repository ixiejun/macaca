use chrono::{DateTime, Utc};
use macaca_proto::MemoryId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::scope::MemoryScope;

/// Tombstone that blocks deleted memories from being revived.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryTombstone {
    pub memory_id: MemoryId,
    pub scope: MemoryScope,
    pub reason: String,
    pub deleted_at: DateTime<Utc>,
    pub propagate: bool,
    pub metadata: Value,
}

impl MemoryTombstone {
    pub fn new(memory_id: MemoryId, scope: MemoryScope, reason: impl Into<String>) -> Self {
        Self {
            memory_id,
            scope,
            reason: reason.into(),
            deleted_at: Utc::now(),
            propagate: true,
            metadata: Value::Null,
        }
    }
}

/// One deletion propagation step across backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDeletePropagationStep {
    pub backend: String,
    pub attempted: bool,
    pub succeeded: bool,
    pub retryable: bool,
    pub message: String,
}

/// Aggregate deletion propagation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDeletePropagation {
    pub tombstone: MemoryTombstone,
    pub steps: Vec<MemoryDeletePropagationStep>,
}
