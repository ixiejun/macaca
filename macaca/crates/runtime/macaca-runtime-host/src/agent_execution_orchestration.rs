//! Provider-neutral agent execution orchestration for `service.agent_execution`.
//!
//! This module owns the **semantic** half of delegated agent runs: prompt
//! composition, heartbeat guards, execution-policy selection, execution-control
//! scope construction, and service-backed context retrieval.  It deliberately
//! does **not** own shell concerns such as SSE wiring, `ActiveSession` resume
//! channels, or kernel executor lifecycle broadcasts — those stay in the Web
//! adapter until P1 fully converges on a single service path.
//!
//! Design patterns:
//! - **Strategy**: completion-policy kind selects loop budget and tool choice.
//! - **Command**: context build flows through typed `AgentContextBuildCommand`.
//! - **Template Method**: prompt sections are assembled in a fixed order so
//!   audit replay can reason about envelope precedence without parsing logs.

use macaca_framework::model::ToolChoice;
use macaca_proto::{
    AgentContextBuildCommand, AgentContextSnapshot, AgentExecutionCommand, AgentExecutionEvent,
    AgentExecutionIntent, AgentExecutionResult, AgentExecutionStatus,
    AutonomousCompletionPolicyKind, ExecutionControlCheckpointMode, ExecutionControlPolicy,
    ExecutionControlPolicyOverride, ExecutionControlResolutionStatus,
    ExecutionControlResolvedPolicy, ExecutionControlResumeSource, ExecutionControlScope,
    ExecutionControlTrigger, KernelServiceId, ServiceBusSource, ServiceError, ServiceResult,
    TaskId, AGENT_CONTEXT_SERVICE_ID,
};
use macaca_tools::{ShellTool, ToolCommand, ToolCommandExecutor};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::execution_control::ExecutionControlPolicyResolver;
use crate::ServiceRuntime;

/// Default ReAct loop budget for generic delegated runtime work.
pub const DEFAULT_RUNTIME_AGENT_MAX_ITERS: usize = 25;

/// Short loop budget for artifact-backed autonomous completion.
///
/// Once the authorized tool mutates the expected artifact, additional ReAct
/// turns only add latency and timeout risk without changing the evidence gate.
pub const ARTIFACT_BACKED_RUNTIME_AGENT_MAX_ITERS: usize = 12;

/// Service-backed context snapshot retrieval used by every Agent Execution host.
///
/// The Runtime Host remains the owner of service routing; shells supply only
/// the `ServiceRuntime` handle and a provider-neutral bus source label for
/// audit attribution.
pub async fn build_agent_context_snapshot_via_service(
    service_runtime: &ServiceRuntime,
    bus_source: &str,
    command: &AgentExecutionCommand,
) -> ServiceResult<AgentContextSnapshot> {
    let context_command = AgentContextBuildCommand::from_execution(command);
    info!(
        trace_id = %command.trace.trace_id,
        service_id = AGENT_CONTEXT_SERVICE_ID,
        target_agent = %command.target_agent,
        bus_source = %bus_source,
        "agent execution requesting context snapshot through service runtime"
    );
    let reply = service_runtime
        .call(
            &KernelServiceId::new(AGENT_CONTEXT_SERVICE_ID),
            ServiceBusSource::new(bus_source),
            context_command
                .into_service_command()
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
        )
        .await
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
    let output = reply.output.ok_or_else(|| {
        ServiceError::AdapterFailure("agent context service returned no output".into())
    })?;
    serde_json::from_value(output).map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}

/// Compose the user-visible prompt from envelope, raw prompt, and evidence JSON.
#[must_use]
pub fn user_prompt_with_context(command: &AgentExecutionCommand) -> String {
    let context = structured_execution_context(command);
    let envelope = render_execution_envelope(command);
    let mut sections = Vec::new();
    if let Some(envelope) = envelope {
        sections.push(envelope);
    }
    sections.push(command.user_prompt.clone());
    if !context.is_null() && context != serde_json::json!({}) {
        let rendered_context =
            serde_json::to_string_pretty(&context).unwrap_or_else(|_| context.to_string());
        sections.push(format!(
            "Structured evidence context for this delegated task:\n```json\n{}\n```",
            rendered_context
        ));
    }
    sections.join("\n\n")
}

