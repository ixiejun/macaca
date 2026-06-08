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
    CapabilityToolInvocationResult, CapabilityToolOriginKind, CapabilityToolPolicyHints,
    CleanupPolicy, KernelServiceId, McpCleanupCommand, McpDiagnosticsSnapshotCommand,
    McpExposureRefreshCommand, McpExposureRefreshResult, McpOAuthLoginCommand,
    McpOAuthStatusCommand, McpProbeCommand, McpRegisterCommand, McpRegisterResult,
    McpReloadCommand, McpResourceListCommand, McpResourceReadCommand, McpRuntimeStatusView,
    McpServerStatusListCommand, McpServerStatusListResult, McpServiceLifecycleScope,
    McpServiceSnapshot, McpServiceSnapshotCommand, McpStatusCommand, McpStatusResult,
    McpToolAttachCommand, McpToolAttachResult, McpToolCatalogCommand, McpToolCatalogResult,
    McpToolInvokeCommand, ServiceCallResult, ServiceCapability, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceScope, ServiceType, TraceContext,
    TraceSchemaRef, MCP_CLEANUP_COMMAND, MCP_DIAGNOSTICS_SNAPSHOT_COMMAND,
    MCP_EXPOSURE_REFRESH_COMMAND, MCP_OAUTH_LOGIN_COMMAND, MCP_OAUTH_STATUS_COMMAND,
    MCP_PROBE_COMMAND, MCP_REGISTER_COMMAND, MCP_RELOAD_COMMAND, MCP_RESOURCE_LIST_COMMAND,
    MCP_RESOURCE_READ_COMMAND, MCP_RESOURCE_TEMPLATE_LIST_COMMAND, MCP_SERVER_STATUS_LIST_COMMAND,
    MCP_SERVICE_ID, MCP_SNAPSHOT_COMMAND, MCP_STATUS_COMMAND, MCP_TOOL_ATTACH_COMMAND,
    MCP_TOOL_CATALOG_COMMAND, MCP_TOOL_INVOKE_COMMAND,
};

use crate::mcp_operator_lifecycle::{
    runtime_policy_from_hints, McpOperatorLifecycle, McpOperatorState,
};
use crate::mcp_runtime::{McpRuntimeContext, McpRuntimeFacade, McpRuntimeStatus, McpToolPolicy};

/// Host-owned MCP service provider backed by an optional facade.
pub struct McpSystemServiceProvider {
    descriptor: ServiceDescriptor,
    facade: Option<Arc<McpRuntimeFacade>>,
    operator_state: Arc<McpOperatorState>,
}

impl McpSystemServiceProvider {
    /// Create a service provider backed by an existing MCP runtime facade.
    pub fn new(facade: Arc<McpRuntimeFacade>) -> Self {
        Self {
            descriptor: mcp_service_descriptor(),
            facade: Some(facade),
            operator_state: Arc::new(McpOperatorState::default()),
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: mcp_service_descriptor(),
            facade: None,
            operator_state: Arc::new(McpOperatorState::default()),
        }
    }

    fn facade(&self) -> ServiceResult<Arc<McpRuntimeFacade>> {
        self.facade
            .clone()
            .ok_or_else(|| ServiceError::ServiceUnavailable("MCP runtime is not configured".into()))
    }

