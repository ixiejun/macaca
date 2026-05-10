use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry};
use tokio::sync::RwLock;

use crate::core::scope::MemoryScope;

use super::audit::{MemoryAuditEvent, MemoryAuditEventKind};
use super::candidate::{CandidateSource, MemoryCandidate};

/// Durable journal boundary for governance audit events.
#[async_trait]
pub trait MemoryGovernanceJournal: Send + Sync {
    async fn append_audit(&self, event: MemoryAuditEvent) -> MacacaResult<()>;
    async fn list_audits(
        &self,
        scope: &MemoryScope,
        limit: usize,
    ) -> MacacaResult<Vec<MemoryAuditEvent>>;
}

/// In-memory journal used by default tests and local runtime wiring.
#[derive(Debug, Default)]
pub struct InMemoryGovernanceJournal {
    audits: RwLock<Vec<MemoryAuditEvent>>,
}

impl InMemoryGovernanceJournal {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MemoryGovernanceJournal for InMemoryGovernanceJournal {
    async fn append_audit(&self, event: MemoryAuditEvent) -> MacacaResult<()> {
        self.audits.write().await.push(event);
        Ok(())
    }

    async fn list_audits(
        &self,
        scope: &MemoryScope,
        limit: usize,
    ) -> MacacaResult<Vec<MemoryAuditEvent>> {
        let mut rows: Vec<_> = self
            .audits
            .read()
            .await
            .iter()
            .filter(|event| &event.scope == scope)
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at));
        rows.truncate(limit);
        Ok(rows)
    }
}

/// Policy seam for automatic candidate capture.
pub trait MemoryCandidateCapturePolicy: Send + Sync {
    fn should_capture(&self, source: &CandidateSource, content: &str) -> bool;
}

/// Conservative default: capture explicit memory and agent summaries only.
#[derive(Debug, Default)]
pub struct DefaultMemoryCandidateCapturePolicy;

impl MemoryCandidateCapturePolicy for DefaultMemoryCandidateCapturePolicy {
    fn should_capture(&self, source: &CandidateSource, content: &str) -> bool {
        if content.trim().is_empty() {
            return false;
        }
        matches!(
            source,
            CandidateSource::UserExplicit | CandidateSource::AgentSummary
        )
    }
}

/// Runtime seam for compaction/dreaming.
#[async_trait]
pub trait MemoryCompactionStrategy: Send + Sync {
    async fn compact(
        &self,
        scope: MemoryScope,
        entries: Vec<MemoryEntry>,
    ) -> MacacaResult<MemoryCompactionResult>;
}

#[derive(Debug, Clone)]
pub struct MemoryCompactionResult {
    pub candidates: Vec<MemoryCandidate>,
    pub diagnostics: Vec<String>,
}

/// Truthful disabled compaction strategy.
#[derive(Debug, Default)]
pub struct DisabledMemoryCompactionStrategy;

#[async_trait]
impl MemoryCompactionStrategy for DisabledMemoryCompactionStrategy {
    async fn compact(
        &self,
        _scope: MemoryScope,
        _entries: Vec<MemoryEntry>,
    ) -> MacacaResult<MemoryCompactionResult> {
        Ok(MemoryCompactionResult {
            candidates: Vec::new(),
            diagnostics: vec!["compaction_disabled".into()],
        })
    }
}

/// Small helper used by runtime wiring to append capture audit events.
pub async fn audit_candidate_capture<J>(
    journal: &J,
    candidate: &MemoryCandidate,
) -> MacacaResult<()>
where
    J: MemoryGovernanceJournal,
{
    journal
        .append_audit(
            MemoryAuditEvent::new(
                MemoryAuditEventKind::CapturedCandidate,
                candidate.scope.clone(),
                "candidate captured by governance runtime",
            )
            .candidate_id(candidate.id.clone()),
        )
        .await
}
