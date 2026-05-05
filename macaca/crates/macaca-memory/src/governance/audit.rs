use chrono::{DateTime, Utc};
use macaca_proto::{MemoryId, MemoryLayer};
use serde::{Deserialize, Serialize};

use crate::core::scope::MemoryScope;

/// High-level governance audit events.
///
/// The audit layer is intentionally descriptive rather than operational. It
/// records what happened so that operators, traces, and future compilers can
/// understand why a candidate was captured, promoted, merged, or deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryAuditEventKind {
    CapturedCandidate,
    Promoted,
    Deferred,
    Rejected,
    Merged,
    Deleted,
    PropagatedDelete,
    CompiledKnowledge,
    ArtifactGenerated,
    ConflictDetected,
}

/// One immutable governance audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAuditEvent {
    pub kind: MemoryAuditEventKind,
    pub scope: MemoryScope,
    pub memory_id: Option<MemoryId>,
    pub candidate_id: Option<String>,
    pub policy_id: Option<String>,
    pub provider_id: Option<String>,
    pub layer: Option<MemoryLayer>,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

impl MemoryAuditEvent {
    /// Create a new governance audit event with the current timestamp.
    pub fn new(kind: MemoryAuditEventKind, scope: MemoryScope, reason: impl Into<String>) -> Self {
        Self {
            kind,
            scope,
            memory_id: None,
            candidate_id: None,
            policy_id: None,
            provider_id: None,
            layer: None,
            reason: reason.into(),
            occurred_at: Utc::now(),
        }
    }

    pub fn memory_id(mut self, memory_id: MemoryId) -> Self {
        self.memory_id = Some(memory_id);
        self
    }

    pub fn candidate_id(mut self, candidate_id: impl Into<String>) -> Self {
        self.candidate_id = Some(candidate_id.into());
        self
    }

    pub fn policy_id(mut self, policy_id: impl Into<String>) -> Self {
        self.policy_id = Some(policy_id.into());
        self
    }

    pub fn provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    pub fn layer(mut self, layer: MemoryLayer) -> Self {
        self.layer = Some(layer);
        self
    }
}
