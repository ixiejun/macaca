//! Provider-neutral proposal materialization contracts for Skill self-evolution.
//!
//! Materialization is the first lane that can turn a processed proposal into a
//! concrete AgentSkills-compatible package file.  The DTOs are intentionally
//! body-free on outputs: callers can audit refs, digests, byte counts, and
//! rollback mementos without receiving raw generated `SKILL.md` content.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::{SkillPackageOwnershipClass, SkillServicePolicyHints, SkillServiceScope};

/// Result status for proposal materialization.
///
/// `Previewed` means the Builder produced a plan and digest without side
/// effects.  `Applied` means the Skill content mutation Strategy reported an
/// actual package write.  `Denied` is a structured, auditable refusal and never
/// means a partial write happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillProposalMaterializationStatus {
    Previewed,
    Applied,
    Denied,
    Unavailable,
}

/// Structured materialization denial returned without throwing service errors.
///
/// The provider returns this shape for policy/readiness denials so shells and
/// operators can distinguish governance refusal from infrastructure failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalMaterializationDenied {
    pub reason: String,
    pub retryable_with_approval: bool,
}

impl SkillProposalMaterializationDenied {
    /// Build a bounded denial reason suitable for logs and audit reports.
    pub fn new(reason: impl Into<String>, retryable_with_approval: bool) -> Self {
        Self {
            reason: reason.into().chars().take(256).collect(),
            retryable_with_approval,
        }
    }
}

/// Command for materializing one ready Skill evolution proposal.
///
/// The command carries a package root selected by an approved caller or future
/// Store provider.  It does not infer filesystem locations from application
/// names, task families, or provider identities, preserving Macaca's generic OS
/// boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalMaterializationCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub proposal_id: String,
    pub package_root: PathBuf,
    pub dry_run: bool,
    pub ownership: SkillPackageOwnershipClass,
    pub reason: String,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub policy: SkillServicePolicyHints,
}

impl SkillProposalMaterializationCommand {
    /// Validate trace, policy, evidence, and package-readiness gates.
    ///
    /// Readiness against proposal-processing state is checked by the provider
    /// because it requires service-owned state.  This envelope validation
    /// ensures no provider reaches that state check with an untraceable or
    /// unauditable command.
    pub fn validate(&self) -> Result<(), SkillProposalMaterializationDenied> {
        if self.trace.trace_id.trim().is_empty() {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires trace context",
                false,
            ));
        }
        if self.proposal_id.trim().is_empty() {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires proposal id",
                false,
            ));
        }
        if self.package_root.as_os_str().is_empty() {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires package root",
                false,
            ));
        }
        if self.reason.trim().is_empty() {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires rationale",
                false,
            ));
        }
        if self.evidence_ids.iter().all(|id| id.trim().is_empty()) {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires evidence references",
                false,
            ));
        }
        if self
            .policy_decision_refs
            .iter()
            .all(|id| id.trim().is_empty())
        {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires policy decision references",
                true,
            ));
        }
        if self.policy.package_ready != Some(true) || self.policy.entitlement_ready != Some(true) {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires package guard and entitlement approval",
                true,
            ));
        }
        Ok(())
    }

    /// Return sanitized evidence refs for result DTOs and downstream mutation.
    pub fn sanitized_evidence_ids(&self) -> Vec<String> {
        sanitized_refs(&self.evidence_ids)
    }

    /// Return sanitized policy refs for result DTOs and downstream mutation.
    pub fn sanitized_policy_decision_refs(&self) -> Vec<String> {
        sanitized_refs(&self.policy_decision_refs)
    }

    /// Return sanitized audit refs for result DTOs.
    pub fn sanitized_audit_event_ids(&self) -> Vec<String> {
        sanitized_refs(&self.audit_event_ids)
    }
}

