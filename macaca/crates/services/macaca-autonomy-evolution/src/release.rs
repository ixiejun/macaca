//! Release safety Strategy for autonomy evolution candidates.
//!
//! This module owns only generic release policy semantics. It never mutates a
//! target package, opens a Skill body, reads an application manifest, or applies
//! source-code changes. Instead, it evaluates provider-neutral evidence refs and
//! rollback mementos, then returns a deterministic release decision that a
//! target adapter can execute later through its own service boundary.

use chrono::Utc;
use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::{
    BenchmarkDecision, EvolutionReleaseAction, EvolutionReleaseCommand, EvolutionReleaseFinding,
    EvolutionReleaseResult, EvolutionReleaseStatus, EvolutionTrustLevel,
    AUTONOMY_EVOLUTION_SERVICE_ID,
};

const MAX_REASON_CODES: usize = 10;
const MAX_FINDINGS: usize = 12;
const MAX_REF_LEN: usize = 160;
const MAX_REFS: usize = 8;
const MAX_CANARY_FAILURE_CODES: usize = 4;
const DEFAULT_BLAST_RADIUS_LIMIT: u32 = 80;

/// Replaceable Strategy for release safety decisions.
pub trait EvolutionReleaseSafetyStrategy: Send + Sync {
    fn evaluate(&self, command: &EvolutionReleaseCommand) -> MacacaResult<EvolutionReleaseResult>;
}

/// Conservative default Strategy for the first release-safety slice.
#[derive(Debug, Default, Clone)]
pub struct DefaultEvolutionReleaseSafetyStrategy;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateOutcome {
    gate: &'static str,
    accepted: bool,
    reason_code: &'static str,
    evidence_refs: Vec<String>,
}

impl EvolutionReleaseSafetyStrategy for DefaultEvolutionReleaseSafetyStrategy {
    fn evaluate(&self, command: &EvolutionReleaseCommand) -> MacacaResult<EvolutionReleaseResult> {
        validate_command_shape(command)?;
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            release_id = command.release_id.as_str(),
            run_id = command.run_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            action = ?command.action,
            dry_run = command.dry_run,
            "evolution release safety evaluation started"
        );

        let outcomes = gate_outcomes(command);
        let denied = outcomes.iter().any(|outcome| !outcome.accepted);
        let status = if command.dry_run || command.action == EvolutionReleaseAction::DryRun {
            EvolutionReleaseStatus::DryRunAccepted
        } else if denied {
            EvolutionReleaseStatus::Denied
        } else {
            status_for_action(&command.action)
        };
        let accepted = !denied;

        if !accepted {
            warn!(
                service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
                release_id = command.release_id.as_str(),
                run_id = command.run_id.as_str(),
                trace_id = command.trace.trace_id.as_str(),
                "evolution release safety evaluation denied action"
            );
        }

        Ok(result(command, accepted, status, outcomes))
    }
}

fn validate_command_shape(command: &EvolutionReleaseCommand) -> MacacaResult<()> {
    if command.release_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution release requires release_id".into(),
        ));
    }
    if command.run_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution release requires run_id".into(),
        ));
    }
    if command.actor_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution release requires actor_id".into(),
        ));
    }
    if command.trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution release requires trace_id".into(),
        ));
    }
    if command.policy_decision_refs.is_empty() {
        return Err(MacacaError::Config(
            "evolution release requires policy decision refs".into(),
        ));
    }
    if command.audit_refs.is_empty() {
        return Err(MacacaError::Config(
            "evolution release requires audit refs".into(),
        ));
    }
    if command.evidence_refs.is_empty() {
        return Err(MacacaError::Config(
            "evolution release requires evidence refs".into(),
        ));
    }
    Ok(())
}

fn gate_outcomes(command: &EvolutionReleaseCommand) -> Vec<GateOutcome> {
    vec![
        non_empty_refs(
            "capability_diff",
            "capability_diff_refs_present",
            "capability_diff_missing",
            &command.policy.capability_diff_refs,
        ),
        non_empty_refs(
            "package_ownership",
            "package_ownership_refs_present",
            "package_ownership_missing",
            &command.policy.package_ownership_refs,
        ),
        non_empty_refs(
            "resource_permission",
            "resource_permission_refs_present",
            "resource_permission_missing",
            &command.policy.resource_permission_refs,
        ),
        benchmark_gate(command),
        trust_gate(command),
        blast_radius_gate(command),
        rollback_memento_gate(command),
    ]
}

fn non_empty_refs(
    gate: &'static str,
    accepted_code: &'static str,
    denied_code: &'static str,
    refs: &[String],
) -> GateOutcome {
    if refs.is_empty() {
        GateOutcome {
            gate,
            accepted: false,
            reason_code: denied_code,
            evidence_refs: Vec::new(),
        }
    } else {
        GateOutcome {
            gate,
            accepted: true,
            reason_code: accepted_code,
            evidence_refs: bounded_refs(refs),
        }
    }
}

fn benchmark_gate(command: &EvolutionReleaseCommand) -> GateOutcome {
    let accepted = matches!(
        command.policy.benchmark_decision,
        BenchmarkDecision::Passed | BenchmarkDecision::Inconclusive
    );
    GateOutcome {
        gate: "benchmark_decision",
        accepted,
        reason_code: if accepted {
            "benchmark_release_eligible"
        } else {
            "benchmark_failed"
        },
        evidence_refs: bounded_refs(&command.policy.benchmark_refs),
    }
}