/// Render the OS-owned autonomous execution envelope before normal prompt context.
#[must_use]
pub fn render_execution_envelope(command: &AgentExecutionCommand) -> Option<String> {
    let envelope = command.execution_envelope.as_ref()?;
    let rendered = serde_json::to_string_pretty(envelope).unwrap_or_else(|_| "{}".to_string());
    Some(format!(
        "Highest-priority delegated execution contract. Follow this contract before persona, memory, profile, or other contextual material. If this contract conflicts with contextual guidance, the contract wins.\n```json\n{}\n```",
        rendered
    ))
}

/// Build provider-neutral JSON context safe to show the runtime agent.
#[must_use]
pub fn structured_execution_context(command: &AgentExecutionCommand) -> serde_json::Value {
    let mut context = serde_json::Map::new();
    if !command.delegated_context.is_null() && command.delegated_context != serde_json::json!({}) {
        context.insert(
            "delegated_context".into(),
            command.delegated_context.clone(),
        );
    }
    let evidence_requirements = command
        .metadata
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix("evidence.")
                .filter(|_| !value.trim().is_empty())
                .map(|name| (name.to_string(), serde_json::Value::String(value.clone())))
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();
    if !evidence_requirements.is_empty() {
        context.insert(
            "evidence_requirements".into(),
            serde_json::Value::Object(evidence_requirements),
        );
    }
    serde_json::Value::Object(context)
}

/// Whether coarse executor lifecycle events should be emitted for this run.
///
/// Migration surface: removed once all callers use a single event owner (P1.6).
#[must_use]
pub fn should_emit_executor_lifecycle(command: &AgentExecutionCommand) -> bool {
    command
        .metadata
        .get("suppress_executor_lifecycle")
        .map(|value| value != "true")
        .unwrap_or(true)
}

/// Verify trusted Agent Context supplied `HEARTBEAT.md` evidence.
#[must_use]
pub fn has_heartbeat_source_evidence(context_snapshot: &AgentContextSnapshot) -> bool {
    context_snapshot.sources.iter().any(|source| {
        source.kind == "profile_file"
            && (source.name == "HEARTBEAT.md"
                || source
                    .location
                    .as_deref()
                    .is_some_and(|location| location.ends_with("/HEARTBEAT.md")))
    })
}

/// Skip heartbeat runs when profile evidence is absent (auditable no-op).
#[must_use]
pub fn should_skip_heartbeat_without_source(
    command: &AgentExecutionCommand,
    context_snapshot: &AgentContextSnapshot,
) -> bool {
    matches!(command.execution_intent, AgentExecutionIntent::Heartbeat)
        && !has_heartbeat_source_evidence(context_snapshot)
}

/// Select the ReAct iteration budget from the compiled completion policy.
#[must_use]
pub fn runtime_agent_max_iters(command: &AgentExecutionCommand) -> usize {
    match command
        .execution_envelope
        .as_ref()
        .map(|envelope| envelope.completion_policy.kind)
    {
        Some(AutonomousCompletionPolicyKind::RequireArtifact) => {
            ARTIFACT_BACKED_RUNTIME_AGENT_MAX_ITERS
        }
        _ => DEFAULT_RUNTIME_AGENT_MAX_ITERS,
    }
}

/// Select provider-neutral model tool policy from the completion contract.
#[must_use]
pub fn runtime_agent_tool_choice(command: &AgentExecutionCommand) -> Option<ToolChoice> {
    match command
        .execution_envelope
        .as_ref()
        .map(|envelope| envelope.completion_policy.kind)
    {
        Some(AutonomousCompletionPolicyKind::RequireArtifact) => Some(ToolChoice::Required),
        _ => None,
    }
}

/// Extract a deterministic heartbeat shell contract from profile source evidence.
#[must_use]
pub fn heartbeat_exact_shell_contract(
    command: &AgentExecutionCommand,
    context_snapshot: &AgentContextSnapshot,
) -> Option<String> {
    if !matches!(command.execution_intent, AgentExecutionIntent::Heartbeat)
        || !matches!(
            command
                .execution_envelope
                .as_ref()
                .map(|envelope| envelope.completion_policy.kind),
            Some(AutonomousCompletionPolicyKind::RequireArtifact)
        )
    {
        return None;
    }
    let source = context_snapshot.sources.iter().find(|source| {
        source.kind == "profile_file"
            && (source.name == "HEARTBEAT.md"
                || source
                    .location
                    .as_deref()
                    .is_some_and(|location| location.ends_with("/HEARTBEAT.md")))
    })?;
    let path = source.location.as_ref()?;
    let body = std::fs::read_to_string(path).ok()?;
    extract_single_shell_fence(&body)
}

