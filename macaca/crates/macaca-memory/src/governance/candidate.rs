use chrono::{DateTime, Utc};
use macaca_proto::{MemoryId, MemoryLayer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::scope::MemoryScope;

/// Source classes for a captured memory candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateSource {
    AutoCapture,
    UserExplicit,
    ToolObservation,
    DelegateObservation,
    AgentSummary,
    RecoveryReplay,
}

/// A memory candidate is not yet committed long-term memory.
///
/// It holds enough provenance to be promoted later without losing context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub id: String,
    pub scope: MemoryScope,
    pub source: CandidateSource,
    pub content: String,
    pub layer: MemoryLayer,
    pub metadata: Value,
    pub recurrence_count: u32,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub committed_memory_id: Option<MemoryId>,
    pub notes: Option<String>,
}

impl MemoryCandidate {
    /// Build a fresh candidate from a scoped observation.
    pub fn new(
        id: impl Into<String>,
        scope: MemoryScope,
        source: CandidateSource,
        content: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            scope,
            source,
            content: content.into(),
            layer: MemoryLayer::Session,
            metadata: Value::Null,
            recurrence_count: 1,
            confidence: 0.5,
            created_at: now,
            last_seen_at: now,
            committed_memory_id: None,
            notes: None,
        }
    }

    pub fn layer(mut self, layer: MemoryLayer) -> Self {
        self.layer = layer;
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn recurrence_count(mut self, recurrence_count: u32) -> Self {
        self.recurrence_count = recurrence_count.max(1);
        self
    }

    pub fn notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Decision attached to a candidate after policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateDecision {
    Promote,
    Defer,
    Reject,
    Merge,
}

/// Replaceable candidate store for governance events.
pub trait MemoryCandidateStore: Send + Sync {
    fn capture(&mut self, candidate: MemoryCandidate) -> String;
    fn list(&self, scope: Option<&MemoryScope>) -> Vec<MemoryCandidate>;
}

/// A trivial in-memory candidate store used by tests and default wiring.
#[derive(Debug, Default)]
pub struct MemoryCandidateCapture {
    candidates: Vec<MemoryCandidate>,
}

impl MemoryCandidateCapture {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MemoryCandidateStore for MemoryCandidateCapture {
    fn capture(&mut self, candidate: MemoryCandidate) -> String {
        let id = candidate.id.clone();
        self.candidates.push(candidate);
        id
    }

    fn list(&self, scope: Option<&MemoryScope>) -> Vec<MemoryCandidate> {
        self.candidates
            .iter()
            .filter(|candidate| scope.is_none_or(|scope| &candidate.scope == scope))
            .cloned()
            .collect()
    }
}
