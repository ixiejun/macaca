//! Agent Execution boundary observer for Skill self-evolution.
//!
//! This module is a Web-shell Decorator around the host's concrete
//! `AgentExecutionBackend`. It does not own Skill evolution semantics. The
//! decorator observes the sanitized `AgentExecutionResult` that already crossed
//! `service.agent_execution`, records a small EventLog checkpoint for live
//! verification, and forwards the bounded candidate to the Skill service through
//! the SDK facade.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{AgentExecutionCommand, AgentExecutionResult, ServiceError, ServiceResult};
use macaca_runtime_host::AgentExecutionBackend;

use crate::runtime_event_bridge::emit_runtime_event;
use crate::skill_self_evolution_observer::observe_agent_execution_result_for_skill_self_evolution;
use crate::state::AppState;

/// Decorates Agent Execution with best-effort Skill self-evolution observation.
///
/// The wrapped backend remains the source of execution truth. This observer is
/// intentionally post-result and non-authoritative: it never changes success or
/// failure semantics, never parses prompts, never branches on applications, and
/// never mutates Skill package files. Proposal validation, storage, curation,
/// promotion, rejection, and rollback stay inside `service.skill`.
pub(crate) struct SkillSelfEvolutionObservedAgentExecutionBackend {
    inner: Arc<dyn AgentExecutionBackend>,
    state: Arc<AppState>,
}

impl SkillSelfEvolutionObservedAgentExecutionBackend {
    /// Create a Decorator around a concrete Agent Execution backend.
    pub(crate) fn new(inner: Arc<dyn AgentExecutionBackend>, state: Arc<AppState>) -> Self {
        Self { inner, state }
    }

    /// Record that the service produced a result and forward it to Skill service.
    ///
    /// EventLog writes happen before returning the result to the caller whenever
    /// possible. That ordering gives live verification a deterministic memento:
    /// if Agent Execution returned a terminal result, the same session should
    /// also contain a bounded `skill_self_evolution_observer` checkpoint.
    async fn observe_result(&self, result: &AgentExecutionResult) {
        tracing::info!(
            app_id = %result.application_id,
            session_id = %result.session_id,
            task_id = ?result.task_id,
            agent = %result.target_agent,
            status = %result.status.as_str(),
            "Skill self-evolution observer saw Agent Execution result"
        );

        emit_runtime_event(
            &self.state,
            &result.session_id,
            "skill_self_evolution_observer",
            "service.agent_execution",
            Some(result.target_agent.as_str()),
            serde_json::json!({
                "status": "agent_execution_completed_seen",
                "task_id": result.task_id.map(|task_id| task_id.to_string()),
                "proposal_id": null,
                "reason": "agent execution service result reached decorated completion boundary",
            }),
        )
        .await;

        if let Some(observation) =
            observe_agent_execution_result_for_skill_self_evolution(&self.state, result).await
        {
            tracing::info!(
                app_id = %result.application_id,
                session_id = %result.session_id,
                task_id = ?observation.task_id,
                proposal_id = ?observation.proposal_id,
                observer_status = observation.status,
                "Skill self-evolution observer recorded Agent Execution outcome"
            );
            emit_runtime_event(
                &self.state,
                &result.session_id,
                "skill_self_evolution_observer",
                "service.agent_execution",
                Some(result.target_agent.as_str()),
                serde_json::json!({
                    "status": observation.status,
                    "task_id": observation.task_id,
                    "proposal_id": observation.proposal_id,
                    "reason": observation.reason,
                }),
            )
            .await;
        }
    }

    /// Record service errors that occur before an `AgentExecutionResult` exists.
    ///
    /// The EventLog payload stores only a generic reason and an error kind. The
    /// concrete error message can contain adapter details, so it is kept out of
    /// the session-visible payload to preserve the same sanitization posture used
    /// by proposal records and reports.
    async fn observe_service_error(&self, command: &AgentExecutionCommand, error: &ServiceError) {
        let error_kind = service_error_kind(error);
        tracing::warn!(
            app_id = %command.application_id,
            session_id = %command.session_id,
            task_id = ?command.task_id,
            agent = %command.target_agent,
            error_kind,
            "Agent Execution returned service error before Skill self-evolution could inspect a result"
        );

        emit_runtime_event(
            &self.state,
            &command.session_id,
            "skill_self_evolution_observer",
            "service.agent_execution",
            Some(command.target_agent.as_str()),
            serde_json::json!({
                "status": "agent_execution_service_error",
                "task_id": command.task_id.map(|task_id| task_id.to_string()),
                "proposal_id": null,
                "reason": "agent execution returned a service error before producing a result",
                "error_kind": error_kind,
            }),
        )
        .await;
    }
}

#[async_trait]
impl AgentExecutionBackend for SkillSelfEvolutionObservedAgentExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        match self.inner.execute(command.clone()).await {
            Ok(result) => {
                self.observe_result(&result).await;
                Ok(result)
            }
            Err(error) => {
                self.observe_service_error(&command, &error).await;
                Err(error)
            }
        }
    }
}

/// Return a bounded, provider-neutral label for service errors.
fn service_error_kind(error: &ServiceError) -> &'static str {
    match error {
        ServiceError::MissingTraceContext => "missing_trace_context",
        ServiceError::InvalidLifecycleTransition { .. } => "invalid_lifecycle_transition",
        ServiceError::ServiceUnavailable(_) => "service_unavailable",
        ServiceError::DisabledByPolicy(_) => "disabled_by_policy",
        ServiceError::InvalidArgument(_) => "invalid_argument",
        ServiceError::UnsupportedCommand(_) => "unsupported_command",
        ServiceError::HealthCheckFailed(_) => "health_check_failed",
        ServiceError::AdapterFailure(_) => "adapter_failure",
        ServiceError::KernelPrimitive(_) => "kernel_primitive",
    }
}