/// Parse exactly one fenced shell block from a profile body.
#[must_use]
pub fn extract_single_shell_fence(body: &str) -> Option<String> {
    let mut captured = Vec::new();
    let mut active = false;
    let mut matches = 0usize;
    let mut current = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        if !active && trimmed.starts_with("```") {
            let lang = trimmed.trim_start_matches("```").trim();
            if matches!(lang, "sh" | "shell" | "bash") {
                active = true;
                current.clear();
            }
            continue;
        }
        if active && trimmed == "```" {
            active = false;
            matches += 1;
            captured = current.clone();
            continue;
        }
        if active {
            current.push(line.to_string());
        }
    }

    if matches == 1 {
        let command = captured.join("\n").trim().to_string();
        (!command.is_empty()).then_some(command)
    } else {
        None
    }
}

/// Deprecated compatibility policy for legacy YAML chat main-thread sessions.
///
/// Migration surface: replaced by manifest projection (P1.6).
#[must_use]
pub fn legacy_chat_main_thread_execution_control_policy(
    command: &AgentExecutionCommand,
) -> Option<ExecutionControlPolicy> {
    if command.execution_control_override.is_some()
        || !matches!(
            command.execution_intent,
            AgentExecutionIntent::ChatMainThread
        )
    {
        return None;
    }

    Some(
        ExecutionControlPolicy::enabled(
            vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::ReferenceOnly,
        )
        .allow_command_overrides(false),
    )
}

/// Resolve execution-control policy using the same Strategy as the service layer.
///
/// Production hosts should prefer the service-backed resolver; this helper
/// exists for unit tests and offline policy admission checks.
#[must_use]
pub fn resolve_execution_control_policy_local(
    command: &AgentExecutionCommand,
) -> ExecutionControlResolvedPolicy {
    let compat_policy = legacy_chat_main_thread_execution_control_policy(command);
    let command_declared_policy =
        command
            .execution_control_override
            .as_ref()
            .map(|override_policy| {
                ExecutionControlPolicy::enabled(
                    override_policy.triggers.clone(),
                    override_policy.resume_sources.clone(),
                    override_policy.checkpoint_mode.clone(),
                )
                .allow_command_overrides(true)
            });
    let app_policy = compat_policy.as_ref().or(command_declared_policy.as_ref());
    let mut resolution = ExecutionControlPolicyResolver::resolve(
        app_policy,
        command.execution_control_override.as_ref(),
    );
    if compat_policy.is_some() && resolution.status == ExecutionControlResolutionStatus::Enabled {
        resolution.metadata.insert(
            "compatibility".into(),
            "legacy_chat_main_thread_goal_pause".into(),
        );
    }
    resolution
}

/// Stable provider-neutral execution id for one service run.
#[must_use]
pub fn execution_control_execution_id(command: &AgentExecutionCommand) -> String {
    match command.task_id {
        Some(task_id) => format!("{}:{}", command.session_id, task_id),
        None => format!("{}:{}", command.session_id, command.target_agent),
    }
}

/// Build the provider-neutral execution-control command scope.
///
/// `source` is the audit attribution label for the calling host adapter (for
/// example `macaca.web.agent_execution` or `macaca.cli.agent_execution`).
pub fn execution_control_scope(
    command: &AgentExecutionCommand,
    execution_id: String,
    source: &str,
) -> ServiceResult<ExecutionControlScope> {
    let mut scope = ExecutionControlScope::new(
        command.application_id,
        command.session_id.clone(),
        execution_id,
        command.trace.clone(),
        source,
    )
    .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
    scope.task_id = command.task_id;
    scope
        .metadata
        .insert("target_agent".into(), command.target_agent.clone());
    scope.metadata.insert(
        "execution_intent".into(),
        format!("{:?}", command.execution_intent),
    );
    Ok(scope)
}

