//! Provider-neutral autonomous proposal materialization operator contracts.
//!
//! The operator is a service-level Director contract. It does not generate raw
//! Skill bodies itself and does not decide application-specific policy. Instead,
//! a runtime-host provider can compose proposal processing, package-target
//! resolution, and proposal materialization Strategies behind this typed,
//! trace-required command/result boundary.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::{
    SkillPackageOwnershipClass, SkillProposalMaterializationResult,
    SkillProposalProcessingRunResult, SkillProposalProcessingStateCounts, SkillServicePolicyHints,
    SkillServiceScope,
};

/// Upper bound for one autonomous run.
///
/// The value is intentionally modest because each selected proposal can create
/// package files in apply mode. Larger backlog work should be split into
/// multiple audited runs so rollback refs and materialization outcomes stay
/// reviewable.
pub const SKILL_AUTONOMOUS_MATERIALIZATION_MAX_BATCH_LIMIT: u16 = 100;

/// Aggregate status for one autonomous materialization cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillAutonomousMaterializationStatus {
    /// No eligible proposal was selected after processing.
    Noop,
    /// At least one selected proposal produced a materialization preview.
    Previewed,
    /// At least one selected proposal was applied through content mutation.
    Applied,
    /// Admission failed before processing or every selected proposal was denied.
    Denied,
    /// Reserved for future providers that are installed but not available.
    Unavailable,
}

/// Structured denial for autonomous operator admission or proposal selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAutonomousMaterializationDenied {
    pub proposal_id: Option<String>,
    pub reason: String,
    pub retryable_with_approval: bool,
}

impl SkillAutonomousMaterializationDenied {
    /// Build a bounded denial that can be logged and returned to shells safely.
    pub fn new(
        proposal_id: Option<String>,
        reason: impl Into<String>,
        retryable_with_approval: bool,
    ) -> Self {
        Self {
            proposal_id: proposal_id.and_then(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty()).then(|| trimmed.chars().take(160).collect())
            }),
            reason: reason.into().trim().chars().take(256).collect(),
            retryable_with_approval,
        }
    }
}

/// Command for one governed autonomous materialization cycle.
///
/// `package_collection_root` is a provider-neutral container directory. The
/// runtime-host provider resolves each selected proposal into a proposal-scoped
/// child package root through a Strategy, avoiding branches on application
/// names, workflow names, providers, drivers, or business domains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAutonomousMaterializationRunCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub dry_run: bool,
    pub batch_limit: u16,
    pub min_ready_score: u8,
    pub package_collection_root: PathBuf,
    pub ownership: SkillPackageOwnershipClass,
    pub reason: String,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub policy: SkillServicePolicyHints,
}

impl SkillAutonomousMaterializationRunCommand {
    /// Validate trace, bounded batch size, evidence, and privileged readiness.
    ///
    /// The validation is intentionally stricter than a read-only snapshot. Even
    /// dry-run previews can expose package-target decisions, so they must carry
    /// evidence and policy refs that auditors can replay later.
    pub fn validate(&self) -> Result<(), SkillAutonomousMaterializationDenied> {
        if self.trace.trace_id.trim().is_empty() {
            return Err(SkillAutonomousMaterializationDenied::new(
                None,
                "skill autonomous materialization requires trace context",
                false,
            ));
        }
        if self.batch_limit == 0 {
            return Err(SkillAutonomousMaterializationDenied::new(
                None,
                "skill autonomous materialization requires positive batch limit",
                false,
            ));
        }
        if self.batch_limit > SKILL_AUTONOMOUS_MATERIALIZATION_MAX_BATCH_LIMIT {
            return Err(SkillAutonomousMaterializationDenied::new(
                None,
                "skill autonomous materialization batch limit exceeds safety cap",
                false,
            ));
        }
        if self.package_collection_root.as_os_str().is_empty() {
            return Err(SkillAutonomousMaterializationDenied::new(
                None,
                "skill autonomous materialization requires package collection root",
                false,
            ));
        }
        if self.reason.trim().is_empty() {
            return Err(SkillAutonomousMaterializationDenied::new(
                None,
                "skill autonomous materialization requires rationale",
                false,
            ));
        }
        if self.evidence_ids.iter().all(|id| id.trim().is_empty()) {
            return Err(SkillAutonomousMaterializationDenied::new(
                None,
                "skill autonomous materialization requires evidence references",
                false,
            ));
        }
        if self
            .policy_decision_refs
            .iter()
            .all(|id| id.trim().is_empty())
        {
            return Err(SkillAutonomousMaterializationDenied::new(
                None,
                "skill autonomous materialization requires policy decision references",
                true,
            ));
        }
        if self.policy.package_ready != Some(true) || self.policy.entitlement_ready != Some(true) {
            return Err(SkillAutonomousMaterializationDenied::new(
                None,
                "skill autonomous materialization requires package guard and entitlement approval",
                true,
            ));
        }
        Ok(())
    }

