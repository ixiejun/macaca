//! Scheduled agent execution dispatch handler.
//!
//! **Pattern:** Strategy + Memento — Scheduler stores only `AutonomyPayloadRef`;
//! this handler resolves the prompt through `service.scheduled_agent_task` at
//! dispatch time, then forwards an audited `AgentExecutionCommand` to
//! `service.agent_execution` without interpreting application business semantics.

use std::time::Duration;

use macaca_proto::{
    AgentExecutionCommand, AgentExecutionResult, AgentExecutionStatus, AgentExecutionTargetCommand,
    AutonomousExecutionEnvelope, AutonomousExecutionSourceKind, KernelServiceId, MacacaResult,
    ResolveScheduledAgentTaskPayloadCommand, ScheduledAgentTaskResolvedPayload, ServiceBusSource,
    ServiceCommand, ServiceCommandName, TraceContext, AGENT_EXECUTION_SERVICE_ID,
    SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND, SCHEDULED_AGENT_TASK_SERVICE_ID,
    SCHEDULER_SERVICE_ID,
};
use tokio::time::timeout;
use tracing::{info, warn};

use crate::autonomy_result_evidence::{AgentExecutionEvidenceDecision, AgentExecutionEvidenceGate};
use crate::skill_alias_resolution::resolve_skill_alias_metadata;

use super::outcome::AutonomyDispatchOutcome;
use super::strategies::AutonomyDispatchStrategies;

/// Dispatch a scheduled `AgentExecution` target through service boundaries.
///
/// Scheduler stores only `AutonomyPayloadRef` and target metadata.  Runtime
/// Host resolves the prompt through `service.scheduled_agent_task` at the
/// dispatch boundary, then creates an `AgentExecutionCommand` for
/// `service.agent_execution`.  This Strategy keeps Scheduler free of prompt
/// interpretation and keeps LLM/tool execution owned by Agent Execution.
pub(crate) async fn dispatch_agent_execution(
    strategies: &AutonomyDispatchStrategies<'_>,
    trace: TraceContext,
    target: AgentExecutionTargetCommand,
) -> MacacaResult<AutonomyDispatchOutcome> {
    let payload_digest = target.payload_ref.content_digest.clone();
    let resolved = resolve_scheduled_agent_payload(strategies, trace.clone(), &target).await?;
    let Some(resolved) = resolved else {
        warn!(
            trace_id = trace.trace_id.as_str(),
            payload_digest = payload_digest.as_deref().unwrap_or("none"),
            "scheduled agent dispatch skipped because payload was unavailable"
        );
        return Ok(AutonomyDispatchOutcome::skipped(
            "scheduled_agent_payload_unavailable",
        ));
    };
    let mut command = AgentExecutionCommand::new(
        resolved.application_id,
        resolved.session_id.clone(),
        target
            .target_agent
            .clone()
            .unwrap_or_else(|| resolved.target_agent.clone()),
        resolved.execution_intent.clone(),
        resolved.user_prompt.clone(),
        trace.clone(),
    )?
    .with_delegated_context(resolved.delegated_context.clone());
    command.task_id = resolved.task_ref;
    command.policy = resolved.policy.clone();
    command.metadata = target.metadata.clone();
    // Copy only evidence and skill-alias metadata keys from the resolved Memento.
    for (key, value) in &resolved.metadata {
        if (key.starts_with("evidence.") || key.starts_with("skill.alias."))
            && !value.trim().is_empty()
        {
            command.metadata.insert(key.clone(), value.clone());
        }
    }
    // Route provenance through proto constants so service ids stay canonical and auditable.
    command
        .metadata
        .insert("source".into(), SCHEDULER_SERVICE_ID.into());
    command
        .metadata
        .insert("scheduler_run_source".into(), SCHEDULER_SERVICE_ID.into());
    command.metadata.insert(
        "scheduled_agent_task_id".into(),
        resolved.task_id.as_str().into(),
    );
    if let Some(digest) = resolved.payload_digest.clone() {
        command.metadata.insert("payload_digest".into(), digest);
    }
    if let Some(audit_id) = resolved.audit_id.clone() {
        command
            .metadata
            .insert("scheduled_agent_task_audit_id".into(), audit_id);
    }
    resolve_skill_alias_metadata(
        strategies.runtime,
        trace.clone(),
        macaca_skill::SkillServiceScope::agent(
            resolved.application_id,
            resolved.session_id.clone(),
            command.target_agent.clone(),
        )?,
        &mut command.metadata,
        "runtime.autonomy_supervisor",
        strategies.timeout_ms,
    )
    .await?;
    let envelope = AutonomousExecutionEnvelope::compile(
        AutonomousExecutionSourceKind::ScheduledAgentTask,
        resolved.user_prompt.clone(),
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
        service_id = AGENT_EXECUTION_SERVICE_ID,
        target_agent = command.target_agent.as_str(),
        task_id = resolved.task_id.as_str(),
        source_kind = envelope.source_kind.as_str(),
        completion_policy = envelope.completion_policy.kind.as_str(),
        payload_digest = command
            .metadata
            .get("payload_digest")
            .map(String::as_str)
            .unwrap_or("none"),
        trace_id = trace.trace_id.as_str(),
        "scheduled agent dispatch invoking agent execution service"
    );
    command.execution_envelope = Some(envelope.clone());
    let service_command = command.into_service_command()?;
    match timeout(
        Duration::from_millis(strategies.timeout_ms),
        strategies.runtime.call(
            &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
            ServiceBusSource::new("runtime.autonomy_supervisor"),
            service_command,
        ),
    )
    .await
    {
        Ok(Ok(reply)) if reply.success => {
            let Some(output) = reply.output else {
                return Ok(AutonomyDispatchOutcome::retryable(
                    "agent_execution_empty_reply",
                ));
            };
            let result: AgentExecutionResult = match serde_json::from_value(output) {
                Ok(result) => result,
                Err(error) => {
                    warn!(
                        error = %error,
                        trace_id = trace.trace_id.as_str(),
                        "scheduled agent dispatch could not decode agent execution result"
                    );
                    return Ok(AutonomyDispatchOutcome::retryable(
                        "agent_execution_decode_failed",
                    ));
                }
            };
            info!(
                service_id = AGENT_EXECUTION_SERVICE_ID,
                target_agent = result.target_agent.as_str(),
                status = result.status.as_str(),
                trace_id = trace.trace_id.as_str(),
                "scheduled agent dispatch completed"
            );
            match result.status {
                AgentExecutionStatus::Completed => {
                    match AgentExecutionEvidenceGate::evaluate_with_policy(
                        &result,
                        &envelope.completion_policy,
                    ) {
                        AgentExecutionEvidenceDecision::Verified { evidence_key } => {
                            info!(
                                service_id = AGENT_EXECUTION_SERVICE_ID,
                                target_agent = result.target_agent.as_str(),
                                evidence_key,
                                trace_id = trace.trace_id.as_str(),
                                "scheduled agent dispatch result evidence verified"
                            );
                            Ok(AutonomyDispatchOutcome::succeeded())
                        }
                        AgentExecutionEvidenceDecision::MissingEvidence => {
                            warn!(
                                service_id = AGENT_EXECUTION_SERVICE_ID,
                                target_agent = result.target_agent.as_str(),
                                trace_id = trace.trace_id.as_str(),
                                "scheduled agent dispatch completed without result evidence"
                            );
                            Ok(AutonomyDispatchOutcome::retryable(
                                "agent_execution_result_evidence_missing",
                            ))
                        }
                        AgentExecutionEvidenceDecision::NotCompleted => Ok(
                            AutonomyDispatchOutcome::retryable("agent_execution_result_not_completed"),
                        ),
                    }
                }
                AgentExecutionStatus::Skipped | AgentExecutionStatus::Denied => {
                    Ok(AutonomyDispatchOutcome::skipped("agent_execution_skipped"))
                }
                AgentExecutionStatus::Unavailable
                | AgentExecutionStatus::Unsupported
                | AgentExecutionStatus::Failed => {
                    Ok(AutonomyDispatchOutcome::retryable("agent_execution_failed"))
                }
            }
        }
        Ok(Ok(_)) => Ok(AutonomyDispatchOutcome::retryable(
            "agent_execution_reply_failed",
        )),
        Ok(Err(error)) => {
            warn!(
                service_id = AGENT_EXECUTION_SERVICE_ID,
                error = %error,
                trace_id = trace.trace_id.as_str(),
                "scheduled agent dispatch failed when calling agent execution service"
            );
            Ok(AutonomyDispatchOutcome::retryable(
                "agent_execution_dispatch_failed",
            ))
        }
        Err(_) => Ok(AutonomyDispatchOutcome::retryable(
            "agent_execution_dispatch_timeout",
        )),
    }
}

