//! MCP tool invocation, registration, lease, and cleanup (Template Method).
//!
//! Service-backed invocation path: validate descriptor route → acquire lease →
//! protocol call → release lease with audit metadata.

use std::sync::Arc;
use std::time::{Duration, Instant};

use macaca_framework::mcp::{McpClient, McpError};
use macaca_framework::tool::Toolkit;
use macaca_proto::{ApplicationId, CapabilityToolInvocationResult, TraceContext,
    MCP_DESCRIPTOR_BACKEND_TOOL_NAME, MCP_SERVICE_ID, CapabilityToolOriginKind};
use tokio::time::timeout;

use crate::lease::McpSessionLease;

use super::helpers::{
    failed_invocation_result, failed_invocation_result_with_metadata, lifecycle_scope_name,
    missing_required_bin, prefixed_tool_name, sanitize_error, stable_json_hash,
    status_for_definition, validate_descriptor_route,
};
use super::manager::McpRuntimeManager;
use super::types::{
    McpLifecycleScope, McpRuntimeContext, McpRuntimeKey, McpRuntimeStatus,
    McpRuntimeStatusState, McpServerDefinition, McpToolPolicy,
};

#[allow(deprecated)]
impl McpRuntimeManager {
    pub(crate) async fn invoke_tool(
        &self,
        server_id: &str,
        backend_tool_name: &str,
        visible_tool_name: &str,
        input: serde_json::Value,
        trace: TraceContext,
        context: &McpRuntimeContext,
        policy: &McpToolPolicy,
    ) -> CapabilityToolInvocationResult {
        let Some(definition) = self.definitions.read().await.get(server_id).cloned() else {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id,
                tool = visible_tool_name,
                "mcp service invocation rejected because server is unknown"
            );
            return failed_invocation_result(visible_tool_name, "unknown_mcp_server", trace);
        };
        let Some(route) = self
            .descriptor_index
            .route_for_visible_tool(visible_tool_name)
            .await
        else {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id,
                tool = visible_tool_name,
                "mcp service invocation rejected because descriptor route is unknown"
            );
            return failed_invocation_result(
                visible_tool_name,
                "mcp_descriptor_route_unknown",
                trace,
            );
        };
        if route.lifecycle != definition.lifecycle {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id,
                visible_tool = visible_tool_name,
                route_lifecycle = ?route.lifecycle,
                definition_lifecycle = ?definition.lifecycle,
                "mcp service invocation rejected because descriptor lifecycle drifted"
            );
            return failed_invocation_result(
                visible_tool_name,
                "mcp_descriptor_lifecycle_mismatch",
                trace,
            );
        }
        if let Some(error) =
            validate_descriptor_route(&route, server_id, backend_tool_name, visible_tool_name)
        {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id,
                backend_tool = backend_tool_name,
                visible_tool = visible_tool_name,
                reason = %error,
                "mcp service invocation rejected by descriptor index"
            );
            return failed_invocation_result(visible_tool_name, error, trace);
        }
        if !definition.enabled || !policy.allows_server(&definition.id) {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id = %definition.id,
                tool = visible_tool_name,
                "mcp service invocation denied by server policy"
            );
            return failed_invocation_result(visible_tool_name, "mcp_server_denied", trace);
        }
        if !policy.allows_tool(backend_tool_name) || !policy.allows_tool(visible_tool_name) {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id = %definition.id,
                backend_tool = backend_tool_name,
                visible_tool = visible_tool_name,
                "mcp service invocation denied by tool policy"
            );
            return failed_invocation_result(visible_tool_name, "mcp_tool_denied", trace);
        }
        let expected_visible_name = prefixed_tool_name(&definition, backend_tool_name);
        if expected_visible_name != visible_tool_name {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id = %definition.id,
                backend_tool = backend_tool_name,
                visible_tool = visible_tool_name,
                expected_visible_tool = %expected_visible_name,
                "mcp service invocation rejected because descriptor routing did not match"
            );
            return failed_invocation_result(visible_tool_name, "mcp_descriptor_mismatch", trace);
        }
        if let Some(missing) = missing_required_bin(&definition) {
            tracing::warn!(
                trace_id = %trace.trace_id,
                server_id = %definition.id,
                missing_dependency = %missing,
                "mcp service invocation unavailable because a required binary is missing"
            );
            return failed_invocation_result(
                visible_tool_name,
                format!("missing dependency: {missing}"),
                trace,
            );
        }

        let mut client = match self.create_client(&definition) {
            Ok(client) => client,
            Err(error) => {
                tracing::warn!(
                    trace_id = %trace.trace_id,
                    server_id = %definition.id,
                    error = %error,
                    "mcp service invocation failed to create protocol client"
                );
                return failed_invocation_result(visible_tool_name, error.to_string(), trace);
            }
        };

        let started = Instant::now();
        let input_hash = stable_json_hash(&input);
        let lease = self.acquire_lease(&definition, context).await;
        tracing::info!(
            trace_id = %trace.trace_id,
            server_id = %definition.id,
            backend_tool = backend_tool_name,
            visible_tool = visible_tool_name,
            lifecycle = ?definition.lifecycle,
            input_hash = %input_hash,
            "mcp service invocation dispatch started"
        );

        let outcome = async {
            timeout(self.timeouts.connect, client.connect())
                .await
                .map_err(|_| "connect_timeout".to_string())?
                .map_err(|error| error.to_string())?;
            timeout(
                self.timeouts.call_tool,
                client.call_tool(backend_tool_name, input),
            )
            .await
            .map_err(|_| "call_tool_timeout".to_string())?
            .map_err(|error| error.to_string())
        }
        .await;
        let close_result = client.close().await;

        let cleanup_reason = if close_result.is_ok() {
            "closed"
        } else {
            "close_failed"
        };
        let force_release = matches!(definition.lifecycle, McpLifecycleScope::Call);
        let _ = self
            .release_lease_with_reason(lease, force_release, cleanup_reason)
            .await;
        tracing::info!(
            trace_id = %trace.trace_id,
            server_id = %definition.id,
            lifecycle = ?definition.lifecycle,
            cleanup_status = cleanup_reason,
            "mcp service invocation cleanup recorded"
        );

        match outcome {
            Ok(result) if !result.is_error => {
                let output = serde_json::json!({
                    "content": result.content,
                    "metadata": result.metadata,
                });
                let output_hash = stable_json_hash(&output);
                tracing::info!(
                    trace_id = %trace.trace_id,
                    server_id = %definition.id,
                    backend_tool = backend_tool_name,
                    visible_tool = visible_tool_name,
                    latency_ms = started.elapsed().as_millis() as u64,
                    output_hash = %output_hash,
                    "mcp service invocation completed"
                );
                let mut service_result = CapabilityToolInvocationResult::ok(
                    MCP_SERVICE_ID,
                    CapabilityToolOriginKind::Mcp,
                    visible_tool_name,
                    output,
                    trace,
                );
                service_result
                    .metadata
                    .insert("mcp.server_id".into(), definition.id);
                service_result.metadata.insert(
                    MCP_DESCRIPTOR_BACKEND_TOOL_NAME.into(),
                    backend_tool_name.into(),
                );
                service_result
                    .metadata
                    .insert("mcp.visible_tool_name".into(), visible_tool_name.into());
                service_result.metadata.insert(
                    "mcp.lifecycle_scope".into(),
                    lifecycle_scope_name(&definition.lifecycle).into(),
                );
                service_result
                    .metadata
                    .insert("mcp.policy_decision".into(), "allow".into());
                service_result
                    .metadata
                    .insert("mcp.reason_code".into(), "ok".into());
                service_result
                    .metadata
                    .insert("mcp.input_hash".into(), input_hash);
                service_result
                    .metadata
                    .insert("mcp.output_hash".into(), output_hash);
                service_result.metadata.insert(
                    "mcp.latency_ms".into(),
                    (started.elapsed().as_millis() as u64).to_string(),
                );
                service_result
            }
            Ok(result) => {
                let summary = serde_json::to_string(&result.content)
                    .unwrap_or_else(|_| "mcp_tool_error".to_string());
                let reason = sanitize_error(summary);
                tracing::warn!(
                    trace_id = %trace.trace_id,
                    server_id = %definition.id,
                    backend_tool = backend_tool_name,
                    visible_tool = visible_tool_name,
                    latency_ms = started.elapsed().as_millis() as u64,
                    reason_code = %reason,
                    "mcp service invocation returned an MCP error result"
                );
                failed_invocation_result_with_metadata(
                    visible_tool_name,
                    reason,
                    trace,
                    server_id,
                    backend_tool_name,
                    &definition.lifecycle,
                    input_hash,
                    started.elapsed(),
                )
            }
            Err(error) => {
                let reason = sanitize_error(error);
                tracing::warn!(
                    trace_id = %trace.trace_id,
                    server_id = %definition.id,
                    backend_tool = backend_tool_name,
                    visible_tool = visible_tool_name,
                    latency_ms = started.elapsed().as_millis() as u64,
                    reason_code = %reason,
                    "mcp service invocation failed"
                );
                failed_invocation_result_with_metadata(
                    visible_tool_name,
                    reason,
                    trace,
                    server_id,
                    backend_tool_name,
                    &definition.lifecycle,
                    input_hash,
                    started.elapsed(),
                )
            }
        }
    }

    #[deprecated(note = "Use `McpRuntimeFacade::register` instead.")]
    pub async fn register_tools(
        self: &Arc<Self>,
        toolkit: &mut Toolkit,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
    ) -> Vec<McpRuntimeStatus> {
        let definitions = self.snapshot_server_definitions().await;
        self.register_definitions(toolkit, definitions, policy, context, None)
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::register_definitions` instead.")]
    pub async fn register_definitions(
        self: &Arc<Self>,
        toolkit: &mut Toolkit,
        definitions: Vec<McpServerDefinition>,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
        on_closed: Option<Arc<dyn Fn(McpRuntimeStatus) + Send + Sync>>,
    ) -> Vec<McpRuntimeStatus> {
        let mut statuses = Vec::new();
        for definition in definitions {
            if !definition.enabled || !policy.allows_server(&definition.id) {
                statuses.push(status_for_definition(
                    &definition,
                    McpRuntimeStatusState::Disabled,
                    Vec::new(),
                    None,
                ));
                continue;
            }
            let status = self
                .register_definition_tools(toolkit, &definition, policy, context, on_closed.clone())
                .await;
            statuses.push(status);
        }
        statuses
    }

    #[deprecated(note = "Use `acquire_lease` for explicit runtime ownership.")]
    pub async fn acquire_runtime_key(
        &self,
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> McpRuntimeKey {
        self.invocation_registry
            .acquire(definition, context)
            .await
            .into_key()
    }

    #[deprecated(note = "Use `release_lease` for explicit runtime ownership release.")]
    pub async fn release_runtime_key(&self, key: &McpRuntimeKey) -> Option<McpRuntimeStatus> {
        let lease = McpSessionLease::new(key.clone());
        self.invocation_registry
            .release(&lease, false, "released")
            .await
    }

    pub async fn acquire_lease(
        &self,
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> McpSessionLease {
        self.invocation_registry.acquire(definition, context).await
    }

    /// Create a protocol client through the configured Strategy.
    ///
    /// This wrapper keeps all call sites honest: lifecycle/policy/descriptor
    /// validation happens before this method is reached, and this method is the
    /// only place where runtime-host crosses into the low-level MCP protocol
    /// implementation for service-backed invocation.
    pub(crate) fn create_client(
        &self,
        definition: &McpServerDefinition,
    ) -> Result<Box<dyn McpClient>, McpError> {
        (self.client_factory)(definition, self.timeouts)
    }

    pub async fn release_lease(&self, lease: McpSessionLease) -> Option<McpRuntimeStatus> {
        self.release_lease_with_reason(lease, false, "released")
            .await
    }

    async fn release_lease_with_reason(
        &self,
        lease: McpSessionLease,
        force_remove: bool,
        reason: &str,
    ) -> Option<McpRuntimeStatus> {
        self.invocation_registry
            .release(&lease, force_remove, reason)
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::cleanup_session` instead.")]
    pub async fn cleanup_session(&self, session_id: &str) -> Vec<McpRuntimeStatus> {
        self.invocation_registry
            .cleanup_matching(
                |key| key.session_id.as_deref() == Some(session_id),
                "session_cleanup",
            )
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::cleanup_app` instead.")]
    pub async fn cleanup_app(&self, app_id: &ApplicationId) -> Vec<McpRuntimeStatus> {
        let app = app_id.0.to_string();
        self.invocation_registry
            .cleanup_matching(
                |key| key.app_id.as_deref() == Some(app.as_str()),
                "app_cleanup",
            )
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::cleanup_all` instead.")]
    pub async fn cleanup_all(&self) -> Vec<McpRuntimeStatus> {
        self.invocation_registry
            .cleanup_matching(|_| true, "all_cleanup")
            .await
    }

    #[deprecated(note = "Use `McpRuntimeFacade::cleanup_idle` instead.")]
    pub async fn cleanup_idle(&self, ttl: Duration) -> Vec<McpRuntimeStatus> {
        self.invocation_registry.cleanup_idle(ttl).await
    }
}
