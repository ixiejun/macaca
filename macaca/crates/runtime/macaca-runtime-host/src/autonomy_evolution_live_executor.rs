//! End-to-end runtime bridge for live autonomy evolution execution.
//!
//! The autonomy evolution service owns policy semantics, admission, benchmark
//! scoring, release safety, OS-code proposal checks, and live audit replay.
//! Runtime Host owns only the Adapter/Bridge role: it sequences typed service
//! commands through `ServiceRuntime`, dispatches to the target service selected
//! by `EvolutionTargetType`, and returns a bounded body-free result. The bridge
//! never branches on application names, workflow names, provider names, or
//! business domains.

use std::sync::Arc;

use macaca_autonomy_evolution::{
    EvolutionLiveAuditCommand, EvolutionLiveAuditResult, EvolutionLiveTickCommand,
    EvolutionLiveTickResult, EvolutionScope, EvolutionTargetType, OsCodeEvolutionProposalCommand,
    OsCodeEvolutionProposalDecision, OsCodeEvolutionProposalResult,
    AUTONOMY_EVOLUTION_LIVE_AUDIT_COMMAND, AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND,
    AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND, AUTONOMY_EVOLUTION_SERVICE_ID,
};
use macaca_proto::{
    KernelServiceId, MacacaError, MacacaResult, ServiceBusSource, ServiceCommand,
    ServiceCommandName, TraceContext,
};
use macaca_skill::{
    SkillAutonomousMaterializationRunCommand, SkillAutonomousMaterializationRunResult,
    SkillAutonomousMaterializationStatus, SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
    SKILL_SERVICE_ID,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::ServiceRuntime;

const EXECUTOR_SOURCE: &str = "runtime.autonomy_evolution_live_executor";

/// One runtime-owned command for closing a live self-evolution loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyEvolutionLiveExecutionCommand {
    pub trace: TraceContext,
    pub scope: EvolutionScope,
    pub live_tick: EvolutionLiveTickCommand,
    pub skill_materialization: Option<SkillAutonomousMaterializationRunCommand>,
    pub os_code_proposal: Option<OsCodeEvolutionProposalCommand>,
}

/// Bounded target adapter result returned by the execution bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyEvolutionTargetExecutionOutcome {
    pub target_type: EvolutionTargetType,
    pub command_name: Option<String>,
    pub accepted: bool,
    pub status: String,
    pub reason_codes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub source_mutation_performed: bool,
}

/// Aggregate result proving live tick, target adapter, and audit replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyEvolutionLiveExecutionResult {
    pub accepted: bool,
    pub live_tick: EvolutionLiveTickResult,
    pub target: AutonomyEvolutionTargetExecutionOutcome,
    pub audit: EvolutionLiveAuditResult,
}

/// Host-side Bridge that sequences service-owned evolution commands.
#[derive(Clone)]
pub struct AutonomyEvolutionLiveExecutor {
    runtime: Arc<ServiceRuntime>,
}

impl AutonomyEvolutionLiveExecutor {
    /// Create an executor from the approved runtime-host composition root.
    pub fn new(runtime: Arc<ServiceRuntime>) -> Self {
        Self { runtime }
    }

    /// Execute one full live self-evolution loop.
    ///
    /// The method intentionally calls the live audit command even when the
    /// target adapter fails. That makes failed and unsupported target dispatch
    /// replayable instead of leaving operators with only local logs.
    pub async fn execute(
        &self,
        command: AutonomyEvolutionLiveExecutionCommand,
    ) -> MacacaResult<AutonomyEvolutionLiveExecutionResult> {
        info!(
            trace_id = command.trace.trace_id.as_str(),
            run_id = command.live_tick.run_id.as_str(),
            target_type = ?command.live_tick.candidate.target_type,
            "autonomy evolution live execution started"
        );

        let live_tick = self.run_live_tick(&command.live_tick).await?;
        let target = if live_tick.accepted {
            self.dispatch_target(&command, &live_tick).await?
        } else {
            skipped_target(&live_tick, "live_tick_not_accepted")
        };
        let audit = self
            .audit_live_run(&command.trace, &command.scope, &live_tick.run_id)
            .await?;
        let accepted = live_tick.accepted && target.accepted && !audit.checkpoints.is_empty();

        info!(
            trace_id = command.trace.trace_id.as_str(),
            run_id = live_tick.run_id.as_str(),
            accepted,
            target_status = target.status.as_str(),
            audit_checkpoints = audit.checkpoints.len(),
            "autonomy evolution live execution completed"
        );
        Ok(AutonomyEvolutionLiveExecutionResult {
            accepted,
            live_tick,
            target,
            audit,
        })
    }

