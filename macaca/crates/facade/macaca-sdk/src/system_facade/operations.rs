//! Audited shell-facing operations delegated through the composed Strategy clients.
//!
//! Each method logs a trace-correlated audit node before forwarding to the
//! injected client.  The facade never constructs providers or encodes application
//! workflows — it only routes typed commands across SDK boundaries.

use tracing::info;

use macaca_proto::MacacaResult;

use crate::context_client::SystemContextClient;
use crate::llm_client::SystemLlmClient;
use crate::memory_client::SystemMemoryClient;
use crate::package_client::{PackageInspectionCommand, PackageInspectionResult, SystemPackageClient};
use crate::service_client::{
    ServiceCallCommand, ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    SystemServiceClient,
};
use crate::status_client::{SystemStatusClient, SystemStatusSnapshot};
use crate::task_client::{SystemTaskClient, TaskBoardQueryCommand, TaskBoardQueryResult};
use crate::trace_client::{
    SessionEventQueryCommand, SystemTraceClient, TraceQueryResult, TraceTailCommand,
};

impl<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
    super::types::SystemFacade<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
where
    T: SystemTaskClient,
    S: SystemStatusClient,
    SV: SystemServiceClient,
    TR: SystemTraceClient,
    P: SystemPackageClient,
    L: SystemLlmClient,
    M: SystemMemoryClient,
    C: SystemContextClient,
{
    /// Query a session-scoped task board through the facade boundary.
    pub async fn query_task_board(
        &self,
        command: TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            "system facade task board query started"
        );
        let result = self.task_board.query_task_board(&command).await?;
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            count = result.count,
            "system facade task board query completed"
        );
        Ok(result)
    }

    /// Return a shell-facing status snapshot without exposing presentation internals.
    pub async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot> {
        info!("system facade status snapshot requested");
        self.status.status_snapshot().await
    }

    /// Inspect service availability through the SDK service client boundary.
    pub async fn inspect_services(
        &self,
        command: ServiceInspectionCommand,
    ) -> MacacaResult<ServiceInspectionResult> {
        info!(
            scope = %command.scope,
            "system facade service inspection started"
        );
        self.service.inspect_services(&command).await
    }

    /// Dispatch a service command through the replaceable SDK service client.
    pub async fn call_service(
        &self,
        command: ServiceCallCommand,
    ) -> MacacaResult<ServiceCallResult> {
        info!(
            service_id = %command.service_id,
            command = %command.command_name,
            "system facade service call started"
        );
        self.service.call_service(&command).await
    }

    /// Replay session events through the replaceable trace client.
    pub async fn replay_events(
        &self,
        command: SessionEventQueryCommand,
    ) -> MacacaResult<TraceQueryResult> {
        info!(
            session_id = %command.session_id,
            since = ?command.since,
            limit = ?command.limit,
            "system facade trace replay started"
        );
        self.trace.replay_events(&command).await
    }

    /// Tail session trace events through the replaceable trace client.
    pub async fn tail_trace(&self, command: TraceTailCommand) -> MacacaResult<TraceQueryResult> {
        info!(
            session_id = %command.session_id,
            since = ?command.since,
            "system facade trace tail started"
        );
        self.trace.tail_trace(&command).await
    }

    /// Inspect packages through the replaceable package client.
    pub async fn inspect_packages(
        &self,
        command: PackageInspectionCommand,
    ) -> MacacaResult<PackageInspectionResult> {
        info!(
            package_ref = ?command.package_ref,
            "system facade package inspection started"
        );
        self.package.inspect_packages(&command).await
    }

    /// Dispatch a typed LLM chat command through the focused LLM client.
    pub async fn llm_chat(
        &self,
        command: macaca_llm::LlmChatCommand,
    ) -> MacacaResult<macaca_llm::LlmChatResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "system facade llm chat started"
        );
        self.llm.chat(command).await
    }

    /// Recall scoped memory through the focused Memory client.
    pub async fn memory_recall(
        &self,
        command: macaca_memory::MemoryRecallCommand,
    ) -> MacacaResult<macaca_memory::MemoryRecallResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "system facade memory recall started"
        );
        self.memory.recall(command).await
    }

    /// Assemble model context through the focused Context client.
    pub async fn assemble_context(
        &self,
        command: macaca_context::ContextAssembleCommand,
    ) -> MacacaResult<macaca_context::ContextAssembleServiceResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "system facade context assembly started"
        );
        self.context.assemble(command).await
    }
}
