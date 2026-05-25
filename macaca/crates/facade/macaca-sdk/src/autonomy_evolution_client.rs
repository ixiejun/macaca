//! SDK client for the Autonomy Evolution Control Plane.
//!
//! This module is a thin Facade over `SystemServiceClient`. It deliberately
//! does not classify candidates, score benchmarks, apply rollouts, or mutate
//! target packages. Those semantics remain service-owned and provider-owned;
//! SDK callers only submit typed commands and receive body-free results.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_autonomy_evolution::{
    EvolutionAdmissionCommand, EvolutionAdmissionResult, EvolutionBenchmarkCommand,
    EvolutionBenchmarkResult, EvolutionLiveAuditCommand, EvolutionLiveAuditResult,
    EvolutionLiveTickCommand, EvolutionLiveTickResult, EvolutionReleaseCommand,
    EvolutionReleaseResult, EvolutionRunState, EvolutionServiceSnapshot, EvolutionSnapshotCommand,
    EvolutionTransitionCommand, EvolutionTransitionResult, OsCodeEvolutionProposalCommand,
    OsCodeEvolutionProposalResult, AUTONOMY_EVOLUTION_ADMISSION_COMMAND,
    AUTONOMY_EVOLUTION_BENCHMARK_COMMAND, AUTONOMY_EVOLUTION_HEALTH_COMMAND,
    AUTONOMY_EVOLUTION_LIVE_AUDIT_COMMAND, AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND,
    AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND, AUTONOMY_EVOLUTION_RELEASE_COMMAND,
    AUTONOMY_EVOLUTION_SERVICE_ID, AUTONOMY_EVOLUTION_SNAPSHOT_COMMAND,
    AUTONOMY_EVOLUTION_TRANSITION_COMMAND,
};
use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused SDK boundary for autonomy evolution operations.
#[async_trait]
pub trait SystemAutonomyEvolutionClient: Send + Sync {
    async fn transition(
        &self,
        command: EvolutionTransitionCommand,
    ) -> MacacaResult<EvolutionTransitionResult>;

    async fn snapshot(
        &self,
        command: EvolutionSnapshotCommand,
    ) -> MacacaResult<EvolutionServiceSnapshot>;

    async fn health(&self, trace: TraceContext) -> MacacaResult<EvolutionServiceSnapshot>;

    async fn admit_candidate(
        &self,
        command: EvolutionAdmissionCommand,
    ) -> MacacaResult<EvolutionAdmissionResult>;

    async fn run_paired_benchmark(
        &self,
        command: EvolutionBenchmarkCommand,
    ) -> MacacaResult<EvolutionBenchmarkResult>;

    async fn evaluate_release(
        &self,
        command: EvolutionReleaseCommand,
    ) -> MacacaResult<EvolutionReleaseResult>;

    async fn evaluate_os_code_proposal(
        &self,
        command: OsCodeEvolutionProposalCommand,
    ) -> MacacaResult<OsCodeEvolutionProposalResult>;

    async fn run_live_tick(
        &self,
        command: EvolutionLiveTickCommand,
    ) -> MacacaResult<EvolutionLiveTickResult>;

    async fn audit_live_run(
        &self,
        command: EvolutionLiveAuditCommand,
    ) -> MacacaResult<EvolutionLiveAuditResult>;
}

/// Null Object client used when no control-plane service is installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemAutonomyEvolutionClient;

#[async_trait]
impl SystemAutonomyEvolutionClient for UnavailableSystemAutonomyEvolutionClient {
    async fn transition(
        &self,
        command: EvolutionTransitionCommand,
    ) -> MacacaResult<EvolutionTransitionResult> {
        warn!(
            trace_id = command.trace.trace_id.as_str(),
            run_id = command.run_id.as_str(),
            "sdk autonomy evolution client unavailable for transition"
        );
        Ok(EvolutionTransitionResult::denied(
            &command,
            command
                .from_state
                .clone()
                .unwrap_or(EvolutionRunState::Observed),
            "autonomy evolution control plane service is unavailable",
        ))
    }

