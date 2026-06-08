//! Heartbeat agent dispatch strategy — orchestrates wake → agent execution.
//!
//! Implements the **Strategy** pattern: injectable timeout and [`ServiceRuntime`],
//! with each audit node logged via `tracing` for traceability.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, AgentExecutionStatus,
    ApplicationHeartbeatAgentView, ApplicationHeartbeatAgentsResult, ApplicationId,
    AutonomousExecutionEnvelope, AutonomousExecutionSourceKind, HeartbeatCommandResult,
    HeartbeatCompleteRunCommand, HeartbeatRunState, KernelServiceId, MacacaError, MacacaResult,
    ServiceBusSource, TraceContext, AGENT_EXECUTE_COMMAND, AGENT_EXECUTION_SERVICE_ID,
    APPLICATION_SERVICE_ID, HEARTBEAT_SERVICE_ID,
};
use tokio::time::timeout;
use tracing::{info, warn};

use crate::autonomy_result_evidence::{AgentExecutionEvidenceDecision, AgentExecutionEvidenceGate};
use crate::skill_alias_resolution::resolve_skill_alias_metadata;
use crate::ServiceRuntime;

use super::summary::HeartbeatAgentDispatchSummary;
use super::support::{
    application_id_from_wake, declaration_matches_wake, dispatch_error_reason, dispatch_metadata,
    session_id_from_wake,
};

/// Replaceable Strategy for dispatching accepted heartbeat wakes to agents.
pub(crate) struct HeartbeatAgentDispatchStrategy {
    pub(crate) runtime: Arc<ServiceRuntime>,
    pub(crate) timeout_ms: u64,
}

impl HeartbeatAgentDispatchStrategy {
    /// Create a strategy with an explicit dispatch timeout.
    ///
    /// Heartbeat agent execution is intentionally bounded at the service-call
    /// handoff. A timed-out dispatch is logged and counted as failure evidence
    /// instead of keeping heartbeat or scheduler coordination stuck forever.
    pub(crate) fn with_timeout(runtime: Arc<ServiceRuntime>, timeout_ms: u64) -> Self {
        Self {
            runtime,
            timeout_ms: timeout_ms.max(1),
        }
    }

    /// Dispatch enabled manifest declarations for one accepted Heartbeat wake.
    ///
    /// The wake result contains only sanitized metadata emitted by Heartbeat.
    /// If the wake is not app-scoped, or services are unavailable, the strategy
    /// returns structured skip/failure counts and leaves the Heartbeat lane
    /// alive. No branch in this method depends on app names, agent roles,
    /// providers, models, workflows, or business domains.
    pub(crate) async fn dispatch_after_accepted_wake(
        &self,
        wake: &HeartbeatCommandResult,
    ) -> MacacaResult<HeartbeatAgentDispatchSummary> {
        if !wake.accepted {
            return Ok(HeartbeatAgentDispatchSummary::default());
        }
        let Some(application_id) = application_id_from_wake(wake) else {
            info!(
                trace_id = wake.trace.trace_id.as_str(),
                "heartbeat agent dispatch skipped because accepted wake is not application-scoped"
            );
            return Ok(HeartbeatAgentDispatchSummary {
                skipped: 1,
                completion_state: Some(HeartbeatRunState::Skipped),
                reason_code: Some("non_application_wake".into()),
                ..HeartbeatAgentDispatchSummary::default()
            });
        };

        let declarations = match self
            .query_declarations(wake.trace.clone(), application_id)
            .await
        {
            Ok(declarations) => declarations,
            Err(error) => {
                warn!(
                    trace_id = wake.trace.trace_id.as_str(),
                    app_id = %application_id,
                    error = %error,
                    "heartbeat agent dispatch declaration query failed"
                );
                return Ok(HeartbeatAgentDispatchSummary {
                    failed: 1,
                    completion_state: Some(HeartbeatRunState::Failed),
                    reason_code: Some("declaration_query_failed".into()),
                    ..HeartbeatAgentDispatchSummary::default()
                });
            }
        };
        let mut summary = HeartbeatAgentDispatchSummary {
            queried: declarations.len(),
            ..HeartbeatAgentDispatchSummary::default()
        };
        let scoped_declarations = declarations
            .into_iter()
            .filter(|declaration| declaration_matches_wake(wake, declaration))
            .collect::<Vec<_>>();
        for declaration in scoped_declarations {
            if !declaration.enabled || !declaration.diagnostics.is_empty() {
                summary.skipped += 1;
                continue;
            }
            summary.enabled += 1;
            match self.dispatch_agent(wake, &declaration).await {
                Ok(metadata) => {
                    summary.dispatched += 1;
                    summary.metadata.extend(metadata);
                }
                Err(error) => {
                    summary.failed += 1;
                    summary.reason_code = Some(dispatch_error_reason(&error).into());
                    warn!(
                        trace_id = wake.trace.trace_id.as_str(),
                        app_id = %application_id,
                        agent_name = %declaration.agent_name,
                        error = %error,
                        "heartbeat agent dispatch request failed"
                    );
                }
            }
        }
        if summary.failed > 0 {
            summary.completion_state = Some(HeartbeatRunState::Failed);
            summary
                .reason_code
                .get_or_insert_with(|| "agent_execution_failed".into());
        } else if summary.dispatched > 0 {
            summary.completion_state = Some(HeartbeatRunState::Succeeded);
            summary.reason_code = Some("agent_execution_completed".into());
        } else {
            summary.completion_state = Some(HeartbeatRunState::Skipped);
            summary.reason_code = Some("no_eligible_heartbeat_declaration".into());
        }
        summary
            .metadata
            .insert("dispatch.queried".into(), summary.queried.to_string());
        summary
            .metadata
            .insert("dispatch.enabled".into(), summary.enabled.to_string());
        summary
            .metadata
            .insert("dispatch.dispatched".into(), summary.dispatched.to_string());
        summary
            .metadata
            .insert("dispatch.skipped".into(), summary.skipped.to_string());
        summary
            .metadata
            .insert("dispatch.failed".into(), summary.failed.to_string());
        info!(
            trace_id = wake.trace.trace_id.as_str(),
            app_id = %application_id,
            queried = summary.queried,
            enabled = summary.enabled,
            dispatched = summary.dispatched,
            skipped = summary.skipped,
            failed = summary.failed,
            "heartbeat agent dispatch completed"
        );
        Ok(summary)
    }

