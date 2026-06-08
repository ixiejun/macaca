//! Memory and wiki context source provider contracts.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::prompt::TrustLevel;
use crate::report::{
    ContextDecisionReport, ContextDecisionSeverity, ContextSourceKind, ContextSourceReport,
};
use crate::source::{ContextSnippet, ContextSourceReference};

/// Privacy classification attached to recalled memory items.
///
/// This makes privacy explicit at the context-engine boundary so future prompt
/// policies can decide whether certain memory can be injected, redacted, or
/// shown only to specific personas.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyTier {
    Public,
    Workspace,
    UserPrivate,
    Secret,
}

impl PrivacyTier {
    /// Stable string form reused by diagnostics/UI payloads.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Workspace => "workspace",
            Self::UserPrivate => "user_private",
            Self::Secret => "secret",
        }
    }
}

/// Bounded confidence score for recalled memory.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfidenceScore(u8);

impl ConfidenceScore {
    /// Clamp caller-provided confidence into the inclusive 0..=100 range.
    pub fn new(score: u8) -> Self {
        Self(score.min(100))
    }

    /// Return the raw bounded score.
    pub fn value(self) -> u8 {
        self.0
    }
}

/// Provenance attached to recalled memory so injected context remains auditable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextSourceProvenance {
    pub provider_id: String,
    pub source_id: String,
    pub evidence: Vec<String>,
}

/// Query shape used by memory/wiki recall providers.
///
/// The query is intentionally prompt-centric rather than storage-centric:
/// providers receive the user's retrieval query plus a max token budget, then
/// decide how many results they can safely return.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecallQuery {
    pub query: String,
    pub session_id: Option<String>,
    /// Application namespace for topology-aware backends (provider-neutral “database” semantic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<String>,
    /// Agent route for agent-private collection semantics (never an app workflow name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    pub max_tokens: u32,
    /// When true, include memories tagged for the routed agent private scope.
    #[serde(default = "serde_default_true_bool")]
    pub include_agent_private: bool,
    /// When true, include session-wide shared memories.
    #[serde(default = "serde_default_true_bool")]
    pub include_session_shared: bool,
}

fn serde_default_true_bool() -> bool {
    true
}

impl MemoryRecallQuery {
    /// Minimal constructor used by tests and simple call sites.
    pub fn lite(query: impl Into<String>, max_tokens: u32) -> Self {
        Self {
            query: query.into(),
            session_id: None,
            application_id: None,
            agent_name: None,
            max_tokens,
            include_agent_private: true,
            include_session_shared: true,
        }
    }

    /// Attaches a session id for routing [`MemorySourceProvider`] implementations.
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// One recalled memory candidate before prompt rendering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecallItem {
    pub source: ContextSourceReference,
    pub text: String,
    pub provenance: ContextSourceProvenance,
    pub confidence: ConfidenceScore,
    pub privacy_tier: PrivacyTier,
}

impl MemoryRecallItem {
    /// Convert recalled memory into an untrusted dynamic prompt candidate.
    ///
    /// Memory recall is intentionally modeled as untrusted/dynamic even when it
    /// comes from internal stores, because recalled facts should not mutate the
    /// stable prompt hash or silently outrank the core system prompt.
    pub fn into_untrusted_snippet(self) -> ContextSnippet {
        let estimated_tokens = crate::estimate::estimate_text_tokens(&self.text);
        ContextSnippet {
            source: self.source,
            byte_size: self.text.len(),
            text: self.text,
            estimated_tokens,
            pruned_tokens: 0,
            trust_level: TrustLevel::Untrusted,
            mode: crate::source::ContextRenderMode::Full,
        }
    }

    /// Convert recalled memory into a redacted report row for diagnostics/UI.
    pub fn to_report(&self) -> ContextSourceReport {
        ContextSourceReport::included(
            self.source.id.clone(),
            self.source.kind.clone(),
            self.source.label.clone(),
            crate::estimate::estimate_text_tokens(&self.text),
            self.text.len(),
        )
        .with_rendering(
            crate::source::ContextRenderMode::Full.as_str(),
            "untrusted",
            self.source.artifact_ref.clone(),
            0,
        )
        .with_recall_metadata(
            self.provenance.provider_id.clone(),
            self.provenance.source_id.clone(),
            self.confidence.value(),
            self.privacy_tier.as_str(),
            true,
        )
    }
}

