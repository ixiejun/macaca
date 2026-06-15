//! MCP operator lifecycle commands: reload, oauth, resources, diagnostics.
//!
//! **Pattern:** Adapter — delegates to `McpOperatorLifecycle` while preserving
//! provider-neutral DTO shaping and audit-friendly trace logging.

use macaca_proto::{
    McpDiagnosticsSnapshotCommand, McpExposureRefreshCommand, McpExposureRefreshResult,
    McpOAuthLoginCommand, McpOAuthStatusCommand, McpReloadCommand, McpResourceListCommand,
    McpResourceReadCommand, McpServerStatusListCommand, McpServerStatusListResult,
    ServiceCallResult, ServiceCommand, ServiceError, ServiceResult,
};

use super::support::{decode, status_result, to_value};
use super::McpSystemServiceProvider;
use crate::mcp_operator_lifecycle::runtime_policy_from_hints;
use crate::mcp_runtime::McpToolPolicy;

impl McpSystemServiceProvider {
    pub(super) async fn handle_server_status_list(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpServerStatusListCommand = super::support::decode(command.payload)?;
        let facade = self.facade()?;
        let policy = crate::mcp_operator_lifecycle::runtime_policy_from_hints(typed.policy);
        let statuses = super::support::status_result(facade.probe(&policy).await).statuses;
        let operator_statuses = self.operator()?.status_list(statuses).await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            count = operator_statuses.len(),
            "mcp operator server statuses emitted"
        );
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(McpServerStatusListResult {
                statuses: operator_statuses,
                captured_at: chrono::Utc::now(),
            })?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_reload(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpReloadCommand = super::support::decode(command.payload)?;
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
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(result)?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_exposure_refresh(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpExposureRefreshCommand = super::support::decode(command.payload)?;
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
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(McpExposureRefreshResult {
                thread_id: typed.thread_id,
                refreshed,
                exposure_generation: generation,
                visible_tool_count: tool_count,
                captured_at: chrono::Utc::now(),
            })?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_oauth_login(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpOAuthLoginCommand = super::support::decode(command.payload)?;
        let result = self.operator()?.start_oauth(&typed.server_id).await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            server_id = %typed.server_id,
            "mcp operator oauth login started"
        );
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(result)?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_oauth_status(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpOAuthStatusCommand = super::support::decode(command.payload)?;
        let result = self.operator()?.oauth_status(&typed.server_id).await;
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(result)?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_resource_list(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpResourceListCommand = super::support::decode(command.payload)?;
        let result = self
            .operator()?
            .resource_list(
                typed.server_id.as_deref(),
                &crate::mcp_operator_lifecycle::runtime_policy_from_hints(typed.policy),
            )
            .await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            count = result.resources.len(),
            "mcp operator resources listed"
        );
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(result)?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_resource_template_list(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpResourceListCommand = super::support::decode(command.payload)?;
        let result = self
            .operator()?
            .resource_template_list(
                typed.server_id.as_deref(),
                &crate::mcp_operator_lifecycle::runtime_policy_from_hints(typed.policy),
            )
            .await;
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(result)?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_resource_read(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpResourceReadCommand = super::support::decode(command.payload)?;
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
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(result)?,
            typed.trace,
        ))
    }

    pub(super) async fn handle_diagnostics_snapshot(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpDiagnosticsSnapshotCommand = super::support::decode(command.payload)?;
        let facade = self.facade()?;
        let statuses =
            super::support::status_result(facade.probe(&McpToolPolicy::default()).await).statuses;
        let result = self.operator()?.diagnostics(statuses).await;
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(result)?,
            typed.trace,
        ))
    }
}
