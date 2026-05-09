//! Runtime-host adapter for the Route C MCP Service.
//!
//! The adapter owns protocol lifecycle dispatch for MCP service calls while
//! delegating actual runtime operations to `McpRuntimeFacade`.  It exposes
//! structured status and snapshots now, and keeps direct toolkit attachment
//! paths available as deprecated compatibility anchors until Web adapters are
//! migrated slice by slice.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    CapabilityToolDescriptor, CapabilityToolOriginKind, CapabilityToolResourceScope, CleanupPolicy,
    KernelServiceId, McpCleanupCommand, McpProbeCommand, McpRegisterCommand, McpRegisterResult,
    McpRuntimeStatusView, McpServiceLifecycleScope, McpServiceSnapshot, McpServiceSnapshotCommand,
    McpStatusCommand, McpStatusResult, McpToolAttachCommand, McpToolAttachResult,
    McpToolCatalogCommand, McpToolCatalogResult, McpToolInvokeCommand, ServiceCallResult,
    ServiceCapability, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ServiceScope, ServiceType, TraceContext, TraceSchemaRef, MCP_CLEANUP_COMMAND,
    MCP_PROBE_COMMAND, MCP_REGISTER_COMMAND, MCP_SERVICE_ID, MCP_SNAPSHOT_COMMAND,
    MCP_STATUS_COMMAND, MCP_TOOL_ATTACH_COMMAND, MCP_TOOL_CATALOG_COMMAND, MCP_TOOL_INVOKE_COMMAND,
};

use crate::mcp_runtime::{McpRuntimeFacade, McpRuntimeStatus, McpToolPolicy};

/// Host-owned MCP service provider backed by an optional facade.
pub struct McpSystemServiceProvider {
    descriptor: ServiceDescriptor,
    facade: Option<Arc<McpRuntimeFacade>>,
}

impl McpSystemServiceProvider {
    /// Create a service provider backed by an existing MCP runtime facade.
    pub fn new(facade: Arc<McpRuntimeFacade>) -> Self {
        Self {
            descriptor: mcp_service_descriptor(),
            facade: Some(facade),
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: mcp_service_descriptor(),
            facade: None,
        }
    }

    fn facade(&self) -> ServiceResult<Arc<McpRuntimeFacade>> {
        self.facade
            .clone()
            .ok_or_else(|| ServiceError::ServiceUnavailable("MCP runtime is not configured".into()))
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn service_result(output: serde_json::Value, trace: TraceContext) -> ServiceCallResult {
        ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        }
    }
}