/// Body-free materialization result.
///
/// The result exposes digest and mutation refs instead of generated markdown.
/// Operators can prove what happened while Store/EventLog or package providers
/// keep access control over actual content bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalMaterializationResult {
    pub status: SkillProposalMaterializationStatus,
    pub proposal_id: String,
    pub skill_id: String,
    pub relative_path: PathBuf,
    pub content_digest: Option<String>,
    pub planned_bytes: u64,
    pub bytes_written: u64,
    pub rollback_memento_ref: Option<String>,
    pub denied_reason: Option<String>,
    pub mutated: bool,
    pub promoted: bool,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub trace_id: String,
    pub captured_at: DateTime<Utc>,
}

impl SkillProposalMaterializationResult {
    /// Build a structured denial without generated content or mutation refs.
    pub fn denied(
        command: &SkillProposalMaterializationCommand,
        denial: SkillProposalMaterializationDenied,
    ) -> Self {
        Self {
            status: SkillProposalMaterializationStatus::Denied,
            proposal_id: command.proposal_id.clone(),
            skill_id: String::new(),
            relative_path: PathBuf::from("SKILL.md"),
            content_digest: None,
            planned_bytes: 0,
            bytes_written: 0,
            rollback_memento_ref: None,
            denied_reason: Some(denial.reason),
            mutated: false,
            promoted: false,
            evidence_ids: command.sanitized_evidence_ids(),
            policy_decision_refs: command.sanitized_policy_decision_refs(),
            audit_event_ids: command.sanitized_audit_event_ids(),
            trace_id: command.trace.trace_id.clone(),
            captured_at: Utc::now(),
        }
    }
}

fn sanitized_refs(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.chars().take(256).collect())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use macaca_proto::TraceContext;

    use crate::{
        SkillPackageOwnershipClass, SkillProposalMaterializationCommand,
        SkillProposalMaterializationResult, SkillProposalMaterializationStatus,
        SkillServicePolicyHints, SkillServiceScope,
    };

    fn approved_policy() -> SkillServicePolicyHints {
        SkillServicePolicyHints {
            required_permissions: vec!["skill.proposal_materialization".into()],
            entitlement_ready: Some(true),
            package_ready: Some(true),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn materialization_apply_requires_policy_refs() {
        let command = SkillProposalMaterializationCommand {
            trace: TraceContext::new("trace-materialization-contract"),
            scope: SkillServiceScope::default(),
            proposal_id: "proposal-ready-1".into(),
            package_root: PathBuf::from("/tmp/materialization-contract"),
            dry_run: false,
            ownership: SkillPackageOwnershipClass::AgentPrivate,
            reason: "approved materialization from processed proposal".into(),
            evidence_ids: vec!["evidence://proposal/1".into()],
            policy_decision_refs: Vec::new(),
            audit_event_ids: vec!["audit://proposal/materialization".into()],
            policy: approved_policy(),
        };

        let denial = command
            .validate()
            .expect_err("apply materialization must require policy refs");
        assert!(denial.reason.contains("policy decision"));
    }

    #[test]
    fn materialization_result_serializes_without_generated_skill_body() {
        let result = SkillProposalMaterializationResult {
            status: SkillProposalMaterializationStatus::Applied,
            proposal_id: "proposal-ready-1".into(),
            skill_id: "skill://agent/proposal-ready-1".into(),
            relative_path: PathBuf::from("SKILL.md"),
            content_digest: Some("skill-draft-digest://abc123".into()),
            planned_bytes: 128,
            bytes_written: 128,
            rollback_memento_ref: Some("skill-mutation-memento://abc123".into()),
            denied_reason: None,
            mutated: true,
            promoted: true,
            evidence_ids: vec!["evidence://proposal/1".into()],
            policy_decision_refs: vec!["policy://proposal/approved".into()],
            audit_event_ids: vec!["audit://proposal/materialization".into()],
            trace_id: "trace-materialization-result".into(),
            captured_at: chrono::Utc::now(),
        };

        let serialized =
            serde_json::to_string(&result).expect("materialization result should serialize");
        assert!(serialized.contains("skill-draft-digest://abc123"));
        assert!(!serialized.contains("reusable procedure body"));
        assert!(!serialized.contains("SKILL.md body"));
    }
}