    async fn run_live_tick(
        &self,
        command: &EvolutionLiveTickCommand,
    ) -> MacacaResult<EvolutionLiveTickResult> {
        self.call(
            AUTONOMY_EVOLUTION_SERVICE_ID,
            AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn audit_live_run(
        &self,
        trace: &TraceContext,
        scope: &EvolutionScope,
        run_id: &str,
    ) -> MacacaResult<EvolutionLiveAuditResult> {
        let audit = EvolutionLiveAuditCommand {
            run_id: run_id.into(),
            trace: trace.clone(),
            scope: scope.clone(),
            after_sequence: None,
            limit: 100,
        };
        self.call(
            AUTONOMY_EVOLUTION_SERVICE_ID,
            AUTONOMY_EVOLUTION_LIVE_AUDIT_COMMAND,
            trace.clone(),
            &audit,
        )
        .await
    }

    async fn dispatch_target(
        &self,
        command: &AutonomyEvolutionLiveExecutionCommand,
        live_tick: &EvolutionLiveTickResult,
    ) -> MacacaResult<AutonomyEvolutionTargetExecutionOutcome> {
        match &live_tick.target_type {
            EvolutionTargetType::SkillPackage => self.dispatch_skill(command).await,
            EvolutionTargetType::OsCodeProposal => self.dispatch_os_code(command).await,
            other => {
                warn!(
                    trace_id = command.trace.trace_id.as_str(),
                    target_type = ?other,
                    "autonomy evolution target adapter unavailable"
                );
                Ok(AutonomyEvolutionTargetExecutionOutcome {
                    target_type: other.clone(),
                    command_name: None,
                    accepted: false,
                    status: "unavailable".into(),
                    reason_codes: vec!["target_adapter_unavailable".into()],
                    evidence_refs: Vec::new(),
                    source_mutation_performed: false,
                })
            }
        }
    }

    async fn dispatch_skill(
        &self,
        command: &AutonomyEvolutionLiveExecutionCommand,
    ) -> MacacaResult<AutonomyEvolutionTargetExecutionOutcome> {
        let Some(materialization) = command.skill_materialization.as_ref() else {
            return Ok(missing_target_command(
                EvolutionTargetType::SkillPackage,
                SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
            ));
        };
        let result: SkillAutonomousMaterializationRunResult = self
            .call(
                SKILL_SERVICE_ID,
                SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
                materialization.trace.clone(),
                materialization,
            )
            .await?;
        let accepted = matches!(
            result.status,
            SkillAutonomousMaterializationStatus::Applied
                | SkillAutonomousMaterializationStatus::Previewed
        );
        Ok(AutonomyEvolutionTargetExecutionOutcome {
            target_type: EvolutionTargetType::SkillPackage,
            command_name: Some(SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND.into()),
            accepted,
            status: format!("{:?}", result.status),
            reason_codes: result
                .denied
                .iter()
                .map(|denial| denial.reason.clone())
                .collect(),
            evidence_refs: result.selected_proposal_ids,
            source_mutation_performed: false,
        })
    }

    async fn dispatch_os_code(
        &self,
        command: &AutonomyEvolutionLiveExecutionCommand,
    ) -> MacacaResult<AutonomyEvolutionTargetExecutionOutcome> {
        let Some(proposal) = command.os_code_proposal.as_ref() else {
            return Ok(missing_target_command(
                EvolutionTargetType::OsCodeProposal,
                AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND,
            ));
        };
        let result: OsCodeEvolutionProposalResult = self
            .call(
                AUTONOMY_EVOLUTION_SERVICE_ID,
                AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND,
                proposal.trace.clone(),
                proposal,
            )
            .await?;
        let accepted = result.decision == OsCodeEvolutionProposalDecision::ReadyForReview;
        Ok(AutonomyEvolutionTargetExecutionOutcome {
            target_type: EvolutionTargetType::OsCodeProposal,
            command_name: Some(AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND.into()),
            accepted,
            status: format!("{:?}", result.decision),
            reason_codes: result.reason_codes,
            evidence_refs: result.bundle.rollback_refs,
            source_mutation_performed: result.source_mutation_performed,
        })
    }

    async fn call<T, R>(
        &self,
        service_id: &str,
        command_name: &str,
        trace: TraceContext,
        payload: &T,
    ) -> MacacaResult<R>
    where
        T: Serialize + ?Sized,
        R: serde::de::DeserializeOwned,
    {
        let service_id = KernelServiceId::new(service_id);
        let payload = serde_json::to_value(payload)?;
        let reply = self
            .runtime
            .call(
                &service_id,
                ServiceBusSource::new(EXECUTOR_SOURCE),
                ServiceCommand::with_trace(ServiceCommandName::new(command_name), payload, trace),
            )
            .await
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        if !reply.success {
            return Err(MacacaError::Config(format!(
                "service command {command_name} failed with status {}",
                reply.status
            )));
        }
        let output = reply
            .output
            .ok_or_else(|| MacacaError::Config("service reply missing output".into()))?;
        serde_json::from_value(output).map_err(MacacaError::from)
    }
}

fn skipped_target(
    live_tick: &EvolutionLiveTickResult,
    reason: &str,
) -> AutonomyEvolutionTargetExecutionOutcome {
    AutonomyEvolutionTargetExecutionOutcome {
        target_type: live_tick.target_type.clone(),
        command_name: None,
        accepted: false,
        status: "skipped".into(),
        reason_codes: vec![reason.into()],
        evidence_refs: Vec::new(),
        source_mutation_performed: false,
    }
}

fn missing_target_command(
    target_type: EvolutionTargetType,
    command_name: &str,
) -> AutonomyEvolutionTargetExecutionOutcome {
    AutonomyEvolutionTargetExecutionOutcome {
        target_type,
        command_name: Some(command_name.into()),
        accepted: false,
        status: "denied".into(),
        reason_codes: vec!["target_adapter_command_missing".into()],
        evidence_refs: Vec::new(),
        source_mutation_performed: false,
    }
}
