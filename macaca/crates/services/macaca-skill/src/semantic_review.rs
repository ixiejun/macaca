//! Optional semantic review provider contracts for Skill curation.
//!
//! Semantic review is a replaceable Strategy owned by the Skill service
//! boundary.  Providers may inspect already-sanitized curation recommendations
//! and return typed proposals, but they must never mutate skill files,
//! lifecycle state, aliases, or package bytes directly.  The default
//! unavailable provider keeps deterministic curation operational while making
//! optional-provider absence explicit and auditable.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::curation::SkillCurationRecommendation;
use crate::service_contract::SkillServiceScope;

const MAX_SEMANTIC_REVIEW_PROPOSALS: usize = 32;
const MAX_SEMANTIC_REVIEW_DIAGNOSTICS: usize = 8;
const MAX_SEMANTIC_REVIEW_DIAGNOSTIC_BYTES: usize = 256;
const MAX_SEMANTIC_REVIEW_RATIONALE_BYTES: usize = 512;
const MAX_SEMANTIC_REVIEW_EVIDENCE_REFS: usize = 16;

/// Coarse provider execution state recorded in bounded reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillSemanticReviewStatus {
    Unavailable,
    Completed,
    Failed,
}

/// Typed semantic proposal categories that can be policy-reviewed later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillSemanticReviewProposalKind {
    SimilarityCluster,
    DuplicateCandidate,
    SuccessFailureCorrelation,
    SupportFileDemotion,
    UmbrellaMerge,
    PatchSuggestion,
}

/// One bounded proposal produced by an optional semantic provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillSemanticReviewProposal {
    pub proposal_id: String,
    pub kind: SkillSemanticReviewProposalKind,
    pub source_skill_ids: Vec<String>,
    pub target_skill_id: Option<String>,
    pub rationale: String,
    pub confidence: f32,
    pub evidence_ids: Vec<String>,
}

impl SkillSemanticReviewProposal {
    /// Validate one typed proposal before the Skill service records it.
    pub fn validate(&self) -> Result<(), String> {
        if self.proposal_id.trim().is_empty() {
            return Err("semantic review proposal requires proposal_id".into());
        }
        if self.source_skill_ids.iter().all(|id| id.trim().is_empty()) {
            return Err("semantic review proposal requires source_skill_ids".into());
        }
        if self.rationale.trim().is_empty() {
            return Err("semantic review proposal requires bounded rationale".into());
        }
        if self.rationale.len() > MAX_SEMANTIC_REVIEW_RATIONALE_BYTES {
            return Err("semantic review proposal rationale exceeds bounded output".into());
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err("semantic review proposal confidence must be between 0 and 1".into());
        }
        if self.evidence_ids.len() > MAX_SEMANTIC_REVIEW_EVIDENCE_REFS {
            return Err("semantic review proposal carries too many evidence refs".into());
        }
        Ok(())
    }
}

/// Pairwise similarity input prepared by deterministic or optional providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillSemanticSimilarityInput {
    pub skill_id: String,
    pub comparable_skill_id: String,
    pub score_hint: Option<f32>,
    pub evidence_ids: Vec<String>,
}

/// Skill-set input for clustering and umbrella-merge analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSemanticSkillSetInput {
    pub set_id: String,
    pub skill_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
}

/// Duplicate-candidate input that keeps matching evidence as refs only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSemanticDuplicateInput {
    pub source_skill_id: String,
    pub candidate_skill_id: String,
    pub evidence_ids: Vec<String>,
}

/// Success/failure correlation input for generic effectiveness review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSemanticEffectivenessInput {
    pub skill_id: String,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub evidence_ids: Vec<String>,
}

/// Reference graph input used for support-file and dependency proposals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSemanticReferenceGraphInput {
    pub skill_id: String,
    pub referenced_skill_ids: Vec<String>,
    pub support_file_refs: Vec<String>,
    pub evidence_ids: Vec<String>,
}

/// Budget hints that bound optional semantic provider work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSemanticReviewBudget {
    pub max_provider_calls: u32,
    pub max_input_items: u32,
    pub max_output_proposals: u32,
}

impl Default for SkillSemanticReviewBudget {
    fn default() -> Self {
        Self {
            max_provider_calls: 0,
            max_input_items: 0,
            max_output_proposals: MAX_SEMANTIC_REVIEW_PROPOSALS as u32,
        }
    }
}

/// Resource limits that runtime-host decorators can enforce before provider calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSemanticReviewResourceLimits {
    pub timeout_ms: u64,
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
}

impl Default for SkillSemanticReviewResourceLimits {
    fn default() -> Self {
        Self {
            timeout_ms: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
        }
    }
}

/// Sanitization policy for optional semantic provider boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSemanticReviewSanitizationPolicy {
    pub redact_prompts: bool,
    pub redact_provider_payloads: bool,
    pub redact_secrets: bool,
}

impl Default for SkillSemanticReviewSanitizationPolicy {
    fn default() -> Self {
        Self {
            redact_prompts: true,
            redact_provider_payloads: true,
            redact_secrets: true,
        }
    }
}