/// Budget limits applied to active recall.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveRecallBudget {
    pub max_hits: usize,
    pub max_chars: usize,
    pub max_tokens: u32,
    pub timeout_ms: u64,
}

impl Default for ActiveRecallBudget {
    fn default() -> Self {
        Self {
            max_hits: 8,
            max_chars: 4_000,
            max_tokens: 1_000,
            timeout_ms: 1_500,
        }
    }
}

/// Per-candidate decision emitted by active recall policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecallDecision {
    pub selected: bool,
    pub reason: String,
}

impl RecallDecision {
    /// Decision that keeps a candidate in the recall result.
    pub fn selected(reason: impl Into<String>) -> Self {
        Self {
            selected: true,
            reason: reason.into(),
        }
    }

    /// Decision that skips a candidate while preserving the reason.
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            selected: false,
            reason: reason.into(),
        }
    }
}

/// One recall candidate after policy scoring but before prompt rendering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecallCandidate {
    pub item: MemoryRecallItem,
    pub score: u32,
    pub freshness_rank: u32,
    pub decision: RecallDecision,
}

/// Result returned by active recall providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryPrefetchResult {
    pub candidates: Vec<RecallCandidate>,
    pub snippets: Vec<ContextSnippet>,
    pub decisions: Vec<ContextDecisionReport>,
}

impl MemoryPrefetchResult {
    /// Create an empty result with a bounded diagnostic note.
    pub fn empty(message: impl Into<String>) -> Self {
        Self {
            candidates: Vec::new(),
            snippets: Vec::new(),
            decisions: vec![ContextDecisionReport {
                code: "active_recall_empty".into(),
                severity: ContextDecisionSeverity::Info,
                message: message.into(),
            }],
        }
    }
}

/// Strategy contract for active memory recall.
#[async_trait]
pub trait ActiveRecallCapability: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn prefetch(
        &self,
        query: MemoryRecallQuery,
    ) -> macaca_proto::MacacaResult<MemoryPrefetchResult>;
}

/// Policy used to score and truncate recall candidates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveRecallPolicy {
    pub budget: ActiveRecallBudget,
    pub include_application_shared: bool,
    pub include_user_scoped: bool,
    pub include_knowledge: bool,
}

impl Default for ActiveRecallPolicy {
    fn default() -> Self {
        Self {
            budget: ActiveRecallBudget::default(),
            include_application_shared: false,
            include_user_scoped: false,
            include_knowledge: false,
        }
    }
}

impl ActiveRecallPolicy {
    /// Decide whether a candidate can be admitted into the bounded result.
    pub fn admit_candidate(
        &self,
        candidate: &MemoryRecallItem,
        used_hits: usize,
        used_chars: usize,
        used_tokens: u32,
    ) -> RecallDecision {
        if used_hits >= self.budget.max_hits {
            return RecallDecision::skipped("hit_budget_exhausted");
        }
        if used_chars >= self.budget.max_chars {
            return RecallDecision::skipped("char_budget_exhausted");
        }
        if used_tokens >= self.budget.max_tokens {
            return RecallDecision::skipped("token_budget_exhausted");
        }
        if candidate.text.trim().is_empty() {
            return RecallDecision::skipped("empty_candidate");
        }
        RecallDecision::selected("within_budget")
    }
}

/// Aggregate active recall report used by diagnostics and UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveRecallReport {
    pub provider_id: String,
    pub source_breakdown: Vec<ContextSourceReport>,
    pub decisions: Vec<ContextDecisionReport>,
    pub total_candidates: usize,
    pub selected_candidates: usize,
    pub latency_ms: u64,
}

/// Hook input delivered to memory providers during lifecycle moments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryProviderHookInput {
    pub session_id: String,
    pub bounded_context_summary: String,
}

/// Contract for long-term memory providers that can supply prompt context.
///
/// Besides direct recall, the trait also exposes lifecycle hooks so providers
/// can persist turn summaries, flush state before compaction, or react when a
/// runtime switches to a new session lineage node.
#[async_trait]
pub trait MemorySourceProvider: Send + Sync {
    fn provider_id(&self) -> &str;

    /// Recall memory candidates for prompt construction.
    async fn recall(
        &self,
        query: MemoryRecallQuery,
    ) -> macaca_proto::MacacaResult<Vec<MemoryRecallItem>>;

    /// Optional hook after a turn completes and a bounded summary is available.
    async fn sync_turn(&self, _input: MemoryProviderHookInput) -> macaca_proto::MacacaResult<()> {
        Ok(())
    }

