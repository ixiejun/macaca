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

/// Request passed to a semantic review Strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillSemanticReviewRequest {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub dry_run: bool,
    pub recommendations: Vec<SkillCurationRecommendation>,
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
            provider_id: "unavailable-skill-semantic-review".into(),
            status: SkillSemanticReviewStatus::Unavailable,
            proposals: Vec::new(),
            diagnostics: vec!["semantic review provider is unavailable".into()],
            mutated: false,
            captured_at,
        }
    }

    /// Default used for backwards-compatible serde decoding of older reports.
    pub fn unavailable_default() -> Self {
        Self::unavailable(Utc::now())
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

#[async_trait]
impl SkillSemanticReviewProvider for UnavailableSkillSemanticReviewProvider {
    async fn review(&self, request: SkillSemanticReviewRequest) -> SkillSemanticReviewResult {
        SkillSemanticReviewResult::unavailable(request.captured_at)
    }
}
