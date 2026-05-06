//! [`ContextCandidate`] and related enums: value objects in the composition pipeline.
//!
//! Design notes (aligned with OS integration plans):
//! - **Anti-corruption**: external sources (files, memory, MCP, skills) become candidates; the
//!   composer alone decides inclusion and ordering.
//! - **Auditability**: `source_id`, `kind`, `trust`, and `cache_class` surface in reports without
//!   leaking provider internals.
//! - **Determinism**: ordering uses stable tie-breakers (`priority`, `source_id`), not map iteration.

use serde::{Deserialize, Serialize};

use crate::prompt::TrustLevel;
use crate::report::{ContextSourceKind, ContextSourceReport};

/// Semantic kind for a candidate: what this content represents for reporting and rendering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContextCandidateKind {
    /// Agent/application bootstrap-style text (future profile provider).
    ProfileBootstrap,
    /// Compact capability index (skills / MCP summaries, etc.).
    CapabilityIndex,
    /// Governed knowledge digest / compiled wiki claims (memory governance layer).
    KnowledgeDigest,
    /// Active or passive memory recall.
    MemoryRecall,
    /// Workspace guides, path hints, etc.
    WorkspaceGuide,
    /// Session/runtime diagnostics or trace text.
    RuntimeDiagnostic,
    /// Extension bucket for callers.
    Custom,
}

/// Scope: which OS boundary this candidate belongs to (not tied to a specific app name).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContextScope {
    /// Install / process-wide.
    System,
    /// Single `ApplicationId`.
    Application,
    /// Agent-level.
    Agent,
    /// Session / conversation.
    Session,
    /// Single model request (narrowest; usually ephemeral).
    Request,
}

/// Render target: where the composer policy places text (system vs user vs dedicated message).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContextTarget {
    /// Contributes to the stable system prefix (when `cache_class == Stable` and policy allows).
    SystemStable,
    /// System-side but dynamic (after stable/dynamic boundary).
    SystemDynamic,
    /// Auxiliary user-side context (common Hermes/OpenClaw pattern: fenced injection).
    UserSide,
    /// Dedicated user message (foundation merge uses one composite user block by default).
    StandaloneUserMessage,
}

/// Classification for prompt-cache behavior (aligned with OpenClaw stable vs dynamic files).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContextCacheClass {
    /// Potentially reusable across requests (stable rendering only when trust/policy allow).
    Stable,
    /// May change every request; must not pollute stable hash.
    Dynamic,
    /// Conservative default when the caller cannot prove stability: treated like dynamic.
    Unknown,
}

/// Numeric signals copied from memory governance claims for Strategy-style suppression decisions.
///
/// Only [`ContextCandidateKind::KnowledgeDigest`] rows populate this structure; other kinds leave it
/// `None` so the merge pass can ignore them cheaply.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct DigestStrengthSnapshot {
    pub confidence: f32,
    pub freshness: f32,
}

/// Alias: capability rows (skills / MCP / runtime tools) use [`ContextCandidateKind::CapabilityIndex`].
pub type CapabilityCandidate = ContextCandidate;

/// One candidate: the only shape a provider may emit (anti-corruption boundary).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextCandidate {
    /// Stable id unique within a provider; used for dedup and tie-break.
    pub source_id: String,
    pub kind: ContextCandidateKind,
    pub scope: ContextScope,
    /// Higher values win budget fights first.
    pub priority: u32,
    pub trust: TrustLevel,
    pub cache_class: ContextCacheClass,
    pub target: ContextTarget,
    /// Raw text (provider must already enforce path/size safety).
    pub content: String,
    /// Estimated token count (typically `estimate_text_tokens`).
    pub token_estimate: u32,
    /// Human-readable diagnostics for the plan (not sent to the model by default).
    #[serde(default)]
    pub diagnostics: Vec<String>,
    /// Stable opaque identifiers for memory rows cited by compiled claims (used exclusively by the
    /// digest-vs-raw **Strategy** for overlap testing — never shown to the model verbatim).
    #[serde(default)]
    pub evidence_memory_ids: Vec<String>,
    /// Structured strength metadata for governed knowledge rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest_strength: Option<DigestStrengthSnapshot>,
    /// Optional diagnostics-only report row preserved across governance/composer stages.
    ///
    /// This lets runtime reports expose per-candidate metadata such as provenance or privacy even
    /// when the actual prompt text is merged into synthetic request messages later.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_report: Option<ContextSourceReport>,
}

impl ContextCandidate {
    /// Maps candidate kind to the existing [`ContextSourceKind`] used in reports.
    pub fn source_kind(&self) -> ContextSourceKind {
        match self.kind {
            ContextCandidateKind::ProfileBootstrap => ContextSourceKind::Workspace,
            ContextCandidateKind::CapabilityIndex => ContextSourceKind::Skill,
            ContextCandidateKind::KnowledgeDigest => ContextSourceKind::WikiDigest,
            ContextCandidateKind::MemoryRecall => ContextSourceKind::Memory,
            ContextCandidateKind::WorkspaceGuide => ContextSourceKind::Workspace,
            ContextCandidateKind::RuntimeDiagnostic => ContextSourceKind::Trace,
            ContextCandidateKind::Custom => ContextSourceKind::External,
        }
    }

    /// Unknown cache class is forced to dynamic (“cannot prove stable ⇒ dynamic”).
    pub fn effective_cache_class(&self) -> ContextCacheClass {
        match self.cache_class {
            ContextCacheClass::Unknown => ContextCacheClass::Dynamic,
            other => other,
        }
    }
}
