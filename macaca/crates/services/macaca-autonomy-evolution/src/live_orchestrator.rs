//! Live orchestrator Strategies for autonomy evolution.
//!
//! The default implementation composes existing service-owned gates instead of
//! duplicating their semantics. It validates lease and idempotency evidence,
//! runs admission, paired benchmark scoring, release safety, and target adapter
//! dispatch checks, then returns bounded checkpoints that can be persisted or
//! replayed by a provider. It never writes Skill files or OS source code.

use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::{
    bounded_values, AdmissionDecision, BenchmarkDecision, DefaultEvolutionAdmissionSpecification,
    DefaultEvolutionBenchmarkScoringStrategy, DefaultEvolutionReleaseSafetyStrategy,
    EvolutionAdmissionCommand, EvolutionAdmissionSpecification, EvolutionBenchmarkCommand,
    EvolutionBenchmarkScoringStrategy, EvolutionLivePhase, EvolutionLivePhaseCheckpoint,
    EvolutionLivePhaseStatus, EvolutionLiveTickCommand, EvolutionLiveTickResult,
    EvolutionReleaseCommand, EvolutionReleaseResult, EvolutionReleaseSafetyStrategy,
    EvolutionTargetType, AUTONOMY_EVOLUTION_SERVICE_ID,
};

/// Strategy for turning observer refs into candidate descriptors.
///
/// The first implementation receives the descriptor in the tick command. Future
/// providers can replace discovery without changing the live tick contract.
pub trait EvolutionCandidateDiscoveryStrategy: Send + Sync {
    fn discover(
        &self,
        command: &EvolutionLiveTickCommand,
    ) -> MacacaResult<EvolutionLivePhaseCheckpoint>;
}

/// Strategy for collecting paired measurements before scoring.
pub trait EvolutionBenchmarkWorkloadRunner: Send + Sync {
    fn collect(
        &self,
        command: &EvolutionLiveTickCommand,
    ) -> MacacaResult<EvolutionBenchmarkCommand>;
}

/// Strategy for dispatching target-specific apply or rollback intents.
pub trait EvolutionTargetDispatchStrategy: Send + Sync {
    fn dispatch(
        &self,
        command: &EvolutionLiveTickCommand,
        release: &EvolutionReleaseResult,
    ) -> EvolutionLivePhaseCheckpoint;
}

/// Strategy for executing one live orchestrator tick.
pub trait EvolutionLiveOrchestrator: Send + Sync {
    fn run_tick(
        &self,
        command: EvolutionLiveTickCommand,
        sequence: u64,
    ) -> MacacaResult<EvolutionLiveTickResult>;
}

/// Conservative built-in orchestrator used by the local provider.
#[derive(Debug, Default, Clone)]
pub struct DefaultEvolutionLiveOrchestrator;

impl EvolutionLiveOrchestrator for DefaultEvolutionLiveOrchestrator {
    fn run_tick(
        &self,
        command: EvolutionLiveTickCommand,
        sequence: u64,
    ) -> MacacaResult<EvolutionLiveTickResult> {
        validate_live_tick(&command)?;
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            run_id = command.run_id.as_str(),
            lease_id = command.lease_id.as_str(),
            idempotency_key = command.idempotency_key.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "autonomy evolution live tick started"
        );

        let mut phases = Vec::new();
        phases.push(DefaultCandidateDiscovery.discover(&command)?);
        phases.push(EvolutionLivePhaseCheckpoint::new(
            EvolutionLivePhase::Transition,
            EvolutionLivePhaseStatus::Accepted,
            true,
            vec!["control_plane_transition_checkpoint_recorded".into()],
            command.observer_evidence_refs.clone(),
        ));

        let admission =
            DefaultEvolutionAdmissionSpecification.evaluate(&EvolutionAdmissionCommand {
                actor_id: command.actor_id.clone(),
                trace: command.trace.clone(),
                scope: command.scope.clone(),
                candidate: command.candidate.clone(),
                evidence_refs: command.observer_evidence_refs.clone(),
                policy_decision_refs: command.policy_decision_refs.clone(),
            })?;
        let admission_accepted = admission.decision == AdmissionDecision::Accepted;
        phases.push(EvolutionLivePhaseCheckpoint::new(
            EvolutionLivePhase::Admission,
            if admission_accepted {
                EvolutionLivePhaseStatus::Accepted
            } else {
                EvolutionLivePhaseStatus::Denied
            },
            admission_accepted,
            admission
                .findings
                .iter()
                .map(|finding| finding.reason_code.clone())
                .collect(),
            admission
                .findings
                .iter()
                .flat_map(|finding| finding.evidence_refs.clone())
                .collect(),
        ));
        if !admission_accepted {
            warn!(
                service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
                run_id = command.run_id.as_str(),
                trace_id = command.trace.trace_id.as_str(),
                "autonomy evolution live tick stopped at admission"
            );
            return Ok(final_result(
                command,
                sequence,
                false,
                EvolutionLivePhaseStatus::Denied,
                phases,
                vec!["admission_not_accepted".into()],
                false,
            ));
        }