    fn operator(&self) -> ServiceResult<McpOperatorLifecycle> {
        Ok(McpOperatorLifecycle::new(
            self.facade()?,
            self.operator_state.clone(),
        ))
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
                let mut descriptors = Vec::new();
                for descriptor in facade.tool_descriptors(&policy).await {
                    match descriptor {
                        Ok(descriptor) => descriptors.push(descriptor),
                        Err(error) => tracing::warn!(
                            trace_id = %typed.trace.trace_id,
                            error = %error,
                            "mcp service descriptor index skipped a server"
                        ),
                    }
                }
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
                if let Some(reason) = validate_invocation_command(&typed) {
                    tracing::warn!(
                        trace_id = %typed.invocation.trace.trace_id,
                        visible_tool = %typed.invocation.tool_name,
                        reason,
                        "mcp service tool invocation rejected before side effects"
                    );
                    let result = failed_invocation_result(
                        &typed.invocation.tool_name,
                        reason,
                        typed.invocation.trace,
                    );
                    return Ok(Self::service_result(to_value(result)?, trace));
                }
                if self.facade.is_none() {
                    tracing::warn!(
                        trace_id = %typed.invocation.trace.trace_id,
                        visible_tool = %typed.invocation.tool_name,
                        "mcp service tool invocation rejected because provider is unavailable"
                    );
                    let result = failed_invocation_result(
                        &typed.invocation.tool_name,
                        "mcp_provider_unavailable",
                        typed.invocation.trace,
                    );
                    return Ok(Self::service_result(to_value(result)?, trace));
                }
                if typed
                    .invocation
                    .policy
                    .metadata
                    .get("mcp.oauth_required")
                    .is_some_and(|value| value == "true")
                {
                    tracing::warn!(
                        trace_id = %typed.invocation.trace.trace_id,
                        visible_tool = %typed.invocation.tool_name,
                        "mcp service tool invocation rejected because oauth is required"
                    );
                    let result = failed_invocation_result(
                        &typed.invocation.tool_name,
                        "mcp_oauth_required",
                        typed.invocation.trace,
                    );
                    return Ok(Self::service_result(to_value(result)?, trace));
                }
                let facade = self.facade()?;
                let server_id = typed.server_id.as_deref().ok_or_else(|| {
                    ServiceError::UnsupportedCommand(
                        "mcp.tool.invoke requires descriptor server_id metadata".into(),
                    )
                })?;
                let backend_tool_name = typed.backend_tool_name.as_deref().ok_or_else(|| {
                    ServiceError::UnsupportedCommand(
                        "mcp.tool.invoke requires descriptor backend_tool_name metadata".into(),
                    )
                })?;
                let context = McpRuntimeContext {
                    app_id: Some(typed.invocation.scope.application_id),
                    session_id: Some(typed.invocation.scope.session_id.clone()),
                    agent_name: Some(typed.invocation.scope.agent_name.clone()),
                };
                tracing::info!(
                    trace_id = %typed.invocation.trace.trace_id,
                    server_id,
                    backend_tool = backend_tool_name,
                    visible_tool = %typed.invocation.tool_name,
                    lifecycle = ?typed.lifecycle,
                    "mcp service tool invocation accepted"
                );
                let result = facade
                    .invoke_tool(
                        server_id,
                        backend_tool_name,
                        &typed.invocation.tool_name,
                        typed.invocation.input,
                        typed.invocation.trace,
                        &context,
                        &runtime_policy_from_capability_hints(&typed.invocation.policy),
                    )
                    .await;
                Ok(Self::service_result(to_value(result)?, trace))
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
            MCP_SERVER_STATUS_LIST_COMMAND => {
                let typed: McpServerStatusListCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let policy = runtime_policy_from_hints(typed.policy);
                let statuses = status_result(facade.probe(&policy).await).statuses;
                let operator_statuses = self.operator()?.status_list(statuses).await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = operator_statuses.len(),
                    "mcp operator server statuses emitted"
                );
                Ok(Self::service_result(
                    to_value(McpServerStatusListResult {
                        statuses: operator_statuses,
                        captured_at: chrono::Utc::now(),
                    })?,
                    typed.trace,
                ))
            }
            MCP_RELOAD_COMMAND => {
                let typed: McpReloadCommand = decode(command.payload)?;
                let result = self
                    .operator()?
                    .reload(&typed.scope, typed.definitions)
                    .await
                    .map_err(ServiceError::AdapterFailure)?;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    reloaded = result.reloaded,
                    exposure_generation = result.exposure_generation,
                    "mcp operator reload completed"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            MCP_EXPOSURE_REFRESH_COMMAND => {
                let typed: McpExposureRefreshCommand = decode(command.payload)?;
                let (refreshed, generation, tool_count) = self
                    .operator()?
                    .refresh_exposure(&typed.thread_id, &McpToolPolicy::default())
                    .await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    thread_id = %typed.thread_id,
                    refreshed,
                    exposure_generation = generation,
                    "mcp operator exposure refresh evaluated"
                );
                Ok(Self::service_result(
                    to_value(McpExposureRefreshResult {
                        thread_id: typed.thread_id,
                        refreshed,
                        exposure_generation: generation,
                        visible_tool_count: tool_count,
                        captured_at: chrono::Utc::now(),
                    })?,
                    typed.trace,
                ))
            }
            MCP_OAUTH_LOGIN_COMMAND => {
                let typed: McpOAuthLoginCommand = decode(command.payload)?;
                let result = self.operator()?.start_oauth(&typed.server_id).await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    server_id = %typed.server_id,
                    "mcp operator oauth login started"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            MCP_OAUTH_STATUS_COMMAND => {
                let typed: McpOAuthStatusCommand = decode(command.payload)?;
                let result = self.operator()?.oauth_status(&typed.server_id).await;
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            MCP_RESOURCE_LIST_COMMAND => {
                let typed: McpResourceListCommand = decode(command.payload)?;
                let result = self
                    .operator()?
                    .resource_list(
                        typed.server_id.as_deref(),
                        &runtime_policy_from_hints(typed.policy),
                    )
                    .await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = result.resources.len(),
                    "mcp operator resources listed"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            MCP_RESOURCE_TEMPLATE_LIST_COMMAND => {
                let typed: McpResourceListCommand = decode(command.payload)?;
                let result = self
                    .operator()?
                    .resource_template_list(
                        typed.server_id.as_deref(),
                        &runtime_policy_from_hints(typed.policy),
                    )
                    .await;
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            MCP_RESOURCE_READ_COMMAND => {
                let typed: McpResourceReadCommand = decode(command.payload)?;
                let result = self
                    .operator()?
                    .read_resource(
                        &typed.server_id,
                        &typed.uri,
                        typed.max_bytes,
                        &McpToolPolicy::default(),
                    )
                    .await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    server_id = %typed.server_id,
                    uri_hash = %crate::tool_service_provider_state::stable_json_hash(&serde_json::json!(typed.uri)),
                    truncated = result.truncated,
                    "mcp operator resource read completed"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            MCP_DIAGNOSTICS_SNAPSHOT_COMMAND => {
                let typed: McpDiagnosticsSnapshotCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let statuses =
                    status_result(facade.probe(&McpToolPolicy::default()).await).statuses;
                let result = self.operator()?.diagnostics(statuses).await;
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            MCP_SNAPSHOT_COMMAND => {
                let typed: McpServiceSnapshotCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let definitions = facade.snapshot_server_definitions().await;
                let statuses = facade.probe(&McpToolPolicy::default()).await;
                let snapshot = snapshot_from_statuses(
                    definitions.len(),
                    service_definition_payloads(typed.include_definitions, definitions),
                    statuses,
                );
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    include_definitions = typed.include_definitions,
                    registered_definitions = snapshot.registered_definitions,
                    emitted_definitions = snapshot.definitions.len(),
                    "mcp service snapshot emitted"
                );
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
            macaca_proto::CapabilityId::new("capability.mcp.tool.invoke"),
            "Invokes MCP tools through descriptor-routed service dispatch.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.cleanup"),
            "Cleans MCP resources through explicit lifecycle scope.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.operator_lifecycle"),
            "Manages MCP reload, OAuth, resources, diagnostics, and exposure refresh.",
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
        "mcp.tool.invoke".into(),
        "mcp.server.reload".into(),
        "mcp.oauth.login".into(),
        "mcp.resource.read".into(),
        "mcp.diagnostics.snapshot".into(),
        "mcp.cleanup".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::Always;
    descriptor
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

fn validate_invocation_command(command: &McpToolInvokeCommand) -> Option<&'static str> {
    if command.invocation.trace.trace_id.trim().is_empty() {
        return Some("missing_trace");
    }
    if command.invocation.scope.session_id.trim().is_empty()
        || command.invocation.scope.agent_name.trim().is_empty()
    {
        return Some("scope_missing");
    }
    if command.invocation.tool_name.trim().is_empty() {
        return Some("tool_missing");
    }
    if command
        .server_id
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Some("server_missing");
    }
    if command
        .backend_tool_name
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Some("backend_tool_missing");
    }
    if command.lifecycle.is_none() {
        return Some("lifecycle_missing");
    }
    None
}

fn failed_invocation_result(
    visible_tool_name: &str,
    reason: impl Into<String>,
    trace: TraceContext,
) -> CapabilityToolInvocationResult {
    let reason = sanitize_reason(reason);
    let mut result = CapabilityToolInvocationResult::failed(
        MCP_SERVICE_ID,
        CapabilityToolOriginKind::Mcp,
        visible_tool_name,
        reason.clone(),
        trace,
    );
    result
        .metadata
        .insert("mcp.policy_decision".into(), "deny".into());
    result.metadata.insert("mcp.reason_code".into(), reason);
    result
}

fn runtime_policy_from_capability_hints(hints: &CapabilityToolPolicyHints) -> McpToolPolicy {
    let mut policy = McpToolPolicy::default();
    if let Some(deny_tools) = hints.metadata.get("mcp.deny_tools") {
        policy.deny_tools = deny_tools
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    if let Some(deny_servers) = hints.metadata.get("mcp.deny_servers") {
        policy.deny_servers = deny_servers
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    policy
}

fn sanitize_reason(reason: impl Into<String>) -> String {
    let mut value = reason.into().replace('\n', " ").replace('\r', " ");
    value.truncate(240);
    value
}

fn snapshot_from_statuses(
    registered_definitions: usize,
    definitions: Vec<serde_json::Value>,
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
        definitions,
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

/// Convert runtime-owned server definitions into snapshot DTO payloads.
///
/// The provider deliberately serializes definitions at the service edge instead
/// of exposing `McpRuntimeFacade` to Web.  Serialization failures are logged and
/// skipped so one malformed extension cannot block the entire diagnostic
/// snapshot; the registered count still reports the full runtime inventory for
/// operator visibility.
fn service_definition_payloads(
    include_definitions: bool,
    definitions: Vec<crate::McpServerDefinition>,
) -> Vec<serde_json::Value> {
    if !include_definitions {
        return Vec::new();
    }

    definitions
        .into_iter()
        .filter_map(|definition| match serde_json::to_value(&definition) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::warn!(
                    server_id = %definition.id,
                    error = %error,
                    "mcp service snapshot skipped unserializable definition"
                );
                None
            }
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_framework::mcp::McpTransportConfig;
    use macaca_kernel::SystemService;
    use macaca_proto::{
        ApplicationId, CapabilityToolInvocation, CapabilityToolInvocationScope,
        McpExposureRefreshCommand, McpOAuthLoginCommand, McpOAuthState, McpReloadCommand,
        ServiceCommandName,
    };

    fn scoped_invocation(trace_id: &str, session_id: &str) -> CapabilityToolInvocation {
        CapabilityToolInvocation {
            trace: TraceContext::new(trace_id),
            scope: CapabilityToolInvocationScope {
                application_id: ApplicationId::new(),
                session_id: session_id.into(),
                agent_name: "agent-a".into(),
            },
            tool_name: "mcp_lookup".into(),
            input: serde_json::json!({"query": "value"}),
            policy: CapabilityToolPolicyHints::default(),
        }
    }

    fn operator_definition(server_id: &str) -> crate::McpServerDefinition {
        crate::McpServerDefinition {
            id: server_id.into(),
            transport: McpTransportConfig::Stdio {
                command: "missing-test-mcp-binary".into(),
                args: Vec::new(),
                env: std::collections::BTreeMap::new(),
                cwd: None,
            },
            lifecycle: crate::mcp_runtime::McpLifecycleScope::AgentSession,
            session_mode: macaca_framework::mcp::McpSessionMode::Stateful,
            tool_prefix: Some("fixture".into()),
            required_bins: Vec::new(),
            enabled: true,
            source: crate::McpDefinitionSource::Global,
            concurrency_isolation: None,
        }
    }

    #[tokio::test]
    async fn invoke_rejects_missing_trace_before_facade_side_effects() {
        let provider = McpSystemServiceProvider::unavailable();
        let command = McpToolInvokeCommand {
            invocation: scoped_invocation("", "session-a"),
            server_id: Some("server-a".into()),
            backend_tool_name: Some("lookup".into()),
            lifecycle: Some(McpServiceLifecycleScope::AgentSession),
        };
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(MCP_TOOL_INVOKE_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("outer-trace"),
            ))
            .await
            .unwrap();
        let invocation: CapabilityToolInvocationResult =
            serde_json::from_value(result.output).unwrap();

        assert_eq!(invocation.status, "failed");
        assert_eq!(invocation.error_summary.as_deref(), Some("missing_trace"));
        assert_eq!(
            invocation.metadata.get("mcp.policy_decision"),
            Some(&"deny".to_string())
        );
    }

    #[tokio::test]
    async fn operator_reload_marks_thread_for_next_turn_exposure_refresh() {
        let provider = McpSystemServiceProvider::new(Arc::new(McpRuntimeFacade::new()));
        let app_id = ApplicationId::new();
        let scope =
            macaca_proto::McpServiceScope::agent_session(app_id, "thread-a", "agent-a").unwrap();
        let reload = McpReloadCommand {
            trace: TraceContext::new("trace-mcp-reload"),
            scope: scope.clone(),
            definitions: vec![serde_json::to_value(operator_definition("server-a")).unwrap()],
            reason: Some("test reload".into()),
        };
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(MCP_RELOAD_COMMAND),
                serde_json::to_value(reload).unwrap(),
                TraceContext::new("trace-mcp-reload"),
            ))
            .await
            .unwrap();
        let reload: macaca_proto::McpReloadResult = serde_json::from_value(result.output).unwrap();
        assert_eq!(reload.reloaded, 1);
        assert_eq!(reload.pending_thread_ids, vec!["thread-a".to_string()]);

        let refresh = McpExposureRefreshCommand {
            trace: TraceContext::new("trace-mcp-refresh"),
            scope,
            thread_id: "thread-a".into(),
            active_turn_id: Some("turn-a".into()),
        };
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(MCP_EXPOSURE_REFRESH_COMMAND),
                serde_json::to_value(refresh).unwrap(),
                TraceContext::new("trace-mcp-refresh"),
            ))
            .await
            .unwrap();
        let refresh: McpExposureRefreshResult = serde_json::from_value(result.output).unwrap();
        assert!(refresh.refreshed);
        assert_eq!(refresh.exposure_generation, 1);
    }