/// Execute one exact shell command from a heartbeat profile contract.
///
/// This Command path bypasses the ReAct loop when the completion policy and
/// trusted context evidence prove that exactly one shell action is authorized.
pub async fn execute_exact_heartbeat_shell(
    command_text: &str,
    agent_event_tx: &mpsc::Sender<AgentExecutionEvent>,
) -> Result<serde_json::Value, String> {
    info!(
        contract = "heartbeat_profile_exact_shell",
        "executing exact heartbeat shell contract"
    );
    let tool_input = serde_json::json!({
        "command": command_text,
        "timeout_secs": 10,
    });
    let _ = agent_event_tx
        .send(AgentExecutionEvent::tool_call(
            "shell".into(),
            serde_json::json!({
                "contract": "heartbeat_profile_exact_shell",
                "timeout_secs": 10,
            }),
        ))
        .await;
    let result = ShellTool::default()
        .execute_command(ToolCommand::new(tool_input))
        .await
        .map_err(|error| error.to_string())?;
    let is_error = result
        .get("exit_code")
        .and_then(|value| value.as_i64())
        .map(|code| code != 0)
        .unwrap_or(false);
    let _ = agent_event_tx
        .send(AgentExecutionEvent::tool_result_with_error(
            "shell".into(),
            serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()),
            is_error,
        ))
        .await;
    if is_error {
        return Err(format!("heartbeat exact shell exited with {result}"));
    }
    Ok(result)
}

/// Build a structured failed execution result with optional context snapshot.
#[must_use]
pub fn failed_execution_result(
    command: &AgentExecutionCommand,
    task_id: TaskId,
    error: impl Into<String>,
    context_snapshot: Option<AgentContextSnapshot>,
) -> AgentExecutionResult {
    let error = error.into();
    warn!(
        trace_id = %command.trace.trace_id,
        target_agent = %command.target_agent,
        task_id = %task_id.0,
        error = %error,
        "agent execution run failed"
    );
    AgentExecutionResult {
        application_id: command.application_id,
        session_id: command.session_id.clone(),
        task_id: Some(task_id),
        target_agent: command.target_agent.clone(),
        status: AgentExecutionStatus::Failed,
        output: serde_json::json!({ "error": error }),
        context_snapshot,
        trace: command.trace.clone(),
        metadata: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{
        AgentExecutionIntent, ExecutionControlCheckpointMode, ExecutionControlPolicyOverride,
        ExecutionControlResolutionStatus, ExecutionControlResumeSource, ExecutionControlTrigger,
        TraceContext,
    };

    #[test]
    fn chat_main_thread_compat_policy_is_enabled() {
        let command = AgentExecutionCommand::new(
            macaca_proto::ApplicationId::from_name("demo"),
            "session-a",
            "entry-agent",
            AgentExecutionIntent::ChatMainThread,
            "create a project goal",
            TraceContext::new("trace-chat-main-thread"),
        )
        .unwrap();

        let resolution = resolve_execution_control_policy_local(&command);

        assert_eq!(resolution.status, ExecutionControlResolutionStatus::Enabled);
        assert_eq!(
            resolution.metadata.get("compatibility"),
            Some(&"legacy_chat_main_thread_goal_pause".to_string())
        );
    }

    #[test]
    fn artifact_policy_selects_short_budget_and_required_tools() {
        let mut command = AgentExecutionCommand::new(
            macaca_proto::ApplicationId::from_name("demo"),
            "session-a",
            "worker",
            AgentExecutionIntent::Heartbeat,
            "run artifact work",
            TraceContext::new("trace-artifact-policy"),
        )
        .unwrap();
        command.execution_envelope = Some(
            macaca_proto::AutonomousExecutionEnvelope::compile(
                macaca_proto::AutonomousExecutionSourceKind::HeartbeatProfile,
                "run artifact work",
                &std::collections::BTreeMap::from([(
                    "evidence.expected_artifact_path".into(),
                    "/workspace/sentinel.md".into(),
                )]),
            )
            .unwrap(),
        );

        assert_eq!(
            runtime_agent_max_iters(&command),
            ARTIFACT_BACKED_RUNTIME_AGENT_MAX_ITERS
        );
        assert_eq!(
            runtime_agent_tool_choice(&command),
            Some(ToolChoice::Required)
        );
    }

    #[test]
    fn extract_single_shell_fence_requires_exactly_one_block() {
        let body = r#"
# Heartbeat
```sh
echo ok
```
"#;
        assert_eq!(
            extract_single_shell_fence(body),
            Some("echo ok".to_string())
        );
        assert!(extract_single_shell_fence("```sh\necho ok").is_none());
    }
}