        let benchmark_command = DefaultBenchmarkWorkloadRunner.collect(&command)?;
        let benchmark = DefaultEvolutionBenchmarkScoringStrategy.score(&benchmark_command)?;
        let benchmark_accepted = benchmark.decision == BenchmarkDecision::Passed;
        let benchmark_status = match benchmark.decision {
            BenchmarkDecision::Passed => EvolutionLivePhaseStatus::Accepted,
            BenchmarkDecision::Failed => EvolutionLivePhaseStatus::Rejected,
            BenchmarkDecision::Inconclusive => EvolutionLivePhaseStatus::Inconclusive,
        };
        phases.push(EvolutionLivePhaseCheckpoint::new(
            EvolutionLivePhase::Benchmark,
            benchmark_status.clone(),
            benchmark_accepted,
            benchmark.reason_codes.clone(),
            benchmark.evidence_refs.clone(),
        ));
        if !benchmark_accepted {
            return Ok(final_result(
                command,
                sequence,
                false,
                benchmark_status,
                phases,
                vec!["benchmark_not_passed".into()],
                false,
            ));
        }

        let release = DefaultEvolutionReleaseSafetyStrategy.evaluate(&EvolutionReleaseCommand {
            release_id: format!("release-{}-{}", command.run_id, command.idempotency_key),
            run_id: command.run_id.clone(),
            actor_id: command.actor_id.clone(),
            target_type: command.candidate.target_type.clone(),
            action: command.release_action.clone(),
            trace: command.trace.clone(),
            scope: command.scope.clone(),
            policy_decision_refs: command.policy_decision_refs.clone(),
            audit_refs: command.audit_refs.clone(),
            evidence_refs: benchmark.evidence_refs.clone(),
            policy: command.release_policy.clone(),
            dry_run: false,
        })?;
        let release_status = release_status(&release);
        phases.push(EvolutionLivePhaseCheckpoint::new(
            EvolutionLivePhase::ReleaseSafety,
            release_status.clone(),
            release.accepted,
            release.reason_codes.clone(),
            release.evidence_refs.clone(),
        ));
        if !release.accepted {
            return Ok(final_result(
                command,
                sequence,
                false,
                EvolutionLivePhaseStatus::Denied,
                phases,
                vec!["release_safety_denied".into()],
                false,
            ));
        }

        let dispatch = DefaultTargetDispatch.dispatch(&command, &release);
        let dispatch_accepted = dispatch.accepted;
        let dispatch_status = dispatch.status.clone();
        let dispatch_reasons = dispatch.reason_codes.clone();
        phases.push(dispatch);
        if !dispatch_accepted {
            return Ok(final_result(
                command,
                sequence,
                false,
                dispatch_status,
                phases,
                dispatch_reasons,
                false,
            ));
        }

        Ok(final_result(
            command,
            sequence,
            true,
            release_status,
            phases,
            vec!["live_tick_completed".into()],
            true,
        ))
    }
}

#[derive(Debug, Default, Clone)]
struct DefaultCandidateDiscovery;

impl EvolutionCandidateDiscoveryStrategy for DefaultCandidateDiscovery {
    fn discover(
        &self,
        command: &EvolutionLiveTickCommand,
    ) -> MacacaResult<EvolutionLivePhaseCheckpoint> {
        Ok(EvolutionLivePhaseCheckpoint::new(
            EvolutionLivePhase::Discovery,
            EvolutionLivePhaseStatus::Accepted,
            true,
            vec!["bounded_candidate_descriptor_present".into()],
            command.observer_evidence_refs.clone(),
        ))
    }
}

#[derive(Debug, Default, Clone)]
struct DefaultBenchmarkWorkloadRunner;

impl EvolutionBenchmarkWorkloadRunner for DefaultBenchmarkWorkloadRunner {
    fn collect(
        &self,
        command: &EvolutionLiveTickCommand,
    ) -> MacacaResult<EvolutionBenchmarkCommand> {
        Ok(EvolutionBenchmarkCommand {
            benchmark_id: format!("benchmark-{}-{}", command.run_id, command.idempotency_key),
            run_id: command.run_id.clone(),
            actor_id: command.actor_id.clone(),
            trace: command.trace.clone(),
            scope: command.scope.clone(),
            baseline: command.baseline.clone(),
            candidate: command.candidate_measurement.clone(),
            policy_decision_refs: command.policy_decision_refs.clone(),
        })
    }
}