    #[tokio::test]
    async fn operator_oauth_login_and_status_are_structured() {
        let provider = McpSystemServiceProvider::new(Arc::new(McpRuntimeFacade::new()));
        let command = McpOAuthLoginCommand {
            trace: TraceContext::new("trace-mcp-oauth"),
            scope: macaca_proto::McpServiceScope::default(),
            server_id: "server-a".into(),
        };
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(MCP_OAUTH_LOGIN_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("trace-mcp-oauth"),
            ))
            .await
            .unwrap();
        let status: macaca_proto::McpOAuthStatusResult =
            serde_json::from_value(result.output).unwrap();
        assert_eq!(status.state, McpOAuthState::LoginStarted);
        assert!(status.login_flow_ref.is_some());
    }

    #[tokio::test]
    async fn invoke_rejects_missing_scope_before_facade_side_effects() {
        let provider = McpSystemServiceProvider::unavailable();
        let command = McpToolInvokeCommand {
            invocation: scoped_invocation("trace-mcp", ""),
            server_id: Some("server-a".into()),
            backend_tool_name: Some("lookup".into()),
            lifecycle: Some(McpServiceLifecycleScope::AgentSession),
        };
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(MCP_TOOL_INVOKE_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("outer-trace"),
            ))
            .await
            .unwrap();
        let invocation: CapabilityToolInvocationResult =
            serde_json::from_value(result.output).unwrap();

        assert_eq!(invocation.status, "failed");
        assert_eq!(invocation.error_summary.as_deref(), Some("scope_missing"));
    }

    #[tokio::test]
    async fn invoke_returns_structured_failure_when_provider_is_unavailable() {
        let provider = McpSystemServiceProvider::unavailable();
        let command = McpToolInvokeCommand {
            invocation: scoped_invocation("trace-mcp", "session-a"),
            server_id: Some("server-a".into()),
            backend_tool_name: Some("lookup".into()),
            lifecycle: Some(McpServiceLifecycleScope::AgentSession),
        };

        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(MCP_TOOL_INVOKE_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("outer-trace"),
            ))
            .await
            .unwrap();
        let invocation: CapabilityToolInvocationResult =
            serde_json::from_value(result.output).unwrap();

        assert_eq!(invocation.status, "failed");
        assert_eq!(
            invocation.error_summary.as_deref(),
            Some("mcp_provider_unavailable")
        );
        assert_eq!(
            invocation.metadata.get("mcp.policy_decision"),
            Some(&"deny".into())
        );
    }

    #[tokio::test]
    async fn invoke_returns_structured_failure_when_oauth_is_required() {
        let provider = McpSystemServiceProvider::new(Arc::new(McpRuntimeFacade::new()));
        let mut invocation = scoped_invocation("trace-mcp-oauth-required", "session-a");
        invocation
            .policy
            .metadata
            .insert("mcp.oauth_required".into(), "true".into());
        let command = McpToolInvokeCommand {
            invocation,
            server_id: Some("server-a".into()),
            backend_tool_name: Some("lookup".into()),
            lifecycle: Some(McpServiceLifecycleScope::AgentSession),
        };

        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(MCP_TOOL_INVOKE_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("outer-trace"),
            ))
            .await
            .unwrap();
        let invocation: CapabilityToolInvocationResult =
            serde_json::from_value(result.output).unwrap();

        assert_eq!(invocation.status, "failed");
        assert_eq!(
            invocation.error_summary.as_deref(),
            Some("mcp_oauth_required")
        );
        assert_eq!(
            invocation.metadata.get("mcp.policy_decision"),
            Some(&"deny".into())
        );
    }

    #[test]
    fn snapshot_definition_payloads_are_explicitly_requested() {
        let definition = crate::McpServerDefinition {
            id: "server-a".into(),
            transport: McpTransportConfig::Stdio {
                command: "service-binary".into(),
                args: vec!["--stdio".into()],
                env: std::collections::BTreeMap::new(),
                cwd: None,
            },
            lifecycle: crate::mcp_runtime::McpLifecycleScope::AgentSession,
            session_mode: macaca_framework::mcp::McpSessionMode::Stateful,
            tool_prefix: Some("sample".into()),
            required_bins: Vec::new(),
            enabled: true,
            source: crate::McpDefinitionSource::Global,
            concurrency_isolation: None,
        };

        // The snapshot contract uses a Command-style flag because most callers
        // only need counts and health.  Toolkit assembly opts into the payloads
        // during migration, while status dashboards can keep the cheaper and
        // less detailed default view.
        assert!(service_definition_payloads(false, vec![definition.clone()]).is_empty());

        let payloads = service_definition_payloads(true, vec![definition]);
        assert_eq!(payloads.len(), 1);
        assert_eq!(
            payloads[0].get("id").and_then(|value| value.as_str()),
            Some("server-a")
        );
        assert_eq!(
            payloads[0]
                .get("transport")
                .and_then(|value| value.as_str()),
            Some("stdio")
        );
    }
}
