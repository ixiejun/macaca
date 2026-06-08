//! Provider-neutral Agent Execution and Agent Context service contracts.
//!
//! These DTOs define the single serviceized path for starting agent work.  They
//! intentionally separate trusted system context from application/user prompts:
//! adapters may supply `user_prompt` and bounded `delegated_context`, while the
//! host-owned Agent Context service builds persona, skill, tool, memory, and
//! workspace context.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ApplicationId, ExecutionControlPolicyOverride, MacacaError, MacacaResult, ServiceCommand,
    ServiceCommandName, TaskId, TraceContext,
};

/// Stable service id for the unified Agent Execution service.
pub const AGENT_EXECUTION_SERVICE_ID: &str = "service.agent_execution";

/// Stable service id for trusted Agent Context construction.
pub const AGENT_CONTEXT_SERVICE_ID: &str = "service.agent_context";

/// Command that starts one context-aware agent execution.
pub const AGENT_EXECUTE_COMMAND: &str = "agent.execute";

/// Command that builds trusted system context for one agent execution.
pub const AGENT_CONTEXT_BUILD_COMMAND: &str = "agent.context.build";

/// Semantic intent behind an agent execution request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentExecutionIntent {
    ChatMainThread,
    WasmDelegate,
    YamlWorkflowStep,
    TaskWorker,
    GoalWorker,
    Planner,
    Reviewer,
    Heartbeat,
    SdkInvocation,
    GatewayInvocation,
    Custom(String),
}

/// Metadata key used by `application.agent.delegate` to select execution intent.
///
/// The Application Service adapter reads this provider-neutral label and maps it
/// to `AgentExecutionIntent` before forwarding work to `service.agent_execution`.
pub const AGENT_EXECUTION_INTENT_METADATA_KEY: &str = "execution_intent";

/// Stable wire label for goal-decomposition planner executions.
///
/// This is an OS execution-path vocabulary token (like `task_worker` / `goal_worker`),
/// not an application agent role name. Keeping the label distinct from legacy role
/// literals prevents audit scanners from conflating intent metadata with persona ids.
pub const GOAL_PLANNER_EXECUTION_INTENT_LABEL: &str = "goal_planner";

impl AgentExecutionIntent {
    /// Serialize one intent into the stable metadata wire label.
    ///
    /// Labels are snake_case and intentionally avoid application-specific names so
    /// YAML, WASM, SDK, and future remote hosts can share the same vocabulary.
    pub fn metadata_value(&self) -> &'static str {
        match self {
            Self::ChatMainThread => "chat_main_thread",
            Self::WasmDelegate => "wasm_delegate",
            Self::YamlWorkflowStep => "yaml_workflow_step",
            Self::TaskWorker => "task_worker",
            Self::GoalWorker => "goal_worker",
            Self::Planner => GOAL_PLANNER_EXECUTION_INTENT_LABEL,
            Self::Reviewer => "reviewer",
            Self::Heartbeat => "heartbeat",
            Self::SdkInvocation => "sdk_invocation",
            Self::GatewayInvocation => "gateway_invocation",
            Self::Custom(_) => "custom",
        }
    }

    /// Resolve one intent from delegate metadata, defaulting to WASM delegation.
    ///
    /// Unknown custom labels are preserved as `Custom` so hosts can experiment
    /// without forking the Application Service contract.
    pub fn from_delegate_metadata(
        metadata: &BTreeMap<String, String>,
    ) -> Self {
        metadata
            .get(AGENT_EXECUTION_INTENT_METADATA_KEY)
            .map(|value| Self::from_metadata_value(value))
            .unwrap_or(Self::WasmDelegate)
    }

    /// Parse one metadata wire label into an execution intent.
    pub fn from_metadata_value(value: &str) -> Self {
        match value.trim() {
            "chat_main_thread" => Self::ChatMainThread,
            "wasm_delegate" => Self::WasmDelegate,
            "yaml_workflow_step" => Self::YamlWorkflowStep,
            "task_worker" => Self::TaskWorker,
            "goal_worker" => Self::GoalWorker,
            GOAL_PLANNER_EXECUTION_INTENT_LABEL => Self::Planner,
            "reviewer" => Self::Reviewer,
            "heartbeat" => Self::Heartbeat,
            "sdk_invocation" => Self::SdkInvocation,
            "gateway_invocation" => Self::GatewayInvocation,
            other if other.is_empty() => Self::WasmDelegate,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// Policy and capability facts attached to an execution request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecutionPolicyContext {
    pub capability_scope: Vec<String>,
    pub required_permissions: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Generic source category for an autonomous execution envelope.
///
/// The enum deliberately models OS entrypoints, not application business
/// domains. Runtime-host strategies select one of these stable categories so
/// audit tools can distinguish heartbeat, scheduled task, recovery, and direct
/// invocation paths without inspecting raw prompts or app-specific names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousExecutionSourceKind {
    ScheduledAgentTask,
    HeartbeatProfile,
    RuntimeRecovery,
    DirectInvocation,
}

impl AutonomousExecutionSourceKind {
    /// Return the stable wire label used in logs and metadata.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ScheduledAgentTask => "scheduled_agent_task",
            Self::HeartbeatProfile => "heartbeat_profile",
            Self::RuntimeRecovery => "runtime_recovery",
            Self::DirectInvocation => "direct_invocation",
        }
    }
}