#[async_trait]
impl SystemService for McpSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.descriptor.id,
            configured = self.facade.is_some(),
            "mcp service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "mcp service command accepted"
        );
        match command.name.as_str() {
            MCP_REGISTER_COMMAND => {
                let typed: McpRegisterCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let count = typed.definitions.len();
                for definition in typed.definitions {
                    let definition = serde_json::from_value(definition)
                        .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                    facade.upsert_definition(definition).await;
                }
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    registered = count,
                    "mcp service definitions registered"
                );
                Ok(Self::service_result(
                    to_value(McpRegisterResult {
                        registered: count,
                        captured_at: chrono::Utc::now(),
                    })?,
                    typed.trace,
                ))
            }
            MCP_PROBE_COMMAND => {
                let typed: McpProbeCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let policy = runtime_policy(typed.policy.tool_policy);
                let statuses = facade.probe(&policy).await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = statuses.len(),
                    "mcp service probe completed"
                );
                Ok(Self::service_result(
                    to_value(status_result(statuses))?,
                    typed.trace,
                ))
            }
            MCP_TOOL_CATALOG_COMMAND => {
                let typed: McpToolCatalogCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let policy = runtime_policy(typed.policy.tool_policy);
                let statuses = facade.probe(&policy).await;
                let descriptors = statuses_to_descriptors(statuses)?;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = descriptors.len(),
                    "mcp service tool catalog emitted"
                );
                Ok(Self::service_result(
                    to_value(McpToolCatalogResult {
                        tools: descriptors,
                        captured_at: chrono::Utc::now(),
                    })?,
                    typed.trace,
                ))
            }
            MCP_TOOL_ATTACH_COMMAND => {
                let typed: McpToolAttachCommand = decode(command.payload)?;
                tracing::warn!(
                    trace_id = %typed.trace.trace_id,
                    "mcp service toolkit attach requested through metadata-only path"
                );
                let statuses = typed
                    .definitions
                    .into_iter()
                    .map(|definition| {
                        let definition: Result<crate::McpServerDefinition, _> =
                            serde_json::from_value(definition);
                        match definition {
                            Ok(definition) => McpRuntimeStatusView {
                                server_id: definition.id,
                                transport: "deferred_attach".into(),
                                lifecycle: service_lifecycle(definition.lifecycle),
                                session_mode: format!("{:?}", definition.session_mode),
                                state: "Deferred".into(),
                                exposed_tools: Vec::new(),
                                failure_reason: Some(
                                    "toolkit attach requires host-local Toolkit and remains a Web adapter operation in this slice"
                                        .into(),
                                ),
                            },
                            Err(err) => McpRuntimeStatusView {
                                server_id: "invalid_definition".into(),
                                transport: "invalid".into(),
                                lifecycle: McpServiceLifecycleScope::Session,
                                session_mode: "unknown".into(),
                                state: "Failed".into(),
                                exposed_tools: Vec::new(),
                                failure_reason: Some(err.to_string()),
                            },
                        }
                    })
                    .collect();
                Ok(Self::service_result(
                    to_value(McpToolAttachResult {
                        statuses,
                        conflicts: Vec::new(),
                        applied_prefixes: Vec::new(),
                        captured_at: chrono::Utc::now(),
                    })?,
                    typed.trace,
                ))
            }
            MCP_TOOL_INVOKE_COMMAND => {
                let typed: McpToolInvokeCommand = decode(command.payload)?;
                Err(ServiceError::UnsupportedCommand(format!(
                    "MCP tool '{}' cannot be invoked without a host-local toolkit session",
                    typed.invocation.tool_name
                )))
            }
            MCP_STATUS_COMMAND => {
                let typed: McpStatusCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let policy = runtime_policy(typed.policy.tool_policy);
                let statuses = facade.probe(&policy).await;
                Ok(Self::service_result(
                    to_value(status_result(statuses))?,
                    typed.trace,
                ))
            }
            MCP_SNAPSHOT_COMMAND => {
                let typed: McpServiceSnapshotCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let definitions = facade.definitions().await;
                let statuses = facade.probe(&McpToolPolicy::default()).await;
                let snapshot = snapshot_from_statuses(definitions.len(), statuses);
                tracing::info!(trace_id = %typed.trace.trace_id, "mcp service snapshot emitted");
                Ok(Self::service_result(to_value(snapshot)?, typed.trace))
            }
            MCP_CLEANUP_COMMAND => {
                let typed: McpCleanupCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let statuses = if let Some(session_id) = typed.scope.session_id.as_deref() {
                    facade.cleanup_session(session_id).await
                } else if let Some(app_id) = typed.scope.application_id.as_ref() {
                    facade.cleanup_app(app_id).await
                } else {
                    facade.cleanup_all().await
                };
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = statuses.len(),
                    "mcp service cleanup completed"
                );
                Ok(Self::service_result(
                    to_value(status_result(statuses))?,
                    typed.trace,
                ))
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported MCP service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "mcp service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "mcp service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.facade.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "MCP runtime is not configured".into(),
            })
        }
    }
}

/// Build the provider-neutral descriptor for the MCP system service.
pub fn mcp_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(MCP_SERVICE_ID),
        ServiceType::new("mcp"),
        TraceSchemaRef::new("trace.system_service.mcp.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.register"),
            "Registers provider-neutral MCP server definitions.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.probe"),
            "Probes MCP dependency and lifecycle status without leaking secrets.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.tool.catalog"),
            "Reports sanitized MCP tool metadata.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.cleanup"),
            "Cleans MCP resources through explicit lifecycle scope.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![
        ServiceScope::Global,
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.required_permissions = vec![
        "mcp.register".into(),
        "mcp.probe".into(),
        "mcp.tool.catalog".into(),
        "mcp.cleanup".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::Always;
    descriptor
}