/// Request passed to a semantic review Strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillSemanticReviewRequest {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub dry_run: bool,
    pub recommendations: Vec<SkillCurationRecommendation>,
    #[serde(default)]
    pub similarity_inputs: Vec<SkillSemanticSimilarityInput>,
    #[serde(default)]
    pub clustering_inputs: Vec<SkillSemanticSkillSetInput>,
    #[serde(default)]
    pub duplicate_inputs: Vec<SkillSemanticDuplicateInput>,
    #[serde(default)]
    pub effectiveness_inputs: Vec<SkillSemanticEffectivenessInput>,
    #[serde(default)]
    pub reference_graph_inputs: Vec<SkillSemanticReferenceGraphInput>,
    #[serde(default)]
    pub budget: SkillSemanticReviewBudget,
    #[serde(default)]
    pub resource_limits: SkillSemanticReviewResourceLimits,
    #[serde(default)]
    pub sanitization_policy: SkillSemanticReviewSanitizationPolicy,
    pub captured_at: DateTime<Utc>,
}

/// Structured semantic review output embedded in curation reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillSemanticReviewResult {
    pub provider_id: String,
    pub status: SkillSemanticReviewStatus,
    pub proposals: Vec<SkillSemanticReviewProposal>,
    pub diagnostics: Vec<String>,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

impl SkillSemanticReviewResult {
    /// Build the canonical absent-provider result.
    ///
    /// The diagnostic is deliberately generic.  It records absence without
    /// leaking prompts, provider payloads, package content, or secret material.
    pub fn unavailable(captured_at: DateTime<Utc>) -> Self {
        Self {
            provider_id: UnavailableSkillSemanticReviewProvider::PROVIDER_ID.into(),
            status: SkillSemanticReviewStatus::Unavailable,
            proposals: Vec::new(),
            diagnostics: vec!["semantic review provider is unavailable".into()],
            mutated: false,
            captured_at,
        }
    }

    /// Default used for stable serde decoding of older reports.
    pub fn unavailable_default() -> Self {
        Self::unavailable(Utc::now())
    }

    /// Convert provider failure into bounded, sanitized structured output.
    pub fn provider_error(
        provider_id: impl Into<String>,
        error: impl AsRef<str>,
        captured_at: DateTime<Utc>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            status: SkillSemanticReviewStatus::Failed,
            proposals: Vec::new(),
            diagnostics: vec![sanitize_semantic_text(error.as_ref())],
            mutated: false,
            captured_at,
        }
    }

    /// Human-readable status kept for existing report fields and shell output.
    pub fn status_message(&self) -> String {
        match self.status {
            SkillSemanticReviewStatus::Unavailable => {
                format!("unavailable: {}", self.diagnostics.join("; "))
            }
            SkillSemanticReviewStatus::Completed => {
                format!(
                    "completed: {} typed semantic proposals",
                    self.proposals.len()
                )
            }
            SkillSemanticReviewStatus::Failed => format!("failed: {}", self.diagnostics.join("; ")),
        }
    }

    /// Validate provider output before the Skill service trusts it.
    ///
    /// Semantic providers are proposal producers only.  Direct mutation is
    /// rejected here so future runtime-host providers cannot accidentally turn
    /// review into a side-effecting pathway.
    pub fn validate_provider_output(&self) -> Result<(), String> {
        if self.mutated {
            return Err("semantic review providers must not mutate skill state".into());
        }
        if self.proposals.len() > MAX_SEMANTIC_REVIEW_PROPOSALS {
            return Err("semantic review provider returned too many proposals".into());
        }
        if self.diagnostics.len() > MAX_SEMANTIC_REVIEW_DIAGNOSTICS {
            return Err("semantic review provider returned too many diagnostics".into());
        }
        if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.len() > MAX_SEMANTIC_REVIEW_DIAGNOSTIC_BYTES)
        {
            return Err("semantic review diagnostics exceed bounded output".into());
        }
        for proposal in &self.proposals {
            proposal.validate()?;
        }
        Ok(())
    }
}

fn sanitize_semantic_text(value: &str) -> String {
    let mut sanitized = value
        .replace("raw_provider_payload", "[redacted]")
        .replace("provider_payload", "[redacted]")
        .replace("prompt", "[redacted]")
        .replace("secret", "[redacted]");
    if sanitized.len() > MAX_SEMANTIC_REVIEW_DIAGNOSTIC_BYTES {
        sanitized.truncate(MAX_SEMANTIC_REVIEW_DIAGNOSTIC_BYTES);
    }
    sanitized
}

impl Default for SkillSemanticReviewResult {
    fn default() -> Self {
        Self::unavailable_default()
    }
}

/// Replaceable Strategy interface for semantic curation providers.
#[async_trait]
pub trait SkillSemanticReviewProvider: Send + Sync {
    /// Return typed proposals only.
    ///
    /// Implementations must treat `request.recommendations` as sanitized input
    /// and must return bounded metadata.  Direct skill mutation belongs to
    /// later policy-gated mutation commands, not this provider.
    async fn review(&self, request: SkillSemanticReviewRequest) -> SkillSemanticReviewResult;
}

/// Null Object provider used when no semantic reviewer is configured.
#[derive(Debug, Default)]
pub struct UnavailableSkillSemanticReviewProvider;

impl UnavailableSkillSemanticReviewProvider {
    /// Stable provider id recorded in logs and reports for absent backends.
    pub const PROVIDER_ID: &'static str = "unavailable-skill-semantic-review";
}

#[async_trait]
impl SkillSemanticReviewProvider for UnavailableSkillSemanticReviewProvider {
    async fn review(&self, request: SkillSemanticReviewRequest) -> SkillSemanticReviewResult {
        SkillSemanticReviewResult::unavailable(request.captured_at)
    }
}
