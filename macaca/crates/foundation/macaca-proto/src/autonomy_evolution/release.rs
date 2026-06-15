//! Provider-neutral DTOs for release safety decisions.
//!
//! These types intentionally contain only generic OS identities, bounded refs,
//! and sanitized policy facts. Release safety must stay reusable across Skill
//! packages, application capability packs, task policies, and future OS-code
//! proposal adapters without importing application-specific semantics.

use crate::TraceContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BenchmarkDecision, EvolutionRunState, EvolutionScope, EvolutionTargetType};

pub const AUTONOMY_EVOLUTION_RELEASE_COMMAND: &str = "autonomy.evolution.release.evaluate";

const MAX_REF_LEN: usize = 160;
const MAX_REFS: usize = 8;

/// Release action requested by a caller or higher-level control-plane loop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionReleaseAction {
    DryRun,
    Quarantine,
    StartCanary,
    Promote,
    StartActiveMonitoring,
    Rollback,
    Supersede,
    Reject,
    MarkInconclusive,
}

/// Bounded release status returned by the safety Strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionReleaseStatus {
    DryRunAccepted,
    Quarantined,
    CanaryRunning,
    Promoted,
    ActiveMonitoring,
    RolledBack,
    Superseded,
    Rejected,
    Inconclusive,
    Denied,
}

impl EvolutionReleaseStatus {
    /// Map release status back to the canonical run lifecycle state.
    pub fn run_state(&self) -> EvolutionRunState {
        match self {
            Self::DryRunAccepted => EvolutionRunState::Quarantined,
            Self::Quarantined => EvolutionRunState::Quarantined,
            Self::CanaryRunning => EvolutionRunState::CanaryRunning,
            Self::Promoted => EvolutionRunState::Promoted,
            Self::ActiveMonitoring => EvolutionRunState::ActiveMonitoring,
            Self::RolledBack => EvolutionRunState::RolledBack,
            Self::Superseded => EvolutionRunState::Superseded,
            Self::Rejected => EvolutionRunState::Rejected,
            Self::Inconclusive => EvolutionRunState::Inconclusive,
            Self::Denied => EvolutionRunState::Rejected,
        }
    }
}

/// Coarse trust level supplied by policy and entitlement layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionTrustLevel {
    Untrusted,
    Limited,
    Trusted,
}

/// Replayable rollback memento metadata.
///
/// The state ref points to Store/EventLog or target-service governance records.
/// It is intentionally a reference instead of embedded target bytes so the
/// release service can remain sanitized and provider-neutral.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionRollbackMemento {
    pub memento_id: String,
    pub target_type: EvolutionTargetType,
    pub scope: EvolutionScope,
    pub state_ref: String,
    pub evidence_refs: Vec<String>,
}

/// Provider-neutral release policy input for one candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionReleasePolicyInput {
    pub capability_diff_refs: Vec<String>,
    pub package_ownership_refs: Vec<String>,
    pub resource_permission_refs: Vec<String>,
    pub benchmark_decision: BenchmarkDecision,
    pub benchmark_refs: Vec<String>,
    pub rollback_mementos: Vec<EvolutionRollbackMemento>,
    pub trust_level: EvolutionTrustLevel,
    pub executable_change: bool,
    pub blast_radius_score: u32,
    pub canary_failure_reason_codes: Vec<String>,
}

/// Typed release command accepted by the Autonomy Evolution service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionReleaseCommand {
    pub release_id: String,
    pub run_id: String,
    pub actor_id: String,
    pub target_type: EvolutionTargetType,
    pub action: EvolutionReleaseAction,
    pub trace: TraceContext,
    pub scope: EvolutionScope,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub policy: EvolutionReleasePolicyInput,
    pub dry_run: bool,
}

/// Structured finding for one release safety condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionReleaseFinding {
    pub gate: String,
    pub accepted: bool,
    pub reason_code: String,
    pub evidence_refs: Vec<String>,
}

/// Result returned by release safety providers and SDK clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionReleaseResult {
    pub release_id: String,
    pub run_id: String,
    pub target_type: EvolutionTargetType,
    pub action: EvolutionReleaseAction,
    pub accepted: bool,
    pub status: EvolutionReleaseStatus,
    pub trace: TraceContext,
    pub findings: Vec<EvolutionReleaseFinding>,
    pub reason_codes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub adapter_dispatch_required: Option<bool>,
    pub captured_at: DateTime<Utc>,
}

impl EvolutionReleaseResult {
    /// Build a fail-closed result when the service is absent.
    pub fn unavailable(command: &EvolutionReleaseCommand, reason: impl Into<String>) -> Self {
        Self {
            release_id: command.release_id.clone(),
            run_id: command.run_id.clone(),
            target_type: command.target_type.clone(),
            action: command.action.clone(),
            accepted: false,
            status: EvolutionReleaseStatus::Denied,
            trace: command.trace.clone(),
            findings: Vec::new(),
            reason_codes: vec![bounded_reason(reason.into())],
            evidence_refs: bounded_refs(&command.evidence_refs),
            policy_decision_refs: bounded_refs(&command.policy_decision_refs),
            audit_refs: bounded_refs(&command.audit_refs),
            rollback_refs: Vec::new(),
            adapter_dispatch_required: Some(false),
            captured_at: Utc::now(),
        }
    }
}

fn bounded_refs(refs: &[String]) -> Vec<String> {
    refs.iter()
        .take(MAX_REFS)
        .map(|value| {
            if value.len() > MAX_REF_LEN {
                let mut bounded = value.clone();
                bounded.truncate(MAX_REF_LEN);
                bounded
            } else {
                value.clone()
            }
        })
        .collect()
}

fn bounded_reason(reason: String) -> String {
    if reason.len() > MAX_REF_LEN {
        let mut bounded = reason;
        bounded.truncate(MAX_REF_LEN);
        bounded
    } else {
        reason
    }
}