fn statuses_to_descriptors(
    statuses: Vec<McpRuntimeStatus>,
) -> ServiceResult<Vec<CapabilityToolDescriptor>> {
    let mut descriptors = Vec::new();
    for status in statuses {
        for tool in status.exposed_tools {
            let descriptor = CapabilityToolDescriptor::new(
                MCP_SERVICE_ID,
                status.server_id.clone(),
                format!("mcp.tool.{}.{}", status.server_id, tool),
                tool.clone(),
                format!("MCP tool '{tool}' exposed by server '{}'", status.server_id),
                serde_json::json!({"type": "object"}),
                CapabilityToolOriginKind::Mcp,
            )
            .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?
            .with_policy_hints(
                vec!["mcp.tool.invoke".into()],
                vec![CapabilityToolResourceScope::AgentSession],
            );
            descriptors.push(descriptor);
        }
    }
    Ok(descriptors)
}

fn status_view(status: McpRuntimeStatus) -> McpRuntimeStatusView {
    McpRuntimeStatusView {
        server_id: status.server_id,
        transport: status.transport,
        lifecycle: service_lifecycle(status.lifecycle),
        session_mode: format!("{:?}", status.session_mode),
        state: format!("{:?}", status.state),
        exposed_tools: status.exposed_tools,
        failure_reason: status.failure_reason,
    }
}

fn status_result(statuses: Vec<McpRuntimeStatus>) -> McpStatusResult {
    McpStatusResult::new(statuses.into_iter().map(status_view).collect())
}

fn snapshot_from_statuses(
    registered_definitions: usize,
    statuses: Vec<McpRuntimeStatus>,
) -> McpServiceSnapshot {
    let mut ready = 0usize;
    let mut failed = 0usize;
    let mut dependency_missing = 0usize;
    let mut disabled = 0usize;
    let mut exposed_tool_count = 0usize;
    let mut lifecycle_scopes = Vec::new();
    let mut failure_reasons = Vec::new();
    for status in statuses {
        match status.state {
            crate::mcp_runtime::McpRuntimeStatusState::Ready => ready += 1,
            crate::mcp_runtime::McpRuntimeStatusState::Failed => failed += 1,
            crate::mcp_runtime::McpRuntimeStatusState::DependencyMissing => dependency_missing += 1,
            crate::mcp_runtime::McpRuntimeStatusState::Disabled => disabled += 1,
        }
        exposed_tool_count += status.exposed_tools.len();
        lifecycle_scopes.push(service_lifecycle(status.lifecycle));
        if let Some(reason) = status.failure_reason {
            failure_reasons.push(reason);
        }
    }
    lifecycle_scopes.sort();
    lifecycle_scopes.dedup();
    McpServiceSnapshot {
        service_id: MCP_SERVICE_ID.into(),
        healthy: failed == 0,
        registered_definitions,
        ready,
        failed,
        dependency_missing,
        disabled,
        exposed_tool_count,
        lifecycle_scopes,
        failure_reasons,
        captured_at: chrono::Utc::now(),
    }
}

fn service_lifecycle(lifecycle: crate::mcp_runtime::McpLifecycleScope) -> McpServiceLifecycleScope {
    match lifecycle {
        crate::mcp_runtime::McpLifecycleScope::Global => McpServiceLifecycleScope::Global,
        crate::mcp_runtime::McpLifecycleScope::App => McpServiceLifecycleScope::App,
        crate::mcp_runtime::McpLifecycleScope::Session => McpServiceLifecycleScope::Session,
        crate::mcp_runtime::McpLifecycleScope::AgentSession => {
            McpServiceLifecycleScope::AgentSession
        }
        crate::mcp_runtime::McpLifecycleScope::Call => McpServiceLifecycleScope::Call,
    }
}

fn runtime_policy(snapshot: macaca_proto::McpToolPolicySnapshot) -> McpToolPolicy {
    McpToolPolicy {
        allow_servers: snapshot
            .allow_servers
            .map(|items| items.into_iter().collect()),
        deny_servers: snapshot.deny_servers.into_iter().collect(),
        allow_tools: snapshot
            .allow_tools
            .map(|items| items.into_iter().collect()),
        deny_tools: snapshot.deny_tools.into_iter().collect(),
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))
}

fn to_value<T: serde::Serialize>(value: T) -> ServiceResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|err| ServiceError::AdapterFailure(err.to_string()))
}
