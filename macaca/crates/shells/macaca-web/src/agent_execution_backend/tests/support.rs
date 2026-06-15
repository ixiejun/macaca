//! Shared imports for agent execution backend contract tests.
//!
//! Centralizes external crate imports so each test submodule stays focused on
//! assertions rather than repetitive `use` blocks (Facade + Module pattern).

pub(crate) use std::path::{Path, PathBuf};

pub(crate) use macaca_host_composition::agent_execution::{
    execution_control_execution_id, execution_control_scope, extract_single_shell_fence,
    heartbeat_exact_shell_contract, resolve_execution_control_policy_local,
    runtime_agent_max_iters, runtime_agent_tool_choice, should_skip_heartbeat_without_source,
    user_prompt_with_context,
};
pub(crate) use macaca_proto::{
    AgentContextBuildCommand, AgentContextSnapshot, AgentExecutionCommand, AgentExecutionIntent,
    ExecutionControlCheckpointMode, ExecutionControlPolicy, ExecutionControlPolicyOverride,
    ExecutionControlResolutionStatus, ExecutionControlResumeSource, ExecutionControlTrigger,
};
