//! SDK Tool Service client for the industrial Tool Capability Plane.
//!
//! `SystemToolClient` is a focused Facade over `service.tool`.  The SDK keeps
//! this boundary deliberately thin: it serializes typed Command DTOs, forwards
//! them through the generic service client, and deserializes typed Result DTOs.
//! It never constructs tool providers, evaluates availability, interprets
//! application manifests, or owns Driver/Skill/MCP/Memory/Scheduler lifecycles.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    MacacaError, MacacaResult, ServiceHealth, ToolArtifactOpenCommand, ToolArtifactOpenResult,
    ToolAuditQueryCommand, ToolAuditQueryResult, ToolCatalogPlanCommand, ToolCatalogPlanResult,
    ToolCatalogSnapshotCommand, ToolCatalogSnapshotResult, ToolCommandResult,
    ToolGenericTraceCommand, ToolInvocationStatusCommand, ToolInvocationStatusResult,
    ToolInvokeCancelCommand, ToolInvokeCancelResult, ToolInvokeCommand, ToolInvokeResult,
    ToolPolicyExplainCommand, ToolPolicyExplainResult, ToolProviderHealthResult,
    ToolProviderStatusCommand, ToolProviderStatusResult, ToolResultClass, ToolResultGetCommand,
    ToolResultGetResult, ToolServiceHealthResult, ToolServiceSnapshotResult,
    ToolToolsetResolveCommand, ToolToolsetResolveResult, TraceContext, TOOL_ARTIFACT_OPEN_COMMAND,
    TOOL_AUDIT_QUERY_COMMAND, TOOL_CATALOG_PLAN_COMMAND, TOOL_CATALOG_SNAPSHOT_COMMAND,
    TOOL_INVOCATION_STATUS_COMMAND, TOOL_INVOKE_CANCEL_COMMAND, TOOL_INVOKE_COMMAND,
    TOOL_POLICY_EXPLAIN_COMMAND, TOOL_PROVIDER_HEALTH_COMMAND, TOOL_PROVIDER_STATUS_COMMAND,
    TOOL_RESULT_GET_COMMAND, TOOL_SERVICE_ID, TOOL_TOOLSET_RESOLVE_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused client for provider-neutral Tool Capability Plane operations.
///
/// This trait uses the Command pattern at the SDK boundary.  A runtime-backed
/// implementation can route to `service.tool`; a Null Object can return
/// structured unavailable results.  Callers do not need to know whether the
/// service is local, remote, plugin-backed, or absent.
#[async_trait]
pub trait SystemToolClient: Send + Sync {
    async fn catalog_plan(
        &self,
        command: ToolCatalogPlanCommand,
    ) -> MacacaResult<ToolCatalogPlanResult>;
    async fn catalog_snapshot(
        &self,
        command: ToolCatalogSnapshotCommand,
    ) -> MacacaResult<ToolCatalogSnapshotResult>;
    async fn toolset_resolve(
        &self,
        command: ToolToolsetResolveCommand,
    ) -> MacacaResult<ToolToolsetResolveResult>;
    async fn invoke(&self, command: ToolInvokeCommand) -> MacacaResult<ToolInvokeResult>;
    async fn cancel(
        &self,
        command: ToolInvokeCancelCommand,
    ) -> MacacaResult<ToolInvokeCancelResult>;
    async fn invocation_status(
        &self,
        command: ToolInvocationStatusCommand,
    ) -> MacacaResult<ToolInvocationStatusResult>;
    async fn result_get(&self, command: ToolResultGetCommand) -> MacacaResult<ToolResultGetResult>;
    async fn artifact_open(
        &self,
        command: ToolArtifactOpenCommand,
    ) -> MacacaResult<ToolArtifactOpenResult>;
    async fn provider_status(
        &self,
        command: ToolProviderStatusCommand,
    ) -> MacacaResult<ToolProviderStatusResult>;
    async fn provider_health(&self, trace: TraceContext) -> MacacaResult<ToolProviderHealthResult>;
    async fn policy_explain(
        &self,
        command: ToolPolicyExplainCommand,
    ) -> MacacaResult<ToolPolicyExplainResult>;
    async fn audit_query(
        &self,
        command: ToolAuditQueryCommand,
    ) -> MacacaResult<ToolAuditQueryResult>;
}

/// Null-object Tool client used when `service.tool` is absent.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemToolClient;

#[async_trait]
impl SystemToolClient for UnavailableSystemToolClient {
    async fn catalog_plan(
        &self,
        command: ToolCatalogPlanCommand,
    ) -> MacacaResult<ToolCatalogPlanResult> {
        info!(trace_id = %command.trace.trace_id, "sdk tool client returning empty unavailable plan");
        Ok(ToolCatalogPlanResult::empty(command.trace))
    }

    async fn catalog_snapshot(
        &self,
        command: ToolCatalogSnapshotCommand,
    ) -> MacacaResult<ToolCatalogSnapshotResult> {
        info!(trace_id = %command.trace.trace_id, "sdk tool client returning unavailable snapshot");
        Ok(ToolServiceSnapshotResult::unavailable(
            command.trace,
            unavailable_reason(),
        ))
    }

    async fn toolset_resolve(
        &self,
        command: ToolToolsetResolveCommand,
    ) -> MacacaResult<ToolToolsetResolveResult> {
        info!(trace_id = %command.trace.trace_id, "sdk tool client returning empty toolset resolution");
        Ok(ToolCatalogPlanResult::empty(command.trace))
    }

    async fn invoke(&self, command: ToolInvokeCommand) -> MacacaResult<ToolInvokeResult> {
        warn!(trace_id = %command.trace.trace_id, tool_id = %command.tool_id, "sdk tool client unavailable for invoke");
        Err(unavailable_error())
    }

