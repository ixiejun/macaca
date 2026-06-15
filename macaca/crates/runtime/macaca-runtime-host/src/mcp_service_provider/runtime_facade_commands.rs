//! Runtime-facade MCP commands: register, probe, catalog, lifecycle cleanup.
//!
//! **Pattern:** Adapter — translates typed service commands into `McpRuntimeFacade`
//! calls while emitting trace logs at admission and completion nodes.

use macaca_proto::{
    McpCleanupCommand, McpProbeCommand, McpRegisterCommand, McpRegisterResult,
    McpRuntimeStatusView, McpServiceLifecycleScope, McpServiceSnapshotCommand, McpStatusCommand,
    McpToolAttachCommand, McpToolAttachResult, McpToolCatalogCommand, McpToolCatalogResult,
    ServiceCallResult, ServiceCommand, ServiceError, ServiceResult,
};

use super::support::{
    decode, runtime_policy, service_definition_payloads, service_lifecycle, snapshot_from_statuses,
    status_result, to_value,
};
use super::McpSystemServiceProvider;
use crate::mcp_runtime::McpToolPolicy;

impl McpSystemServiceProvider {
    pub(super) async fn handle_register(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpRegisterCommand = super::support::decode(command.payload)?;
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
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(McpRegisterResult {
                registered: count,
                captured_at: chrono::Utc::now(),
            })?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_probe(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpProbeCommand = super::support::decode(command.payload)?;
        let facade = self.facade()?;
        let policy = super::support::runtime_policy(typed.policy.tool_policy);
        let statuses = facade.probe(&policy).await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            count = statuses.len(),
            "mcp service probe completed"
        );
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(super::support::status_result(statuses))?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_tool_catalog(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpToolCatalogCommand = super::support::decode(command.payload)?;
        let facade = self.facade()?;
        let policy = super::support::runtime_policy(typed.policy.tool_policy);
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
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(McpToolCatalogResult {
                tools: descriptors,
                captured_at: chrono::Utc::now(),
            })?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_tool_attach(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpToolAttachCommand = super::support::decode(command.payload)?;
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
                lifecycle: super::support::service_lifecycle(definition.lifecycle),
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
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(McpToolAttachResult {
                statuses,
                conflicts: Vec::new(),
                applied_prefixes: Vec::new(),
                captured_at: chrono::Utc::now(),
            })?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_status(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpStatusCommand = super::support::decode(command.payload)?;
        let facade = self.facade()?;
        let policy = super::support::runtime_policy(typed.policy.tool_policy);
        let statuses = facade.probe(&policy).await;
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(super::support::status_result(statuses))?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_snapshot(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpServiceSnapshotCommand = super::support::decode(command.payload)?;
        let facade = self.facade()?;
        let definitions = facade.snapshot_server_definitions().await;
        let statuses = facade.probe(&McpToolPolicy::default()).await;
        let snapshot = super::support::snapshot_from_statuses(
            definitions.len(),
            super::support::service_definition_payloads(typed.include_definitions, definitions),
            statuses,
        );
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            include_definitions = typed.include_definitions,
            registered_definitions = snapshot.registered_definitions,
            emitted_definitions = snapshot.definitions.len(),
            "mcp service snapshot emitted"
        );
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(snapshot)?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_cleanup(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpCleanupCommand = super::support::decode(command.payload)?;
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
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(super::support::status_result(statuses))?,
            typed.trace,
        ))
    }
}