    /// Optional hook before the runtime compacts the current conversation.
    async fn before_compaction(
        &self,
        _input: MemoryProviderHookInput,
    ) -> macaca_proto::MacacaResult<()> {
        Ok(())
    }

    /// Optional hook when the active session lineage node changes.
    async fn session_switched(
        &self,
        _input: MemoryProviderHookInput,
    ) -> macaca_proto::MacacaResult<()> {
        Ok(())
    }
}

/// One governed digest row normalized for context assembly (DTO / anti-corruption boundary).
///
/// [`KnowledgeDigestContextProvider`] renders this into [`crate::composer::ContextCandidate`] with
/// **reference-only** provenance: `evidence_memory_ids` mirrors `ClaimEvidence::source_id` from the
/// memory governance compiler. Operators may correlate these with tombstone/audit logs without
/// stuffing raw row bodies into prompt reports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeDigestItem {
    pub claim_id: String,
    pub label: String,
    pub statement: String,
    pub provenance: ContextSourceProvenance,
    pub evidence_memory_ids: Vec<String>,
    pub confidence: f32,
    pub freshness: f32,
    pub privacy_tier: PrivacyTier,
    pub request_only: bool,
    pub redacted: bool,
}

impl KnowledgeDigestItem {
    /// Convert the claim confidence score into the bounded integer range used by reports/UI.
    pub fn confidence_score(&self) -> u8 {
        let bounded = (self.confidence * 100.0).round().clamp(0.0, 100.0);
        bounded as u8
    }

    /// Convert governed digest metadata into a redacted report row for diagnostics/UI.
    pub fn to_report(&self, rendered_text: &str) -> ContextSourceReport {
        let evidence_ref = (!self.evidence_memory_ids.is_empty())
            .then(|| format!("evidence_refs={}", self.evidence_memory_ids.join(",")));
        ContextSourceReport::included(
            self.claim_id.clone(),
            ContextSourceKind::WikiDigest,
            self.label.clone(),
            crate::estimate::estimate_text_tokens(rendered_text),
            rendered_text.len(),
        )
        .with_rendering(
            crate::source::ContextRenderMode::Full.as_str(),
            "untrusted",
            evidence_ref,
            0,
        )
        .with_recall_metadata(
            self.provenance.provider_id.clone(),
            self.provenance.source_id.clone(),
            self.confidence_score(),
            self.privacy_tier.as_str(),
            self.request_only,
        )
    }
}

/// Port implemented by memory-side adapters (see web/kernel bridges) to expose compiled digests.
///
/// ### Dependency inversion
/// `macaca-context` never imports vector drivers or Milvus; it only awaits bounded `Vec` rows that
/// were **already** filtered by governance (tombstones, scope, promotions).
#[async_trait]
pub trait KnowledgeDigestCapability: Send + Sync {
    fn capability_id(&self) -> &str;

    async fn digest_for_request(
        &self,
        input: &crate::engine::ContextAssembleInput,
    ) -> macaca_proto::MacacaResult<Vec<KnowledgeDigestItem>>;
}

/// Contract for wiki/digest style providers that return curated knowledge snippets.
#[async_trait]
pub trait WikiDigestSourceProvider: Send + Sync {
    fn provider_id(&self) -> &str;

    /// Return bounded digest items relevant to the query.
    async fn digest(
        &self,
        query: MemoryRecallQuery,
    ) -> macaca_proto::MacacaResult<Vec<MemoryRecallItem>>;
}

/// Helper for building a `Memory`-kind source reference with provider provenance.
pub fn memory_source(
    provider_id: impl Into<String>,
    source_id: impl Into<String>,
    label: impl Into<String>,
) -> ContextSourceReference {
    ContextSourceReference::new(
        source_id,
        ContextSourceKind::Memory,
        format!("memory:{}", label.into()),
    )
    .artifact_ref(provider_id.into())
}

/// Helper for building a `WikiDigest`-kind source reference with provider provenance.
pub fn wiki_digest_source(
    provider_id: impl Into<String>,
    source_id: impl Into<String>,
    label: impl Into<String>,
) -> ContextSourceReference {
    ContextSourceReference::new(
        source_id,
        ContextSourceKind::WikiDigest,
        format!("wiki:{}", label.into()),
    )
    .artifact_ref(provider_id.into())
}

/// Contract tests for memory recall governance (extracted to satisfy OS file-size gate).
#[cfg(test)]
mod tests;
