//! HTTP request DTOs for skill-operations routes.
//!
//! These structs are transport-layer only: they deserialize operator input and
//! are mapped into SDK command types by handler modules. Lifecycle validation,
//! policy admission, rollback ownership, and proposal semantics remain inside
//! the SDK-backed Skill service provider.

use std::path::PathBuf;

use macaca_sdk::skill::{SelfEvolutionEvaluationRecord, SelfEvolutionScore, SkillAuthorKind};
use serde::{Deserialize, Serialize};

/// Request body shared by approval-gated Skill operation routes.
///
/// The Web shell only normalizes operator transport fields into service DTOs.
/// Lifecycle validation, policy admission, rollback ownership, and proposal
/// semantics remain inside the SDK-backed Skill service provider.
#[derive(Debug, Default, Deserialize)]
pub struct SkillOperatorCommandRequest {
    pub reason: Option<String>,
    pub rationale: Option<String>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub policy_decision_refs: Vec<String>,
    #[serde(default)]
    pub approval_refs: Vec<String>,
    #[serde(default)]
    pub audit_event_ids: Vec<String>,
    #[serde(default)]
    pub required_permissions: Vec<String>,
    pub entitlement_ready: Option<bool>,
    pub package_ready: Option<bool>,
    #[serde(default)]
    pub metadata: std::collections::BTreeMap<String, String>,
    pub name: Option<String>,
    pub source: Option<String>,
    pub source_scope: Option<String>,
    pub author_kind: Option<SkillAuthorKind>,
    pub stale_after_days: Option<i64>,
    pub narrow_use_threshold: Option<u64>,
    pub rollback_ref: Option<String>,
    pub package_collection_root: Option<PathBuf>,
    pub dry_run: Option<bool>,
    pub batch_limit: Option<u16>,
    pub min_ready_score: Option<u8>,
    pub ownership: Option<macaca_sdk::skill::SkillPackageOwnershipClass>,
}

/// Request body for building a self-evolution evaluation report.
///
/// The Web shell accepts only the provider-neutral record and optional prior
/// score. If a score is omitted, Web asks the SDK-backed Skill service to score
/// first; either way, report construction remains service-owned.
#[derive(Debug, Deserialize, Serialize)]
pub struct SkillEvaluationReportRequest {
    pub record: SelfEvolutionEvaluationRecord,
    pub score: Option<SelfEvolutionScore>,
    #[serde(default = "default_include_markdown")]
    pub include_markdown: bool,
}

fn default_include_markdown() -> bool {
    true
}
