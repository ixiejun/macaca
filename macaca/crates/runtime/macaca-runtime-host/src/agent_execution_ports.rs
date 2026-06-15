//! Host adapter ports for `service.agent_execution`.
//!
//! Runtime-host owns the **composed** execution backend and provider-neutral
//! orchestration.  Shell-specific wiring (SSE sessions, kernel executor
//! lifecycle broadcasts, and host-local framework materialization) is injected
//! through these ports using the Adapter pattern so Web/CLI can remain thin
//! transports while the service provider stays in runtime-host.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_framework::model::ToolChoice;
use macaca_proto::{
    AgentContextSnapshot, AgentExecutionCommand, AgentExecutionEvent, AgentExecutionResult,
    ExecutionControlPolicy, ExecutionControlResolvedPolicy, ServiceResult, TaskId,
};
use tokio::sync::mpsc;
use tracing::info;

/// Opaque pause/resume handle installed by the shell and consumed by the framework port.
///
/// Runtime-host never inspects the inner type; the framework adapter downcasts to
/// its host-specific execution-control middleware.
pub struct OpaqueExecutionControlHandle(pub Arc<dyn std::any::Any + Send + Sync>);

/// Collects post-run evidence metadata after one delegated agent execution.
#[async_trait]
pub trait AgentExecutionEvidenceCollector: Send {
    /// Flush observer state into provider-neutral result metadata.
    async fn finish(self: Box<Self>) -> BTreeMap<String, String>;
}

/// Type-erased ReAct agent produced by a shell-specific construction adapter.
///
/// Runtime-host owns the execution loop (`reply`) while the shell adapter owns
/// how toolkit/model/hooks are wired.  This split keeps framework construction
/// injectable without pulling shell types into the service provider.
#[async_trait]
pub trait ConstructedRuntimeAgent: Send {
    /// Run one user turn through the framework ReAct loop and return text output.
    async fn reply_user_prompt(&self, user_prompt: String) -> Result<String, String>;
}

/// Host materialization port for runtime-host owned framework construction.
///
/// Shells implement this lower-level port to supply host-local composition
/// dependencies such as toolkit assembly, SSE bridges, and execution-control
/// middleware. Runtime-host owns the construction service that calls this port;
/// shells no longer implement the public construction port directly.
#[async_trait]
pub trait FrameworkAgentMaterializationPort: Send + Sync {
    /// Materialize one runtime ReAct agent from an already trusted snapshot.
    async fn build_runtime_react_agent(
        &self,
        command: &AgentExecutionCommand,
        context_snapshot: &AgentContextSnapshot,
        agent_event_tx: mpsc::Sender<AgentExecutionEvent>,
        execution_control: Option<OpaqueExecutionControlHandle>,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
    ) -> Result<Box<dyn ConstructedRuntimeAgent>, String>;
}

/// Construction port: provider-neutral runtime-host service boundary.
///
/// Runtime-host implementations own construction tracing, failure reporting,
/// and the stable contract between `service.agent_execution` and framework
/// materialization. Host adapters stay behind [`FrameworkAgentMaterializationPort`].
#[async_trait]
pub trait FrameworkAgentConstructionPort: Send + Sync {
    /// Build one runtime ReAct agent from an Agent Context service snapshot.
    ///
    /// The snapshot is authoritative for persona/system prompt; this method must
    /// not rebuild context and should only wire model/tools/hooks for execution.
    async fn build_runtime_react_agent(
        &self,
        command: &AgentExecutionCommand,
        context_snapshot: &AgentContextSnapshot,
        agent_event_tx: mpsc::Sender<AgentExecutionEvent>,
        execution_control: Option<OpaqueExecutionControlHandle>,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
    ) -> Result<Box<dyn ConstructedRuntimeAgent>, String>;
}

