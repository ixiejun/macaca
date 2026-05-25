//! State machine rules for evolution run transitions.
//!
//! Keeping the rules in one module applies the State and Specification patterns:
//! providers validate lifecycle movement with deterministic predicates before
//! any target adapter can run. This prevents Web, CLI, or target-specific code
//! from inventing their own promotion or rollback semantics.

use macaca_proto::{MacacaError, MacacaResult};

use crate::{EvolutionRunState, EvolutionRunTransition, EvolutionTransitionCommand};

/// Body-free validation output used by providers before mutating read models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDecision {
    pub next_state: EvolutionRunState,
    pub adapter_dispatch_required: bool,
}

/// Validate one transition against the current state and command evidence.
pub fn validate_transition(
    current_state: Option<&EvolutionRunState>,
    command: &EvolutionTransitionCommand,
) -> MacacaResult<TransitionDecision> {
    validate_command_shape(command)?;
    let current = current_state
        .cloned()
        .or_else(|| command.from_state.clone())
        .unwrap_or(EvolutionRunState::Observed);
    let next = next_state(&current, &command.transition).ok_or_else(|| {
        MacacaError::Config(format!(
            "invalid transition from {:?} using {:?}",
            current, command.transition
        ))
    })?;
    if requires_policy_and_audit(&command.transition) {
        require_policy_and_audit(command)?;
    }
    Ok(TransitionDecision {
        adapter_dispatch_required: adapter_dispatch_required(&command.transition),
        next_state: next,
    })
}

fn validate_command_shape(command: &EvolutionTransitionCommand) -> MacacaResult<()> {
    if command.run_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution transition requires run_id".into(),
        ));
    }
    if command.actor_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution transition requires actor_id".into(),
        ));
    }
    if command.trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution transition requires trace_id".into(),
        ));
    }
    if command.evidence_refs.is_empty() {
        return Err(MacacaError::Config(
            "evolution transition requires bounded evidence refs".into(),
        ));
    }
    Ok(())
}

fn require_policy_and_audit(command: &EvolutionTransitionCommand) -> MacacaResult<()> {
    if command.policy_decision_refs.is_empty() {
        return Err(MacacaError::Config(
            "side-effecting evolution transition requires policy decision refs".into(),
        ));
    }
    if command.audit_refs.is_empty() {
        return Err(MacacaError::Config(
            "side-effecting evolution transition requires audit refs".into(),
        ));
    }
    if command.transition == EvolutionRunTransition::Rollback && command.rollback_refs.is_empty() {
        return Err(MacacaError::Config(
            "rollback transition requires rollback memento refs".into(),
        ));
    }
    Ok(())
}

fn next_state(
    current: &EvolutionRunState,
    transition: &EvolutionRunTransition,
) -> Option<EvolutionRunState> {
    use EvolutionRunState::*;
    use EvolutionRunTransition::*;

    match (current, transition) {
        (Observed, Observe) => Some(Observed),
        (Observed, QueueCandidate) => Some(CandidateQueued),
        (CandidateQueued, ClassifyCandidate) => Some(CandidateClassified),
        (CandidateClassified, GenerateProposal) => Some(ProposalGenerated),
        (ProposalGenerated, StartAdmissionReview) => Some(AdmissionReview),
        (AdmissionReview, Quarantine) => Some(Quarantined),
        (Quarantined, PrepareBenchmark) => Some(BenchmarkPrepared),
        (BenchmarkPrepared, RecordBaseline) => Some(BaselineMeasured),
        (BaselineMeasured, RecordCandidateMeasurement) => Some(CandidateMeasured),
        (CandidateMeasured, StartCanary) => Some(CanaryRunning),
        (CanaryRunning, Promote) => Some(Promoted),
        (Promoted, StartActiveMonitoring) => Some(ActiveMonitoring),
        (ActiveMonitoring, Supersede) => Some(Superseded),
        (ActiveMonitoring, Rollback) => Some(RolledBack),
        (_, Reject) => Some(Rejected),
        (_, MarkInconclusive) => Some(Inconclusive),
        _ => None,
    }
}

fn requires_policy_and_audit(transition: &EvolutionRunTransition) -> bool {
    matches!(
        transition,
        EvolutionRunTransition::Promote
            | EvolutionRunTransition::Rollback
            | EvolutionRunTransition::Supersede
    )
}

fn adapter_dispatch_required(transition: &EvolutionRunTransition) -> bool {
    matches!(
        transition,
        EvolutionRunTransition::Promote
            | EvolutionRunTransition::Rollback
            | EvolutionRunTransition::Supersede
    )
}