/// Instruction priority policy applied by Agent Execution.
///
/// Natural-language user work can remain flexible, but delegated autonomous
/// runs need a deterministic ordering rule: the source task instruction wins
/// over persona, memory, profile, and other contextual material. This policy is
/// auditable data instead of prompt-only wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousInstructionPriority {
    TaskOverridesContext,
}

impl AutonomousInstructionPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TaskOverridesContext => "task_overrides_context",
        }
    }
}

/// Execution strategy hint derived from generic task metadata.
///
/// This is a Strategy selector, not an application workflow branch. The first
/// implementation keeps all autonomous natural-language work interpreted by
/// the agent, while artifact-backed tasks are marked tool-assisted so the agent
/// sees that tools may be necessary inside the existing policy boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousExecutionMode {
    AgentInterpreted,
    ToolAssisted,
    ExactToolCall,
    StructuredReport,
}

impl AutonomousExecutionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AgentInterpreted => "agent_interpreted",
            Self::ToolAssisted => "tool_assisted",
            Self::ExactToolCall => "exact_tool_call",
            Self::StructuredReport => "structured_report",
        }
    }
}

/// Completion policy category for an autonomous run.
///
/// The policy separates "agent responded" from stronger, machine-checkable
/// evidence such as artifacts or structured output. Runtime services can then
/// record wake, dispatch, execution, and evidence states independently instead
/// of treating a model response as final proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousCompletionPolicyKind {
    RequireAgentResult,
    RequireArtifact,
    RequireStructuredOutput,
    BestEffortWithAudit,
}

impl AutonomousCompletionPolicyKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RequireAgentResult => "require_agent_result",
            Self::RequireArtifact => "require_artifact",
            Self::RequireStructuredOutput => "require_structured_output",
            Self::BestEffortWithAudit => "best_effort_with_audit",
        }
    }
}

/// Generic completion policy compiled from task metadata.
///
/// The initial Specification supports the durable artifact evidence already
/// used by autonomy dispatchers. Future policy kinds can extend this struct
/// without changing callers that only need the default agent-result policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousCompletionPolicy {
    pub kind: AutonomousCompletionPolicyKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_artifact_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_contains: Vec<String>,
}

impl AutonomousCompletionPolicy {
    fn require_agent_result() -> Self {
        Self {
            kind: AutonomousCompletionPolicyKind::RequireAgentResult,
            expected_artifact_path: None,
            required_contains: Vec::new(),
        }
    }

    fn require_artifact(path: String, required_contains: Vec<String>) -> Self {
        Self {
            kind: AutonomousCompletionPolicyKind::RequireArtifact,
            expected_artifact_path: Some(path),
            required_contains,
        }
    }
}