    /// Record the terminal dispatch observation through the Heartbeat service.
    ///
    /// Runtime Host owns the observer role for Agent Execution results, but the
    /// Heartbeat service remains the memento owner. This method therefore goes
    /// back through `ServiceRuntime` instead of mutating provider state through
    /// an application-specific side channel.
    pub(crate) async fn record_completion(
        &self,
        command: HeartbeatCompleteRunCommand,
    ) -> MacacaResult<()> {
        let service_command = command.into_service_command()?;
        let trace_id = service_command
            .trace
            .as_ref()
            .map(|trace| trace.trace_id.clone())
            .unwrap_or_else(|| "missing-trace".into());
        let reply = self
            .runtime
            .call(
                &KernelServiceId::new(HEARTBEAT_SERVICE_ID),
                ServiceBusSource::new("runtime.heartbeat_agent_dispatch"),
                service_command,
            )
            .await
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        if !reply.success {
            return Err(MacacaError::Config(format!(
                "heartbeat completion record returned {}",
                reply.status
            )));
        }
        info!(
            trace_id = trace_id.as_str(),
            "heartbeat dispatch completion recorded through service.heartbeat"
        );
        Ok(())
    }

    /// Query manifest heartbeat agent declarations via Application Service bus.
    async fn query_declarations(
        &self,
        trace: TraceContext,
        application_id: ApplicationId,
    ) -> MacacaResult<ApplicationHeartbeatAgentsResult> {
        use macaca_proto::ApplicationHeartbeatAgentsQueryCommand;

        let command =
            ApplicationHeartbeatAgentsQueryCommand::application(trace.clone(), application_id)?
                .into_service_command()?;
        let reply = self
            .runtime
            .call(
                &KernelServiceId::new(APPLICATION_SERVICE_ID),
                ServiceBusSource::new("runtime.heartbeat_agent_dispatch"),
                command,
            )
            .await
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        let output = reply.output.ok_or_else(|| {
            MacacaError::Config("application heartbeat declaration query returned no output".into())
        })?;
        serde_json::from_value(output).map_err(|error| MacacaError::Config(error.to_string()))
    }