    /// Return sanitized evidence refs for nested processing/materialization commands.
    pub fn sanitized_evidence_ids(&self) -> Vec<String> {
        sanitized_refs(&self.evidence_ids)
    }

    /// Return sanitized policy refs for nested processing/materialization commands.
    pub fn sanitized_policy_decision_refs(&self) -> Vec<String> {
        sanitized_refs(&self.policy_decision_refs)
    }

    /// Return sanitized audit refs for nested processing/materialization commands.
    pub fn sanitized_audit_event_ids(&self) -> Vec<String> {
        sanitized_refs(&self.audit_event_ids)
    }
}

/// Body-free aggregate result for one autonomous materialization cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAutonomousMaterializationRunResult {
    pub status: SkillAutonomousMaterializationStatus,
    pub run_id: String,
    pub trace_id: String,
    pub dry_run: bool,
    pub batch_limit: u16,
    pub selected_proposal_ids: Vec<String>,
    pub processing: SkillProposalProcessingRunResult,
    pub materializations: Vec<SkillProposalMaterializationResult>,
    pub denied: Vec<SkillAutonomousMaterializationDenied>,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

impl SkillAutonomousMaterializationRunResult {
    /// Build a body-free admission denial result.
    pub fn denied(
        command: &SkillAutonomousMaterializationRunCommand,
        denial: SkillAutonomousMaterializationDenied,
    ) -> Self {
        let captured_at = Utc::now();
        let processing = SkillProposalProcessingRunResult {
            run_id: event_ref("skill-proposal-processing-skipped", captured_at),
            records: Vec::new(),
            state_counts: SkillProposalProcessingStateCounts::default(),
            duplicate_group_count: 0,
            mutated: false,
            captured_at,
        };
        Self {
            status: SkillAutonomousMaterializationStatus::Denied,
            run_id: event_ref("skill-materialization-operator-denied", captured_at),
            trace_id: command.trace.trace_id.clone(),
            dry_run: command.dry_run,
            batch_limit: command.batch_limit,
            selected_proposal_ids: Vec::new(),
            processing,
            materializations: Vec::new(),
            denied: vec![denial],
            mutated: false,
            captured_at,
        }
    }
}

/// Command for reading recent autonomous materialization operator evidence.
///
/// The snapshot is intentionally read-only and body-free. It is a Memento-style
/// service DTO: monitoring surfaces can prove that an operator cycle ran, which
/// proposals it selected, and whether mutation happened without receiving
/// generated `SKILL.md` bytes or owning materialization semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAutonomousMaterializationSnapshotCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub limit: u16,
}

/// Bounded counters for recent autonomous materialization runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAutonomousMaterializationStatusCounts {
    pub noop: u64,
    pub previewed: u64,
    pub applied: u64,
    pub denied: u64,
    pub unavailable: u64,
}

impl SkillAutonomousMaterializationStatusCounts {
    /// Count statuses without requiring shells to classify operator results.
    pub fn from_runs<'a>(
        runs: impl IntoIterator<Item = &'a SkillAutonomousMaterializationRunResult>,
    ) -> Self {
        let mut counts = Self::default();
        for run in runs {
            match run.status {
                SkillAutonomousMaterializationStatus::Noop => counts.noop += 1,
                SkillAutonomousMaterializationStatus::Previewed => counts.previewed += 1,
                SkillAutonomousMaterializationStatus::Applied => counts.applied += 1,
                SkillAutonomousMaterializationStatus::Denied => counts.denied += 1,
                SkillAutonomousMaterializationStatus::Unavailable => counts.unavailable += 1,
            }
        }
        counts
    }
}

/// Body-free read model for operator runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAutonomousMaterializationSnapshotResult {
    pub recent_runs: Vec<SkillAutonomousMaterializationRunResult>,
    pub status_counts: SkillAutonomousMaterializationStatusCounts,
    pub last_run_ref: Option<String>,
    pub captured_at: DateTime<Utc>,
}

/// Build a compact, replay-friendly id for DTO-only results.
pub fn operator_event_ref(prefix: &str, captured_at: DateTime<Utc>) -> String {
    event_ref(prefix, captured_at)
}