#[derive(Debug, Default, Clone)]
struct DefaultTargetDispatch;

impl EvolutionTargetDispatchStrategy for DefaultTargetDispatch {
    fn dispatch(
        &self,
        command: &EvolutionLiveTickCommand,
        release: &EvolutionReleaseResult,
    ) -> EvolutionLivePhaseCheckpoint {
        if !command.adapter_dispatch.supported {
            return EvolutionLivePhaseCheckpoint::new(
                EvolutionLivePhase::TargetDispatch,
                EvolutionLivePhaseStatus::Unavailable,
                false,
                vec!["target_adapter_unavailable".into()],
                command.adapter_dispatch.evidence_refs.clone(),
            );
        }
        if command.candidate.target_type == EvolutionTargetType::OsCodeProposal {
            return EvolutionLivePhaseCheckpoint::new(
                EvolutionLivePhase::TargetDispatch,
                release_status(release),
                true,
                vec!["os_code_proposal_adapter_non_mutating".into()],
                command.adapter_dispatch.evidence_refs.clone(),
            );
        }
        EvolutionLivePhaseCheckpoint::new(
            EvolutionLivePhase::TargetDispatch,
            release_status(release),
            true,
            vec!["target_adapter_dispatch_recorded".into()],
            command.adapter_dispatch.evidence_refs.clone(),
        )
    }
}

fn validate_live_tick(command: &EvolutionLiveTickCommand) -> MacacaResult<()> {
    if command.run_id.trim().is_empty() {
        return Err(MacacaError::Config("live tick requires run_id".into()));
    }
    if command.actor_id.trim().is_empty() {
        return Err(MacacaError::Config("live tick requires actor_id".into()));
    }
    if command.lease_id.trim().is_empty() {
        return Err(MacacaError::Config("live tick requires lease_id".into()));
    }
    if command.idempotency_key.trim().is_empty() {
        return Err(MacacaError::Config(
            "live tick requires idempotency_key".into(),
        ));
    }
    if command.trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config("live tick requires trace_id".into()));
    }
    if command.observer_evidence_refs.is_empty() {
        return Err(MacacaError::Config(
            "live tick requires observer evidence refs".into(),
        ));
    }
    if command.policy_decision_refs.is_empty() {
        return Err(MacacaError::Config(
            "live tick requires policy decision refs".into(),
        ));
    }
    if command.audit_refs.is_empty() {
        return Err(MacacaError::Config("live tick requires audit refs".into()));
    }
    Ok(())
}

fn release_status(release: &EvolutionReleaseResult) -> EvolutionLivePhaseStatus {
    use crate::EvolutionReleaseStatus;
    match release.status {
        EvolutionReleaseStatus::CanaryRunning => EvolutionLivePhaseStatus::CanaryRunning,
        EvolutionReleaseStatus::Promoted => EvolutionLivePhaseStatus::Promoted,
        EvolutionReleaseStatus::RolledBack => EvolutionLivePhaseStatus::RolledBack,
        EvolutionReleaseStatus::Rejected => EvolutionLivePhaseStatus::Rejected,
        EvolutionReleaseStatus::Inconclusive => EvolutionLivePhaseStatus::Inconclusive,
        EvolutionReleaseStatus::Denied => EvolutionLivePhaseStatus::Denied,
        _ => EvolutionLivePhaseStatus::Accepted,
    }
}

fn final_result(
    command: EvolutionLiveTickCommand,
    sequence: u64,
    accepted: bool,
    status: EvolutionLivePhaseStatus,
    phases: Vec<EvolutionLivePhaseCheckpoint>,
    reason_codes: Vec<String>,
    adapter_dispatch_performed: bool,
) -> EvolutionLiveTickResult {
    info!(
        service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
        run_id = command.run_id.as_str(),
        trace_id = command.trace.trace_id.as_str(),
        status = ?status,
        accepted,
        "autonomy evolution live tick completed"
    );
    EvolutionLiveTickResult {
        service_id: AUTONOMY_EVOLUTION_SERVICE_ID.into(),
        run_id: command.run_id,
        target_type: command.candidate.target_type,
        idempotency_key: command.idempotency_key,
        lease_id: command.lease_id,
        sequence,
        accepted,
        status,
        trace: command.trace,
        phases,
        reason_codes: bounded_values(&reason_codes),
        evidence_refs: bounded_values(&command.observer_evidence_refs),
        policy_decision_refs: bounded_values(&command.policy_decision_refs),
        audit_refs: bounded_values(&command.audit_refs),
        rollback_refs: bounded_values(&command.adapter_dispatch.rollback_refs),
        adapter_dispatch_performed,
        source_mutation_performed: false,
        captured_at: chrono::Utc::now(),
    }
}
