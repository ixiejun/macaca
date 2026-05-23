//! Metadata-only Skill curation lifecycle commands.
//!
//! These DTOs are separated from usage telemetry so lifecycle mutation can grow
//! into policy, approval, memento, and durable Store providers without turning
//! the governance module into a large curation engine.  The commands carry only
//! sanitized identity, reason, and evidence references; skill bodies and raw
//! provider payloads stay outside this service contract.

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::governance::{SkillAuthorKind, SkillLifecycleState};
use crate::service_contract::{SkillServicePolicyHints, SkillServiceScope};

/// Metadata-only lifecycle actions accepted by curation commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCurationLifecycleAction {
    Pin,
    Unpin,
    Archive,
    Restore,
}

/// Traced command for one governed Skill lifecycle mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationLifecycleCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub skill_id: String,
    pub name: String,
    pub source: String,
    pub source_scope: String,
    pub author_kind: SkillAuthorKind,
    pub reason: String,
    pub evidence_ids: Vec<String>,
    /// Policy decision refs prove that the lifecycle mutation passed an
    /// explicit governance decision before the provider mutates metadata.  The
    /// refs are typed separately from free-form policy metadata so Store/EventLog
    /// replay can audit every mutating transition without parsing hints.
    #[serde(default)]
    pub policy_decision_refs: Vec<String>,
    pub policy: SkillServicePolicyHints,
}

impl SkillCurationLifecycleCommand {
    /// Stable record key used by providers before a dedicated governance id.
    pub fn key(&self) -> String {
        let trimmed_id = self.skill_id.trim();
        if trimmed_id.is_empty() {
            self.name.trim().to_string()
        } else {
            trimmed_id.to_string()
        }
    }

    /// Validate the audit evidence required before mutating lifecycle metadata.
    pub fn validate(&self) -> Result<(), String> {
        if self.key().is_empty() {
            return Err("skill curation lifecycle command requires skill identity".into());
        }
        if self.reason.trim().is_empty() {
            return Err("skill curation lifecycle command requires reason".into());
        }
        if self.evidence_ids.iter().all(|id| id.trim().is_empty()) {
            return Err("skill curation lifecycle command requires evidence references".into());
        }
        if self
            .policy_decision_refs
            .iter()
            .all(|id| id.trim().is_empty())
        {
            return Err(
                "skill curation lifecycle command requires policy decision references".into(),
            );
        }
        Ok(())
    }
}

/// Result returned after a governed lifecycle metadata mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationLifecycleResult {
    pub skill_id: String,
    pub name: String,
    pub action: SkillCurationLifecycleAction,
    pub lifecycle: SkillLifecycleState,
    pub pinned: bool,
    pub mutated: bool,
    pub reason: String,
    pub evidence_ids: Vec<String>,
    /// Policy decision refs copied from the accepted command so callers can
    /// correlate the mutation response with durable governance audit records.
    #[serde(default)]
    pub policy_decision_refs: Vec<String>,
    pub trace_id: String,
    pub captured_at: DateTime<Utc>,
}