fn trust_gate(command: &EvolutionReleaseCommand) -> GateOutcome {
    let accepted = command.policy.trust_level != EvolutionTrustLevel::Untrusted;
    GateOutcome {
        gate: "trust_level",
        accepted,
        reason_code: if accepted {
            "trust_level_release_eligible"
        } else {
            "trust_level_untrusted"
        },
        evidence_refs: Vec::new(),
    }
}

fn blast_radius_gate(command: &EvolutionReleaseCommand) -> GateOutcome {
    let accepted = !command.policy.executable_change
        || command.policy.blast_radius_score <= DEFAULT_BLAST_RADIUS_LIMIT;
    GateOutcome {
        gate: "blast_radius",
        accepted,
        reason_code: if accepted {
            "blast_radius_within_release_threshold"
        } else {
            "blast_radius_exceeds_release_threshold"
        },
        evidence_refs: Vec::new(),
    }
}

fn rollback_memento_gate(command: &EvolutionReleaseCommand) -> GateOutcome {
    if !requires_rollback_memento(&command.action) && command.dry_run {
        return GateOutcome {
            gate: "rollback_memento",
            accepted: true,
            reason_code: "rollback_memento_not_required_for_dry_run",
            evidence_refs: Vec::new(),
        };
    }
    let matching = command
        .policy
        .rollback_mementos
        .iter()
        .filter(|memento| {
            memento.target_type == command.target_type
                && memento.scope == command.scope
                && !memento.state_ref.trim().is_empty()
        })
        .collect::<Vec<_>>();
    let accepted = if requires_rollback_memento(&command.action) {
        !matching.is_empty()
    } else {
        true
    };
    GateOutcome {
        gate: "rollback_memento",
        accepted,
        reason_code: if accepted {
            "rollback_memento_ready"
        } else {
            "rollback_memento_missing"
        },
        evidence_refs: matching
            .iter()
            .flat_map(|memento| memento.evidence_refs.iter().cloned())
            .take(MAX_REFS)
            .collect(),
    }
}

fn requires_rollback_memento(action: &EvolutionReleaseAction) -> bool {
    matches!(
        action,
        EvolutionReleaseAction::StartCanary
            | EvolutionReleaseAction::Promote
            | EvolutionReleaseAction::Rollback
            | EvolutionReleaseAction::Supersede
    )
}

fn status_for_action(action: &EvolutionReleaseAction) -> EvolutionReleaseStatus {
    match action {
        EvolutionReleaseAction::DryRun => EvolutionReleaseStatus::DryRunAccepted,
        EvolutionReleaseAction::Quarantine => EvolutionReleaseStatus::Quarantined,
        EvolutionReleaseAction::StartCanary => EvolutionReleaseStatus::CanaryRunning,
        EvolutionReleaseAction::Promote => EvolutionReleaseStatus::Promoted,
        EvolutionReleaseAction::StartActiveMonitoring => EvolutionReleaseStatus::ActiveMonitoring,
        EvolutionReleaseAction::Rollback => EvolutionReleaseStatus::RolledBack,
        EvolutionReleaseAction::Supersede => EvolutionReleaseStatus::Superseded,
        EvolutionReleaseAction::Reject => EvolutionReleaseStatus::Rejected,
        EvolutionReleaseAction::MarkInconclusive => EvolutionReleaseStatus::Inconclusive,
    }
}

fn result(
    command: &EvolutionReleaseCommand,
    accepted: bool,
    status: EvolutionReleaseStatus,
    outcomes: Vec<GateOutcome>,
) -> EvolutionReleaseResult {
    let mut reason_codes = outcomes
        .iter()
        .map(|outcome| outcome.reason_code.to_owned())
        .collect::<Vec<_>>();
    reason_codes.extend(
        command
            .policy
            .canary_failure_reason_codes
            .iter()
            .take(MAX_CANARY_FAILURE_CODES)
            .map(|code| bounded_reason(code.clone())),
    );
    let rollback_refs = command
        .policy
        .rollback_mementos
        .iter()
        .filter(|memento| {
            memento.target_type == command.target_type && memento.scope == command.scope
        })
        .map(|memento| memento.state_ref.clone())
        .filter(|value| !value.trim().is_empty())
        .take(MAX_REFS)
        .collect::<Vec<_>>();

    info!(
        service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
        release_id = command.release_id.as_str(),
        run_id = command.run_id.as_str(),
        accepted,
        status = ?status,
        "evolution release safety evaluation completed"
    );

    EvolutionReleaseResult {
        release_id: command.release_id.clone(),
        run_id: command.run_id.clone(),
        target_type: command.target_type.clone(),
        action: command.action.clone(),
        accepted,
        status,
        trace: command.trace.clone(),
        findings: outcomes
            .into_iter()
            .take(MAX_FINDINGS)
            .map(|outcome| EvolutionReleaseFinding {
                gate: outcome.gate.into(),
                accepted: outcome.accepted,
                reason_code: outcome.reason_code.into(),
                evidence_refs: bounded_refs(&outcome.evidence_refs),
            })
            .collect(),
        reason_codes: reason_codes.into_iter().take(MAX_REASON_CODES).collect(),
        evidence_refs: bounded_refs(&command.evidence_refs),
        policy_decision_refs: bounded_refs(&command.policy_decision_refs),
        audit_refs: bounded_refs(&command.audit_refs),
        rollback_refs: bounded_refs(&rollback_refs),
        adapter_dispatch_required: if command.dry_run
            || command.action == EvolutionReleaseAction::DryRun
        {
            None
        } else {
            Some(accepted && side_effecting_action(&command.action))
        },
        captured_at: Utc::now(),
    }
}

fn side_effecting_action(action: &EvolutionReleaseAction) -> bool {
    !matches!(
        action,
        EvolutionReleaseAction::DryRun | EvolutionReleaseAction::MarkInconclusive
    )
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