    async fn cancel(
        &self,
        command: ToolInvokeCancelCommand,
    ) -> MacacaResult<ToolInvokeCancelResult> {
        Ok(unavailable_command_result(command.trace))
    }

    async fn invocation_status(
        &self,
        command: ToolInvocationStatusCommand,
    ) -> MacacaResult<ToolInvocationStatusResult> {
        Ok(unavailable_command_result(command.trace))
    }

    async fn result_get(&self, command: ToolResultGetCommand) -> MacacaResult<ToolResultGetResult> {
        Ok(unavailable_command_result(command.trace))
    }

    async fn artifact_open(
        &self,
        command: ToolArtifactOpenCommand,
    ) -> MacacaResult<ToolArtifactOpenResult> {
        Ok(unavailable_command_result(command.trace))
    }

    async fn provider_status(
        &self,
        command: ToolProviderStatusCommand,
    ) -> MacacaResult<ToolProviderStatusResult> {
        Ok(ToolServiceSnapshotResult::unavailable(
            command.trace,
            unavailable_reason(),
        ))
    }

    async fn provider_health(&self, trace: TraceContext) -> MacacaResult<ToolProviderHealthResult> {
        info!(trace_id = %trace.trace_id, "sdk tool client returning unavailable health");
        Ok(ToolServiceHealthResult {
            trace,
            health: ServiceHealth::Unavailable {
                reason: unavailable_reason().into(),
            },
            provider_count: 0,
            captured_at: chrono::Utc::now(),
            metadata: Default::default(),
        })
    }

    async fn policy_explain(
        &self,
        command: ToolPolicyExplainCommand,
    ) -> MacacaResult<ToolPolicyExplainResult> {
        Ok(unavailable_command_result(command.trace))
    }

    async fn audit_query(
        &self,
        command: ToolAuditQueryCommand,
    ) -> MacacaResult<ToolAuditQueryResult> {
        Ok(unavailable_command_result(command.trace))
    }
}

/// Runtime-backed Tool client implemented over the generic service dispatcher.
#[derive(Clone)]
pub struct ServiceBackedToolClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedToolClient {
    /// Create a service-backed client from an existing service dispatcher.
    ///
    /// This is an Adapter over `SystemServiceClient`; it does not construct a
    /// runtime host or provider.  Composition roots decide which dispatcher is
    /// installed.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemToolClient for ServiceBackedToolClient {
    async fn catalog_plan(
        &self,
        command: ToolCatalogPlanCommand,
    ) -> MacacaResult<ToolCatalogPlanResult> {
        call(
            &self.service,
            TOOL_CATALOG_PLAN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn catalog_snapshot(
        &self,
        command: ToolCatalogSnapshotCommand,
    ) -> MacacaResult<ToolCatalogSnapshotResult> {
        call(
            &self.service,
            TOOL_CATALOG_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn toolset_resolve(
        &self,
        command: ToolToolsetResolveCommand,
    ) -> MacacaResult<ToolToolsetResolveResult> {
        call(
            &self.service,
            TOOL_TOOLSET_RESOLVE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn invoke(&self, command: ToolInvokeCommand) -> MacacaResult<ToolInvokeResult> {
        call(
            &self.service,
            TOOL_INVOKE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn cancel(
        &self,
        command: ToolInvokeCancelCommand,
    ) -> MacacaResult<ToolInvokeCancelResult> {
        call(
            &self.service,
            TOOL_INVOKE_CANCEL_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn invocation_status(
        &self,
        command: ToolInvocationStatusCommand,
    ) -> MacacaResult<ToolInvocationStatusResult> {
        call(
            &self.service,
            TOOL_INVOCATION_STATUS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn result_get(&self, command: ToolResultGetCommand) -> MacacaResult<ToolResultGetResult> {
        call(
            &self.service,
            TOOL_RESULT_GET_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn artifact_open(
        &self,
        command: ToolArtifactOpenCommand,
    ) -> MacacaResult<ToolArtifactOpenResult> {
        call(
            &self.service,
            TOOL_ARTIFACT_OPEN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn provider_status(
        &self,
        command: ToolProviderStatusCommand,
    ) -> MacacaResult<ToolProviderStatusResult> {
        call(
            &self.service,
            TOOL_PROVIDER_STATUS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn provider_health(&self, trace: TraceContext) -> MacacaResult<ToolProviderHealthResult> {
        let command = ToolGenericTraceCommand::new(trace)?;
        call(
            &self.service,
            TOOL_PROVIDER_HEALTH_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn policy_explain(
        &self,
        command: ToolPolicyExplainCommand,
    ) -> MacacaResult<ToolPolicyExplainResult> {
        call(
            &self.service,
            TOOL_POLICY_EXPLAIN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn audit_query(
        &self,
        command: ToolAuditQueryCommand,
    ) -> MacacaResult<ToolAuditQueryResult> {
        call(
            &self.service,
            TOOL_AUDIT_QUERY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let service_command = ServiceCallCommand::new(
        TOOL_SERVICE_ID,
        command_name,
        serde_json::to_value(payload)?,
    )?
    .with_trace(trace);
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}

fn unavailable_command_result(trace: TraceContext) -> ToolCommandResult {
    ToolCommandResult {
        trace,
        status: "unavailable".into(),
        result_class: ToolResultClass::Failure,
        inline_output: None,
        artifact_refs: Vec::new(),
        invocation_ref: None,
        audit_refs: Vec::new(),
        error_summary: Some(unavailable_reason().into()),
        captured_at: chrono::Utc::now(),
        metadata: Default::default(),
    }
}

fn unavailable_error() -> MacacaError {
    MacacaError::Config(unavailable_reason().into())
}

fn unavailable_reason() -> &'static str {
    "Tool service is unavailable"
}