/// Deterministic envelope around natural-language autonomous work.
///
/// Users and applications keep writing normal task descriptions. Runtime Host
/// compiles those descriptions plus generic metadata into this envelope so the
/// Agent Execution boundary can apply a stable priority rule, emit safe audit
/// metadata, and evaluate completion evidence without hardcoded application
/// workflows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousExecutionEnvelope {
    pub source_kind: AutonomousExecutionSourceKind,
    pub source_instruction: String,
    pub instruction_priority: AutonomousInstructionPriority,
    pub execution_mode: AutonomousExecutionMode,
    pub completion_policy: AutonomousCompletionPolicy,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl AutonomousExecutionEnvelope {
    /// Compile a minimal envelope from source instruction and generic metadata.
    ///
    /// This Builder-style constructor is intentionally conservative. It does
    /// not infer business semantics from natural language and it does not ask a
    /// model to decide OS policy. It only promotes stable `evidence.*` keys into
    /// completion policy while preserving the original instruction verbatim.
    pub fn compile(
        source_kind: AutonomousExecutionSourceKind,
        source_instruction: impl Into<String>,
        metadata: &BTreeMap<String, String>,
    ) -> MacacaResult<Self> {
        let source_instruction = non_empty(
            source_instruction.into(),
            "autonomous execution envelope source_instruction is required",
        )?;
        let expected_artifact_path = metadata
            .get("evidence.expected_artifact_path")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let required_contains = metadata
            .get("evidence.required_contains")
            .map(|value| {
                value
                    .split('\n')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let (execution_mode, completion_policy) = match expected_artifact_path {
            Some(path) => (
                AutonomousExecutionMode::ToolAssisted,
                AutonomousCompletionPolicy::require_artifact(path, required_contains),
            ),
            None => (
                AutonomousExecutionMode::AgentInterpreted,
                AutonomousCompletionPolicy::require_agent_result(),
            ),
        };
        let mut envelope_metadata = BTreeMap::new();
        envelope_metadata.insert("compiler".into(), "deterministic.v1".into());
        Ok(Self {
            source_kind,
            source_instruction,
            instruction_priority: AutonomousInstructionPriority::TaskOverridesContext,
            execution_mode,
            completion_policy,
            metadata: envelope_metadata,
        })
    }
}

/// Command accepted by `service.agent_execution`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentExecutionCommand {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub task_id: Option<TaskId>,
    pub source_agent: Option<String>,
    pub target_agent: String,
    pub execution_intent: AgentExecutionIntent,
    pub user_prompt: String,
    pub delegated_context: serde_json::Value,
    pub trace: TraceContext,
    pub policy: AgentExecutionPolicyContext,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_control_override: Option<ExecutionControlPolicyOverride>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_envelope: Option<AutonomousExecutionEnvelope>,
    pub metadata: BTreeMap<String, String>,
}

impl AgentExecutionCommand {
    /// Create a validated command from the minimal required execution facts.
    pub fn new(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        target_agent: impl Into<String>,
        execution_intent: AgentExecutionIntent,
        user_prompt: impl Into<String>,
        trace: TraceContext,
    ) -> MacacaResult<Self> {
        validate_trace(&trace)?;
        Ok(Self {
            application_id,
            session_id: non_empty(session_id.into(), "agent execution session_id is required")?,
            task_id: None,
            source_agent: None,
            target_agent: non_empty(
                target_agent.into(),
                "agent execution target_agent is required",
            )?,
            execution_intent,
            user_prompt: non_empty(
                user_prompt.into(),
                "agent execution user_prompt is required",
            )?,
            delegated_context: serde_json::json!({}),
            trace,
            policy: AgentExecutionPolicyContext::default(),
            execution_control_override: None,
            execution_envelope: None,
            metadata: BTreeMap::new(),
        })
    }

    /// Attach bounded delegated context without changing trusted system prompt
    /// ownership.
    pub fn with_delegated_context(mut self, context: serde_json::Value) -> Self {
        self.delegated_context = context;
        self
    }

    /// Attach a per-run execution-control override.
    ///
    /// The command only records the request. Runtime policy resolution still
    /// verifies that this override is allowed by the application declaration
    /// before any pause/resume side effects are installed.
    pub fn with_execution_control_override(
        mut self,
        override_policy: ExecutionControlPolicyOverride,
    ) -> Self {
        self.execution_control_override = Some(override_policy);
        self
    }

    /// Attach an autonomous execution envelope compiled by a runtime strategy.
    pub fn with_execution_envelope(mut self, envelope: AutonomousExecutionEnvelope) -> Self {
        self.execution_envelope = Some(envelope);
        self
    }

    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(AGENT_EXECUTE_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

/// Command accepted by `service.agent_context`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentContextBuildCommand {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub task_id: Option<TaskId>,
    pub target_agent: String,
    pub execution_intent: AgentExecutionIntent,
    pub trace: TraceContext,
    pub policy: AgentExecutionPolicyContext,
    pub context_budget_tokens: Option<u32>,
    pub metadata: BTreeMap<String, String>,
}

impl AgentContextBuildCommand {
    /// Build context for the agent targeted by an execution command.
    pub fn from_execution(command: &AgentExecutionCommand) -> Self {
        Self {
            application_id: command.application_id,
            session_id: command.session_id.clone(),
            task_id: command.task_id,
            target_agent: command.target_agent.clone(),
            execution_intent: command.execution_intent.clone(),
            trace: command.trace.clone(),
            policy: command.policy.clone(),
            context_budget_tokens: None,
            metadata: command.metadata.clone(),
        }
    }

    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(AGENT_CONTEXT_BUILD_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

/// Sanitized source evidence used to compose trusted agent context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentContextSource {
    pub kind: String,
    pub name: String,
    pub location: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Replayable result of trusted system-context construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentContextSnapshot {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub task_id: Option<TaskId>,
    pub target_agent: String,
    pub system_prompt: String,
    pub sources: Vec<AgentContextSource>,
    pub visible_skills: Vec<String>,
    pub filtered_skills: Vec<String>,
    pub tool_policy: BTreeMap<String, String>,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

impl AgentContextSnapshot {
    /// Build a minimal snapshot for unavailable/degraded providers and tests.
    pub fn minimal(command: &AgentContextBuildCommand, system_prompt: impl Into<String>) -> Self {
        Self {
            application_id: command.application_id,
            session_id: command.session_id.clone(),
            task_id: command.task_id,
            target_agent: command.target_agent.clone(),
            system_prompt: system_prompt.into(),
            sources: Vec::new(),
            visible_skills: Vec::new(),
            filtered_skills: Vec::new(),
            tool_policy: BTreeMap::new(),
            trace: command.trace.clone(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Status returned by the Agent Execution service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentExecutionStatus {
    Completed,
    Failed,
    Denied,
    Unavailable,
    Unsupported,
    Skipped,
}

impl AgentExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Denied => "denied",
            Self::Unavailable => "unavailable",
            Self::Unsupported => "unsupported",
            Self::Skipped => "skipped",
        }
    }
}

/// Result returned by `service.agent_execution`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentExecutionResult {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub task_id: Option<TaskId>,
    pub target_agent: String,
    pub status: AgentExecutionStatus,
    pub output: serde_json::Value,
    pub context_snapshot: Option<AgentContextSnapshot>,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

impl AgentExecutionResult {
    /// Build a successful result with bounded, provider-neutral output.
    pub fn completed(command: &AgentExecutionCommand, output: serde_json::Value) -> Self {
        Self {
            application_id: command.application_id,
            session_id: command.session_id.clone(),
            task_id: command.task_id,
            target_agent: command.target_agent.clone(),
            status: AgentExecutionStatus::Completed,
            output,
            context_snapshot: None,
            trace: command.trace.clone(),
            metadata: BTreeMap::new(),
        }
    }

    /// Build a structured skip result for policy or context preconditions.
    ///
    /// Skips are successful service replies with a non-completed status. The
    /// output and metadata carry only bounded reason codes so callers can audit
    /// why no model/tool invocation happened without leaking prompts or raw
    /// context bodies.
    pub fn skipped(
        command: &AgentExecutionCommand,
        reason_code: impl Into<String>,
        context_snapshot: Option<AgentContextSnapshot>,
    ) -> Self {
        let reason_code = reason_code.into();
        let mut metadata = BTreeMap::new();
        metadata.insert("reason_code".into(), reason_code.clone());
        Self {
            application_id: command.application_id,
            session_id: command.session_id.clone(),
            task_id: command.task_id,
            target_agent: command.target_agent.clone(),
            status: AgentExecutionStatus::Skipped,
            output: serde_json::json!({ "reason_code": reason_code }),
            context_snapshot,
            trace: command.trace.clone(),
            metadata,
        }
    }
}

fn validate_trace(trace: &TraceContext) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Agent(
            "agent service command requires trace_id".into(),
        ));
    }
    Ok(())
}

fn non_empty(value: String, message: &str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        Err(MacacaError::Agent(message.into()))
    } else {
        Ok(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ExecutionControlCheckpointMode, ExecutionControlPolicyOverride,
        ExecutionControlResumeSource, ExecutionControlTrigger,
    };

    /// Provider-neutral fixture agent id for round-trip contract tests (Object Mother).
    ///
    /// Intentionally avoids legacy application role literals (`worker`, `planner`, etc.)
    /// so terminal `hardcoded-agent-role` guards can scan this module without false
    /// positives while still exercising `target_agent` serialization boundaries.
    const FIXTURE_TARGET_AGENT: &str = "fixture-target-agent";

    /// Provider-neutral heartbeat artifact path paired with [`FIXTURE_TARGET_AGENT`].
    ///
    /// Envelope compiler tests only assert path round-trip semantics; the directory
    /// segment must not embed application role vocabulary.
    const FIXTURE_HEARTBEAT_ARTIFACT_PATH: &str =
        "/workspace/agents/fixture-target-agent/heartbeat.md";

    #[test]
    fn planner_intent_uses_provider_neutral_wire_label() {
        assert_eq!(
            AgentExecutionIntent::Planner.metadata_value(),
            GOAL_PLANNER_EXECUTION_INTENT_LABEL
        );
        assert_eq!(
            AgentExecutionIntent::from_metadata_value(GOAL_PLANNER_EXECUTION_INTENT_LABEL),
            AgentExecutionIntent::Planner
        );
    }

    #[test]
    fn execution_command_round_trips_through_service_command() {
        let mut trace = TraceContext::new("trace-agent-exec");
        trace.session_id = Some("session-a".into());
        let command = AgentExecutionCommand::new(
            ApplicationId::from_name("demo"),
            "session-a",
            FIXTURE_TARGET_AGENT,
            AgentExecutionIntent::WasmDelegate,
            "Analyze BTC",
            trace,
        )
        .unwrap()
        .with_delegated_context(serde_json::json!({"symbol": "BTC"}));

        let service_command = command.clone().into_service_command().unwrap();
        assert_eq!(service_command.name.as_str(), AGENT_EXECUTE_COMMAND);
        assert!(service_command.trace.is_some());

        let decoded: AgentExecutionCommand =
            serde_json::from_value(service_command.payload).unwrap();
        assert_eq!(decoded.user_prompt, "Analyze BTC");
        assert_eq!(decoded.delegated_context["symbol"], "BTC");
        assert_eq!(decoded.target_agent, FIXTURE_TARGET_AGENT);
    }

    #[test]
    fn execution_envelope_compiler_requires_artifact_when_expected_path_exists() {
        let metadata = BTreeMap::from([(
            "evidence.expected_artifact_path".into(),
            FIXTURE_HEARTBEAT_ARTIFACT_PATH.into(),
        )]);

        let envelope = AutonomousExecutionEnvelope::compile(
            AutonomousExecutionSourceKind::HeartbeatProfile,
            "Write the heartbeat sentinel.",
            &metadata,
        )
        .unwrap();

        assert_eq!(
            envelope.source_kind,
            AutonomousExecutionSourceKind::HeartbeatProfile
        );
        assert_eq!(
            envelope.instruction_priority,
            AutonomousInstructionPriority::TaskOverridesContext
        );
        assert_eq!(
            envelope.completion_policy.kind,
            AutonomousCompletionPolicyKind::RequireArtifact
        );
        assert_eq!(
            envelope.completion_policy.expected_artifact_path.as_deref(),
            Some(FIXTURE_HEARTBEAT_ARTIFACT_PATH)
        );
    }

    #[test]
    fn execution_command_roundtrips_execution_envelope() {
        let mut command = AgentExecutionCommand::new(
            ApplicationId::from_name("demo"),
            "session-a",
            FIXTURE_TARGET_AGENT,
            AgentExecutionIntent::TaskWorker,
            "Summarize project state",
            TraceContext::new("trace-execution-envelope"),
        )
        .unwrap();
        command.execution_envelope = Some(
            AutonomousExecutionEnvelope::compile(
                AutonomousExecutionSourceKind::ScheduledAgentTask,
                "Summarize project state",
                &BTreeMap::new(),
            )
            .unwrap(),
        );

        let encoded = serde_json::to_string(&command).unwrap();
        let decoded: AgentExecutionCommand = serde_json::from_str(&encoded).unwrap();
        let envelope = decoded.execution_envelope.unwrap();

        assert_eq!(
            envelope.source_kind,
            AutonomousExecutionSourceKind::ScheduledAgentTask
        );
        assert_eq!(envelope.source_instruction, "Summarize project state");
        assert_eq!(
            envelope.completion_policy.kind,
            AutonomousCompletionPolicyKind::RequireAgentResult
        );
    }

    #[test]
    fn context_command_preserves_user_prompt_boundary() {
        let command = AgentExecutionCommand::new(
            ApplicationId::from_name("demo"),
            "session-a",
            "risk_manager",
            AgentExecutionIntent::WasmDelegate,
            "This is user work, not a system prompt",
            TraceContext::new("trace-agent-context"),
        )
        .unwrap();

        let context_command = AgentContextBuildCommand::from_execution(&command);
        let snapshot = AgentContextSnapshot::minimal(&context_command, "trusted system context");

        assert_eq!(snapshot.system_prompt, "trusted system context");
        assert_eq!(
            command.user_prompt,
            "This is user work, not a system prompt"
        );
    }

    #[test]
    fn constructor_rejects_empty_required_fields() {
        let err = AgentExecutionCommand::new(
            ApplicationId::from_name("demo"),
            " ",
            FIXTURE_TARGET_AGENT,
            AgentExecutionIntent::ChatMainThread,
            "hello",
            TraceContext::new("trace-invalid"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("session_id"));
    }

    #[test]
    fn execution_command_roundtrips_execution_control_override() {
        let command = AgentExecutionCommand::new(
            ApplicationId::from_name("demo"),
            "session-a",
            FIXTURE_TARGET_AGENT,
            AgentExecutionIntent::TaskWorker,
            "pause after delegated work reaches a barrier",
            TraceContext::new("trace-execution-control-override"),
        )
        .unwrap()
        .with_execution_control_override(ExecutionControlPolicyOverride::enable_for_run(
            vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::ReferenceOnly,
        ));

        let encoded = serde_json::to_string(&command).unwrap();
        let decoded: AgentExecutionCommand = serde_json::from_str(&encoded).unwrap();
        let override_policy = decoded.execution_control_override.unwrap();

        assert_eq!(override_policy.triggers.len(), 1);
        assert_eq!(override_policy.resume_sources.len(), 1);
        assert_eq!(
            override_policy.checkpoint_mode,
            ExecutionControlCheckpointMode::ReferenceOnly
        );
    }
}