/// Resolve scheduled agent task payload through `service.scheduled_agent_task`.
///
/// **Pattern:** Memento — Scheduler retains only `AutonomyPayloadRef`; the
/// Scheduled Agent Task service owns prompt materialization.  This resolver
/// never reads raw prompts from Scheduler target metadata directly.
async fn resolve_scheduled_agent_payload(
    strategies: &AutonomyDispatchStrategies<'_>,
    trace: TraceContext,
    target: &AgentExecutionTargetCommand,
) -> MacacaResult<Option<ScheduledAgentTaskResolvedPayload>> {
    let mut resolve = ResolveScheduledAgentTaskPayloadCommand::new(
        trace.clone(),
        target.payload_ref.clone(),
    )?;
    resolve.metadata = target.metadata.clone();
    let command = ServiceCommand::with_trace(
        ServiceCommandName::new(SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND),
        serde_json::to_value(resolve)?,
        trace.clone(),
    );
    info!(
        service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
        payload_digest = target
            .payload_ref
            .content_digest
            .as_deref()
            .unwrap_or("none"),
        trace_id = trace.trace_id.as_str(),
        "scheduled agent dispatch resolving payload reference"
    );
    match timeout(
        Duration::from_millis(strategies.timeout_ms),
        strategies.runtime.call(
            &KernelServiceId::new(SCHEDULED_AGENT_TASK_SERVICE_ID),
            ServiceBusSource::new("runtime.autonomy_supervisor"),
            command,
        ),
    )
    .await
    {
        Ok(Ok(reply)) if reply.success => {
            let Some(output) = reply.output else {
                return Ok(None);
            };
            let resolved: Option<ScheduledAgentTaskResolvedPayload> =
                serde_json::from_value(output)?;
            if let Some(resolved) = &resolved {
                info!(
                    service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
                    task_id = resolved.task_id.as_str(),
                    payload_digest = resolved.payload_digest.as_deref().unwrap_or("none"),
                    trace_id = trace.trace_id.as_str(),
                    "scheduled agent dispatch payload resolved"
                );
            }
            Ok(resolved)
        }
        Ok(Ok(_)) => Ok(None),
        Ok(Err(error)) => {
            warn!(
                service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
                error = %error,
                trace_id = trace.trace_id.as_str(),
                "scheduled agent dispatch payload resolution failed"
            );
            Ok(None)
        }
        Err(_) => Ok(None),
    }
}
