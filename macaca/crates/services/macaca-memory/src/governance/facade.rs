//! Governance decorator for memory facades.
//!
//! `GovernedMemoryFacade` uses the Decorator pattern: it wraps any existing
//! `MemoryFacade` and adds governance behavior without changing the wrapped
//! backend. This keeps the core storage implementations pluggable while still
//! giving runtime code one place to record audit events, tombstones, promotion
//! decisions, and deletion propagation diagnostics.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId};
use tokio::sync::RwLock;

use crate::core::facade::{
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemorySearchRequest, MemoryWriteRequest,
};
use crate::core::status::MemoryStatusReport;

use super::audit::{MemoryAuditEvent, MemoryAuditEventKind};
use super::candidate::{CandidateDecision, MemoryCandidate};
use super::promotion::{MemoryPromotionDecision, MemoryPromotionPolicy};
use super::tombstone::{MemoryDeletePropagation, MemoryDeletePropagationStep, MemoryTombstone};

/// Result of a candidate capture operation.
#[derive(Debug, Clone)]
pub struct CandidateCaptureResult {
    pub candidate_id: String,
    pub audit_event: MemoryAuditEvent,
}

/// Result of a promotion attempt.
#[derive(Debug, Clone)]
pub struct CandidatePromotionResult {
    pub decision: MemoryPromotionDecision,
    pub committed_memory_id: Option<MemoryId>,
    pub audit_event: MemoryAuditEvent,
}

/// Pluggable deletion propagation target.
///
/// Backends that need deletion notifications can implement this tiny boundary
/// rather than forcing the governance facade to know about file stores, vector
/// stores, remote providers, or artifact writers directly.
#[async_trait]
pub trait MemoryDeletePropagator: Send + Sync {
    fn backend_id(&self) -> &str;
    async fn propagate_delete(&self, request: &MemoryDeleteRequest) -> MemoryDeletePropagationStep;
}

/// Propagator adapter for any normal `MemoryFacade`.
pub struct FacadeDeletePropagator {
    backend_id: String,
    facade: Arc<dyn MemoryFacade>,
}

impl FacadeDeletePropagator {
    pub fn new(backend_id: impl Into<String>, facade: Arc<dyn MemoryFacade>) -> Self {
        Self {
            backend_id: backend_id.into(),
            facade,
        }
    }
}

#[async_trait]
impl MemoryDeletePropagator for FacadeDeletePropagator {
    fn backend_id(&self) -> &str {
        &self.backend_id
    }

    async fn propagate_delete(&self, request: &MemoryDeleteRequest) -> MemoryDeletePropagationStep {
        match self.facade.delete(request.clone()).await {
            Ok(()) => MemoryDeletePropagationStep {
                backend: self.backend_id.clone(),
                attempted: true,
                succeeded: true,
                retryable: false,
                message: "delete propagated".into(),
            },
            Err(error) => MemoryDeletePropagationStep {
                backend: self.backend_id.clone(),
                attempted: true,
                succeeded: false,
                retryable: true,
                message: error.to_string(),
            },
        }
    }
}

/// In-memory governance journal used by the default decorator.
#[derive(Debug, Default)]
struct GovernanceJournal {
    candidates: Vec<MemoryCandidate>,
    audits: Vec<MemoryAuditEvent>,
    tombstones: Vec<MemoryTombstone>,
    propagations: Vec<MemoryDeletePropagation>,
    promotions: Vec<MemoryPromotionDecision>,
}

/// Decorator that adds governance behavior around any memory facade.
pub struct GovernedMemoryFacade<P> {
    inner: Arc<dyn MemoryFacade>,
    promotion_policy: P,
    delete_propagators: Vec<Arc<dyn MemoryDeletePropagator>>,
    journal: RwLock<GovernanceJournal>,
}