    /// Dispatch one enabled declaration through Agent Execution with evidence gate.
    async fn dispatch_agent(
        &self,
        wake: &HeartbeatCommandResult,
        declaration: &ApplicationHeartbeatAgentView,
    ) -> MacacaResult<BTreeMap<String, String>> {
        let mut command = AgentExecutionCommand::new(
            declaration.application_id,
            session_id_from_wake(wake, declaration),
            declaration.agent_name.clone(),
            AgentExecutionIntent::Heartbeat,
            "Execute the trusted HEARTBEAT.md task exactly. If the task specifies an exact artifact path, write that exact artifact and do not create an alternate file.",
            wake.trace.clone(),
        )?;
        command.metadata = dispatch_metadata(wake, declaration);
        resolve_skill_alias_metadata(
            self.runtime.as_ref(),
            wake.trace.clone(),
            macaca_skill::SkillServiceScope::agent(
                declaration.application_id,
                command.session_id.clone(),
                declaration.agent_name.clone(),
            )?,
            &mut command.metadata,
            "runtime.heartbeat_agent_dispatch",
            self.timeout_ms,
        )
        .await?;
        let envelope = AutonomousExecutionEnvelope::compile(
            AutonomousExecutionSourceKind::HeartbeatProfile,
            command.user_prompt.clone(),
            &command.metadata,
        )?;
        command.metadata.insert(
            "execution_envelope.source_kind".into(),
            envelope.source_kind.as_str().into(),
        );
        command.metadata.insert(
            "execution_envelope.completion_policy".into(),
            envelope.completion_policy.kind.as_str().into(),
        );
        info!(
            trace_id = wake.trace.trace_id.as_str(),
            app_id = %declaration.application_id,
            agent_name = %declaration.agent_name,
            source_kind = envelope.source_kind.as_str(),
            completion_policy = envelope.completion_policy.kind.as_str(),
            "heartbeat agent dispatch compiled autonomous execution envelope"
        );
        command.execution_envelope = Some(envelope.clone());
        command.delegated_context = serde_json::json!({
            "heartbeat": {
                "run_id": wake.run_id.as_ref().map(|run_id| run_id.as_str()),
                "audit_id": wake.audit_id,
                "profile_id": declaration.profile_id,
            }
        });
        let service_command = command.into_service_command()?;
        let reply = timeout(
            Duration::from_millis(self.timeout_ms),
            self.runtime.call(
                &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                ServiceBusSource::new("runtime.heartbeat_agent_dispatch"),
                service_command,
            ),
        )
        .await
        .map_err(|_| MacacaError::Config("agent execution timed out".into()))?
        .map_err(|error| MacacaError::Config(error.to_string()))?;
        if !reply.success {
            return Err(MacacaError::Config(format!(
                "agent execution returned {}",
                reply.status
            )));
        }
        let output = reply.output.ok_or_else(|| {
            MacacaError::Config("agent execution returned no result output".into())
        })?;
        let result: AgentExecutionResult = serde_json::from_value(output)
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        if result.status != AgentExecutionStatus::Completed {
            return Err(MacacaError::Config(format!(
                "agent execution status {}",
                result.status.as_str()
            )));
        }
        match AgentExecutionEvidenceGate::evaluate_with_policy(&result, &envelope.completion_policy)
        {
            AgentExecutionEvidenceDecision::Verified { evidence_key } => {
                info!(
                    trace_id = wake.trace.trace_id.as_str(),
                    command = AGENT_EXECUTE_COMMAND,
                    status = %reply.status,
                    evidence_key,
                    "heartbeat agent execution result evidence verified"
                );
                Ok(BTreeMap::from([
                    ("agent_execution.status".into(), "completed".into()),
                    (
                        "agent_execution.evidence_key".into(),
                        evidence_key.to_string(),
                    ),
                    (
                        "agent_execution.completion_policy".into(),
                        envelope.completion_policy.kind.as_str().into(),
                    ),
                    (
                        "execution_envelope.source_kind".into(),
                        envelope.source_kind.as_str().into(),
                    ),
                    (
                        "execution_envelope.completion_policy".into(),
                        envelope.completion_policy.kind.as_str().into(),
                    ),
                ]))
            }
            AgentExecutionEvidenceDecision::MissingEvidence => Err(MacacaError::Config(
                "agent execution completed without result evidence".into(),
            )),
            AgentExecutionEvidenceDecision::NotCompleted => Err(MacacaError::Config(
                "agent execution did not complete".into(),
            )),
        }
    }
}
