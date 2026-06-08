//! Autonomous execution envelope types and deterministic compiler.
//!
//! **Pattern:** Specification + Builder — runtime-host strategies compile generic task
//! metadata into auditable envelope data without inferring application business semantics
//! from natural-language prompts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::MacacaResult;

use super::non_empty;

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