impl<P> GovernedMemoryFacade<P>
where
    P: MemoryPromotionPolicy,
{
    /// Wrap an existing memory facade with a promotion policy.
    pub fn new(inner: Arc<dyn MemoryFacade>, promotion_policy: P) -> Self {
        Self {
            inner,
            promotion_policy,
            delete_propagators: Vec::new(),
            journal: RwLock::new(GovernanceJournal::default()),
        }
    }

    /// Add a deletion propagation target.
    pub fn with_delete_propagator(mut self, propagator: Arc<dyn MemoryDeletePropagator>) -> Self {
        self.delete_propagators.push(propagator);
        self
    }

    /// Capture an observation as a candidate without committing it.
    pub async fn capture_candidate(
        &self,
        candidate: MemoryCandidate,
    ) -> MacacaResult<CandidateCaptureResult> {
        candidate.scope.validate()?;
        let audit = MemoryAuditEvent::new(
            MemoryAuditEventKind::CapturedCandidate,
            candidate.scope.clone(),
            "candidate captured without committing durable memory",
        )
        .candidate_id(candidate.id.clone());
        let id = candidate.id.clone();
        let mut journal = self.journal.write().await;
        journal.candidates.push(candidate);
        journal.audits.push(audit.clone());
        Ok(CandidateCaptureResult {
            candidate_id: id,
            audit_event: audit,
        })
    }

    /// Evaluate and, when allowed, commit a candidate through the wrapped facade.
    pub async fn promote_candidate(
        &self,
        candidate_id: &str,
    ) -> MacacaResult<Option<CandidatePromotionResult>> {
        let candidate = {
            let journal = self.journal.read().await;
            journal
                .candidates
                .iter()
                .find(|candidate| candidate.id == candidate_id)
                .cloned()
        };
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        let mut decision = self.promotion_policy.evaluate(&candidate);
        let committed_memory_id = if decision.decision == CandidateDecision::Promote {
            let id = self
                .inner
                .remember(
                    MemoryWriteRequest::new(candidate.scope.clone(), candidate.content.clone())
                        .layer(candidate.layer)
                        .metadata(candidate.metadata.clone()),
                )
                .await?;
            decision.committed_memory_id = Some(id);
            Some(id)
        } else {
            None
        };
        let audit_kind = match decision.decision {
            CandidateDecision::Promote => MemoryAuditEventKind::Promoted,
            CandidateDecision::Defer => MemoryAuditEventKind::Deferred,
            CandidateDecision::Reject => MemoryAuditEventKind::Rejected,
            CandidateDecision::Merge => MemoryAuditEventKind::Merged,
        };
        let audit =
            MemoryAuditEvent::new(audit_kind, candidate.scope.clone(), decision.reason.clone())
                .candidate_id(candidate.id.clone())
                .policy_id(decision.policy_id.0.clone());
        let mut journal = self.journal.write().await;
        journal.promotions.push(decision.clone());
        journal.audits.push(audit.clone());
        Ok(Some(CandidatePromotionResult {
            decision,
            committed_memory_id,
            audit_event: audit,
        }))
    }

    pub async fn audit_events(&self) -> Vec<MemoryAuditEvent> {
        self.journal.read().await.audits.clone()
    }

    pub async fn tombstones(&self) -> Vec<MemoryTombstone> {
        self.journal.read().await.tombstones.clone()
    }

    pub async fn propagations(&self) -> Vec<MemoryDeletePropagation> {
        self.journal.read().await.propagations.clone()
    }
}

#[async_trait]
impl<P> MemoryFacade for GovernedMemoryFacade<P>
where
    P: MemoryPromotionPolicy,
{
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        let scope = request.scope.clone();
        let layer = request.layer;
        let id = self.inner.remember(request).await?;
        let audit = MemoryAuditEvent::new(
            MemoryAuditEventKind::Promoted,
            scope,
            "explicit memory write committed through governed facade",
        )
        .memory_id(id)
        .layer(layer);
        self.journal.write().await.audits.push(audit);
        Ok(id)
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        self.inner.search(request).await
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.inner.get(request).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        let tombstone = MemoryTombstone::new(request.id, request.scope.clone(), "delete requested");
        let mut steps = Vec::new();
        let inner_step = match self.inner.delete(request.clone()).await {
            Ok(()) => MemoryDeletePropagationStep {
                backend: "primary".into(),
                attempted: true,
                succeeded: true,
                retryable: false,
                message: "primary delete succeeded".into(),
            },
            Err(error) => MemoryDeletePropagationStep {
                backend: "primary".into(),
                attempted: true,
                succeeded: false,
                retryable: true,
                message: error.to_string(),
            },
        };
        steps.push(inner_step);
        for propagator in &self.delete_propagators {
            steps.push(propagator.propagate_delete(&request).await);
        }
        let propagation = MemoryDeletePropagation {
            tombstone: tombstone.clone(),
            steps,
        };
        let audit = MemoryAuditEvent::new(
            MemoryAuditEventKind::Deleted,
            request.scope,
            "delete accepted and tombstoned locally",
        )
        .memory_id(tombstone.memory_id);
        let mut journal = self.journal.write().await;
        journal.tombstones.push(tombstone);
        journal.propagations.push(propagation);
        journal.audits.push(audit);
        Ok(())
    }

    fn status(&self) -> MemoryStatusReport {
        self.inner.status()
    }
}
