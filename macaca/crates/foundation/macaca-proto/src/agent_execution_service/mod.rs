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
    ApplicationId, ExecutionControlPolicyOverride, MacacaError, MacacaResult, TaskId,
    TraceContext,
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
    pub fn from_delegate_metadata(metadata: &BTreeMap<String, String>) -> Self {
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

/// Autonomous execution envelope types (Specification + Builder compiler).
mod autonomous_envelope;
pub use autonomous_envelope::*;

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

/// Adapter layer: typed agent-execution commands → provider-neutral `ServiceCommand`.
mod command_adapters;

fn validate_trace(trace: &TraceContext) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Agent(
            "agent service command requires trace_id".into(),
        ));
    }
    Ok(())
}

/// Shared non-empty string validator for command constructors and envelope compiler.
pub(super) fn non_empty(value: String, message: &str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        Err(MacacaError::Agent(message.into()))
    } else {
        Ok(trimmed)
    }
}

#[cfg(test)]
mod tests;