fn event_ref(prefix: &str, captured_at: DateTime<Utc>) -> String {
    format!(
        "{prefix}-{}",
        captured_at.timestamp_nanos_opt().unwrap_or_default()
    )
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
        SkillAutonomousMaterializationRunCommand, SkillAutonomousMaterializationRunResult,
        SkillAutonomousMaterializationSnapshotResult, SkillAutonomousMaterializationStatus,
        SkillAutonomousMaterializationStatusCounts, SkillPackageOwnershipClass,
        SkillProposalMaterializationResult, SkillProposalMaterializationStatus,
        SkillProposalProcessingRunResult, SkillProposalProcessingStateCounts,
        SkillServicePolicyHints, SkillServiceScope,
    };

    fn approved_policy() -> SkillServicePolicyHints {
        SkillServicePolicyHints {
            required_permissions: vec!["skill.proposal_materialization_operator".into()],
            entitlement_ready: Some(true),
            package_ready: Some(true),
            metadata: BTreeMap::new(),
        }
    }

    fn command(policy_decision_refs: Vec<String>) -> SkillAutonomousMaterializationRunCommand {
        SkillAutonomousMaterializationRunCommand {
            trace: TraceContext::new("trace-operator-contract"),
            scope: SkillServiceScope::default(),
            dry_run: false,
            batch_limit: 1,
            min_ready_score: 40,
            package_collection_root: PathBuf::from("/tmp/operator-contract"),
            ownership: SkillPackageOwnershipClass::AgentPrivate,
            reason: "approved autonomous operator run".into(),
            evidence_ids: vec!["evidence://operator/run".into()],
            policy_decision_refs,
            audit_event_ids: vec!["audit://operator/run".into()],
            policy: approved_policy(),
        }
    }

    #[test]
    fn autonomous_operator_apply_requires_policy_refs() {
        let denial = command(Vec::new())
            .validate()
            .expect_err("operator apply must require policy refs");
        assert!(denial.reason.contains("policy decision"));
    }

    #[test]
    fn autonomous_operator_result_serializes_without_generated_skill_body() {
        let captured_at = chrono::Utc::now();
        let result = SkillAutonomousMaterializationRunResult {
            status: SkillAutonomousMaterializationStatus::Applied,
            run_id: "skill-materialization-operator-run-1".into(),
            trace_id: "trace-operator-result".into(),
            dry_run: false,
            batch_limit: 1,
            selected_proposal_ids: vec!["proposal-1".into()],
            processing: SkillProposalProcessingRunResult {
                run_id: "skill-proposal-processing-run-1".into(),
                records: Vec::new(),
                state_counts: SkillProposalProcessingStateCounts::default(),
                duplicate_group_count: 0,
                mutated: true,
                captured_at,
            },
            materializations: vec![SkillProposalMaterializationResult {
                status: SkillProposalMaterializationStatus::Applied,
                proposal_id: "proposal-1".into(),
                skill_id: "skill://agent/proposal-1".into(),
                relative_path: PathBuf::from("SKILL.md"),
                content_digest: Some("skill-draft-digest://abc123".into()),
                planned_bytes: 128,
                bytes_written: 128,
                rollback_memento_ref: Some("skill-mutation-memento://abc123".into()),
                denied_reason: None,
                mutated: true,
                promoted: true,
                evidence_ids: vec!["evidence://operator/run".into()],
                policy_decision_refs: vec!["policy://operator/approved".into()],
                audit_event_ids: vec!["audit://operator/run".into()],
                trace_id: "trace-operator-result".into(),
                captured_at,
            }],
            denied: Vec::new(),
            mutated: true,
            captured_at,
        };

        let serialized = serde_json::to_string(&result).expect("operator result should serialize");
        assert!(serialized.contains("skill-draft-digest://abc123"));
        assert!(!serialized.contains("Inspect service-owned evidence"));
        assert!(!serialized.contains("SKILL.md body"));
    }

    #[test]
    fn autonomous_operator_snapshot_counts_statuses_without_skill_bodies() {
        let captured_at = chrono::Utc::now();
        let run = SkillAutonomousMaterializationRunResult {
            status: SkillAutonomousMaterializationStatus::Applied,
            run_id: "skill-materialization-operator-run-1".into(),
            trace_id: "trace-operator-snapshot".into(),
            dry_run: false,
            batch_limit: 1,
            selected_proposal_ids: vec!["proposal-ready-1".into()],
            processing: SkillProposalProcessingRunResult {
                run_id: "processing-run-1".into(),
                records: Vec::new(),
                state_counts: SkillProposalProcessingStateCounts::default(),
                duplicate_group_count: 0,
                mutated: true,
                captured_at,
            },
            materializations: Vec::new(),
            denied: Vec::new(),
            mutated: true,
            captured_at,
        };
        let snapshot = SkillAutonomousMaterializationSnapshotResult {
            status_counts: SkillAutonomousMaterializationStatusCounts::from_runs([&run]),
            last_run_ref: Some(run.run_id.clone()),
            recent_runs: vec![run],
            captured_at,
        };
        let serialized =
            serde_json::to_string(&snapshot).expect("operator snapshot should serialize");

        assert_eq!(snapshot.status_counts.applied, 1);
        assert!(serialized.contains("skill-materialization-operator-run-1"));
        assert!(!serialized.contains("generated_markdown"));
    }
}