/// Framework port: build and run one ReAct agent from a trusted context snapshot.
#[async_trait]
pub trait FrameworkRuntimeAgentPort: Send + Sync {
    /// Execute one model/tool loop and return the final natural-language output.
    async fn run_react_agent(
        &self,
        command: &AgentExecutionCommand,
        context_snapshot: &AgentContextSnapshot,
        agent_event_tx: mpsc::Sender<AgentExecutionEvent>,
        execution_control: Option<OpaqueExecutionControlHandle>,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
        user_prompt: String,
    ) -> Result<String, String>;
}

/// Shell port: lifecycle, execution-control service wiring, and evidence bridges.
#[async_trait]
pub trait AgentExecutionHostAdapter: Send + Sync {
    /// Resolve execution-control policy through `service.execution_control`.
    async fn resolve_execution_control_policy(
        &self,
        command: &AgentExecutionCommand,
    ) -> ServiceResult<ExecutionControlResolvedPolicy>;

    /// Register an enabled execution with the execution-control service.
    async fn register_execution_control(
        &self,
        command: &AgentExecutionCommand,
        policy: ExecutionControlResolvedPolicy,
    ) -> ServiceResult<Option<(ExecutionControlPolicy, String)>>;

    /// Install in-process pause/resume channels for this session run.
    async fn install_execution_control(
        &self,
        command: &AgentExecutionCommand,
        policy: ExecutionControlPolicy,
        execution_id: String,
    ) -> Option<OpaqueExecutionControlHandle>;

    /// Emit coarse lifecycle + kernel status at run start.
    async fn on_run_started(
        &self,
        command: &AgentExecutionCommand,
        task_id: TaskId,
        emit_lifecycle: bool,
    );

    /// Emit coarse lifecycle + kernel status after successful completion.
    async fn on_run_completed(
        &self,
        command: &AgentExecutionCommand,
        task_id: TaskId,
        emit_lifecycle: bool,
        output_preview: &str,
    );

    /// Emit coarse lifecycle + kernel status after failure.
    async fn on_run_failed(
        &self,
        command: &AgentExecutionCommand,
        task_id: TaskId,
        emit_lifecycle: bool,
        error: &str,
    );

    /// Mirror the run request into application-execution audit when configured.
    async fn observe_application_execution_requested(&self, command: &AgentExecutionCommand);

    /// Create the fine-grained agent-event channel and evidence collector.
    async fn create_evidence_observer(
        &self,
        command: &AgentExecutionCommand,
        task_id: TaskId,
    ) -> (
        mpsc::Sender<AgentExecutionEvent>,
        Box<dyn AgentExecutionEvidenceCollector>,
    );
}

/// Compute a stable output hash for audit replay.
pub trait AgentExecutionOutputHasher: Send + Sync {
    fn stable_output_hash(&self, output: &serde_json::Value) -> String;
}

/// Default hasher placeholder used until hosts inject a crypto-stable implementation.
pub struct PassthroughOutputHasher;

impl AgentExecutionOutputHasher for PassthroughOutputHasher {
    fn stable_output_hash(&self, output: &serde_json::Value) -> String {
        output.to_string()
    }
}

/// Log one composed backend construction for traceability.
pub fn log_composed_backend_wired(bus_source: &str) {
    info!(
        bus_source = %bus_source,
        pattern = "adapter+strategy",
        "composed agent execution backend wired with host and framework ports"
    );
}

/// Helper to build a completed result with context snapshot and metadata extensions.
pub fn completed_execution_result(
    command: &AgentExecutionCommand,
    task_id: TaskId,
    context_snapshot: AgentContextSnapshot,
    output: serde_json::Value,
    output_hash: String,
    extra_metadata: BTreeMap<String, String>,
) -> AgentExecutionResult {
    let mut result = AgentExecutionResult::completed(
        &AgentExecutionCommand {
            task_id: Some(task_id),
            ..command.clone()
        },
        output,
    );
    result.context_snapshot = Some(context_snapshot);
    result
        .metadata
        .insert("result_output_hash".into(), output_hash);
    result.metadata.extend(extra_metadata);
    result
}
