//! Memory governance layer.
//!
//! This module keeps long-term memory under explicit policy control rather than
//! letting auto-capture write directly into durable stores. The design follows
//! a small policy chain:
//! - candidates capture raw observations
//! - promotion policies decide whether data becomes committed memory
//! - audit events record every state transition
//! - tombstones prevent deleted entries from being revived during replay
//!
//! The module is intentionally provider-neutral. It defines the governance
//! contracts and a conservative in-memory implementation that can later be
//! swapped for a database-backed or remote implementation.

pub mod artifacts;
pub mod audit;
pub mod candidate;
pub mod compiler;
pub mod context;
pub mod facade;
pub mod migration;
pub mod promotion;
pub mod runtime;
pub mod tombstone;

pub use artifacts::{
    ArtifactContent, ArtifactKind, CitationArtifact, MemoryArtifact, MemoryArtifactList,
    MemoryArtifactScope, ProjectDecisionLogArtifact, WikiDigestArtifact,
};
pub use audit::{MemoryAuditEvent, MemoryAuditEventKind};
pub use candidate::{
    CandidateDecision, CandidateSource, MemoryCandidate, MemoryCandidateCapture,
    MemoryCandidateStore,
};
pub use compiler::{
    ClaimConfidence, ClaimEvidence, ClaimFreshness, ClaimGroup, ContradictionStrategy,
    DeterministicContradictionStrategy, KnowledgeClaim, KnowledgeCompileCapability,
    KnowledgeCompileRequest, KnowledgeCompileResult, KnowledgeCompiler,
};
pub use context::{
    compiled_digest_candidates, KnowledgeContextCandidate, KnowledgeContextSourceKind,
};
pub use facade::{
    CandidateCaptureResult, CandidatePromotionResult, FacadeDeletePropagator, GovernedMemoryFacade,
    MemoryDeletePropagator,
};
pub use migration::{
    MemoryProviderMigrationPlan, MemoryProviderMigrationPort, MemoryProviderMigrationResult,
    MemoryProviderMigrationRuntime, MemoryProviderMigrationStatus,
};
pub use promotion::{
    ConservativePromotionPolicy, MemoryPromotionDecision, MemoryPromotionPolicy,
    MemoryPromotionPolicyId, MemoryPromotionTarget,
};
pub use runtime::{
    audit_candidate_capture, DefaultMemoryCandidateCapturePolicy, DisabledMemoryCompactionStrategy,
    InMemoryGovernanceJournal, MemoryCandidateCapturePolicy, MemoryCompactionResult,
    MemoryCompactionStrategy, MemoryGovernanceJournal,
};
pub use tombstone::{MemoryDeletePropagation, MemoryDeletePropagationStep, MemoryTombstone};

#[cfg(test)]
mod tests;
