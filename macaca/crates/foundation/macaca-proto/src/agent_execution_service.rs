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
    SdkInvocation,
    GatewayInvocation,
    Custom(String),
}

/// Policy and capability facts attached to an execution request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecutionPolicyContext {
    pub capability_scope: Vec<String>,
    pub required_permissions: Vec<String>,
    pub metadata: BTreeMap<String, String>,
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
}

impl AgentExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Denied => "denied",
            Self::Unavailable => "unavailable",
            Self::Unsupported => "unsupported",
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

    #[test]
    fn execution_command_round_trips_through_service_command() {
        let mut trace = TraceContext::new("trace-agent-exec");
        trace.session_id = Some("session-a".into());
        let command = AgentExecutionCommand::new(
            ApplicationId::from_name("demo"),
            "session-a",
            "worker",
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
        assert_eq!(decoded.target_agent, "worker");
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
            "worker",
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
            "worker",
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