    async fn snapshot(
        &self,
        command: EvolutionSnapshotCommand,
    ) -> MacacaResult<EvolutionServiceSnapshot> {
        info!(
            trace_id = command.trace.trace_id.as_str(),
            "sdk autonomy evolution client returning unavailable snapshot"
        );
        Ok(EvolutionServiceSnapshot::unavailable(
            "autonomy evolution control plane service is unavailable",
        ))
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<EvolutionServiceSnapshot> {
        info!(
            trace_id = trace.trace_id.as_str(),
            "sdk autonomy evolution client returning unavailable health"
        );
        Ok(EvolutionServiceSnapshot::unavailable(
            "autonomy evolution control plane service is unavailable",
        ))
    }

    async fn admit_candidate(
        &self,
        command: EvolutionAdmissionCommand,
    ) -> MacacaResult<EvolutionAdmissionResult> {
        warn!(
            trace_id = command.trace.trace_id.as_str(),
            candidate_id = command.candidate.candidate_id.as_str(),
            "sdk autonomy evolution client unavailable for candidate admission"
        );
        Ok(EvolutionAdmissionResult::unavailable(
            &command,
            "autonomy evolution control plane service is unavailable",
        ))
    }

    async fn run_paired_benchmark(
        &self,
        command: EvolutionBenchmarkCommand,
    ) -> MacacaResult<EvolutionBenchmarkResult> {
        warn!(
            trace_id = command.trace.trace_id.as_str(),
            benchmark_id = command.benchmark_id.as_str(),
            run_id = command.run_id.as_str(),
            "sdk autonomy evolution client unavailable for paired benchmark"
        );
        Ok(EvolutionBenchmarkResult::unavailable(
            &command,
            "autonomy evolution control plane service is unavailable",
        ))
    }

    async fn evaluate_release(
        &self,
        command: EvolutionReleaseCommand,
    ) -> MacacaResult<EvolutionReleaseResult> {
        warn!(
            trace_id = command.trace.trace_id.as_str(),
            release_id = command.release_id.as_str(),
            run_id = command.run_id.as_str(),
            "sdk autonomy evolution client unavailable for release safety"
        );
        Ok(EvolutionReleaseResult::unavailable(
            &command,
            "autonomy evolution control plane service is unavailable",
        ))
    }

    async fn evaluate_os_code_proposal(
        &self,
        command: OsCodeEvolutionProposalCommand,
    ) -> MacacaResult<OsCodeEvolutionProposalResult> {
        warn!(
            trace_id = command.trace.trace_id.as_str(),
            proposal_id = command.proposal_id.as_str(),
            "sdk autonomy evolution client unavailable for os-code proposal"
        );
        Ok(OsCodeEvolutionProposalResult {
            proposal_id: command.proposal_id,
            run_id: command.run_id,
            decision: macaca_autonomy_evolution::OsCodeEvolutionProposalDecision::Denied,
            trace: command.trace,
            bundle: macaca_autonomy_evolution::OsCodeEvolutionProposalBundle {
                proposal_ref: None,
                title: String::new(),
                summary: String::new(),
                affected_capability_refs: Vec::new(),
                requested_change_refs: Vec::new(),
                openspec_refs: Vec::new(),
                superpowers_refs: Vec::new(),
                gitnexus_refs: Vec::new(),
                expected_test_refs: Vec::new(),
                release_gate_refs: Vec::new(),
                rollback_refs: Vec::new(),
            },
            findings: Vec::new(),
            missing_evidence: Vec::new(),
            reason_codes: vec!["autonomy evolution control plane service is unavailable".into()],
            source_mutation_performed: false,
            policy_decision_refs: command.policy_decision_refs,
            audit_refs: command.audit_refs,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn run_live_tick(
        &self,
        command: EvolutionLiveTickCommand,
    ) -> MacacaResult<EvolutionLiveTickResult> {
        warn!(
            trace_id = command.trace.trace_id.as_str(),
            run_id = command.run_id.as_str(),
            idempotency_key = command.idempotency_key.as_str(),
            "sdk autonomy evolution client unavailable for live tick"
        );
        Ok(EvolutionLiveTickResult::unavailable(
            &command,
            "autonomy evolution control plane service is unavailable",
        ))
    }

    async fn audit_live_run(
        &self,
        command: EvolutionLiveAuditCommand,
    ) -> MacacaResult<EvolutionLiveAuditResult> {
        info!(
            trace_id = command.trace.trace_id.as_str(),
            run_id = command.run_id.as_str(),
            "sdk autonomy evolution client returning unavailable live audit"
        );
        Ok(EvolutionLiveAuditResult::unavailable(
            &command,
            "autonomy evolution control plane service is unavailable",
        ))
    }
}

/// Runtime-backed client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedAutonomyEvolutionClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedAutonomyEvolutionClient {
    /// Create a service-backed client without constructing any provider.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemAutonomyEvolutionClient for ServiceBackedAutonomyEvolutionClient {
    async fn transition(
        &self,
        command: EvolutionTransitionCommand,
    ) -> MacacaResult<EvolutionTransitionResult> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_TRANSITION_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: EvolutionSnapshotCommand,
    ) -> MacacaResult<EvolutionServiceSnapshot> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<EvolutionServiceSnapshot> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_HEALTH_COMMAND,
            trace.clone(),
            serde_json::json!({}),
        )
        .await
    }

    async fn admit_candidate(
        &self,
        command: EvolutionAdmissionCommand,
    ) -> MacacaResult<EvolutionAdmissionResult> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_ADMISSION_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn run_paired_benchmark(
        &self,
        command: EvolutionBenchmarkCommand,
    ) -> MacacaResult<EvolutionBenchmarkResult> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_BENCHMARK_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn evaluate_release(
        &self,
        command: EvolutionReleaseCommand,
    ) -> MacacaResult<EvolutionReleaseResult> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_RELEASE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn evaluate_os_code_proposal(
        &self,
        command: OsCodeEvolutionProposalCommand,
    ) -> MacacaResult<OsCodeEvolutionProposalResult> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn run_live_tick(
        &self,
        command: EvolutionLiveTickCommand,
    ) -> MacacaResult<EvolutionLiveTickResult> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn audit_live_run(
        &self,
        command: EvolutionLiveAuditCommand,
    ) -> MacacaResult<EvolutionLiveAuditResult> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_LIVE_AUDIT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let command = ServiceCallCommand::new(
        AUTONOMY_EVOLUTION_SERVICE_ID,
        command_name,
        serde_json::to_value(payload).map_err(|error| MacacaError::Config(error.to_string()))?,
    )?
    .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(|error| MacacaError::Config(error.to_string()))
}
