//! Command Router for the built-in MCP system service provider.
//!
//! **Pattern:** Command — each `MCP_*` command name maps to a focused handler
//! method.  The router stays free of business logic so new commands can be added
//! without growing a monolithic `call` body.

use macaca_proto::{
    ServiceCallResult, ServiceCommand, ServiceError, ServiceResult, TraceContext,
    MCP_CLEANUP_COMMAND, MCP_DIAGNOSTICS_SNAPSHOT_COMMAND, MCP_EXPOSURE_REFRESH_COMMAND,
    MCP_OAUTH_LOGIN_COMMAND, MCP_OAUTH_STATUS_COMMAND, MCP_PROBE_COMMAND, MCP_REGISTER_COMMAND,
    MCP_RELOAD_COMMAND, MCP_RESOURCE_LIST_COMMAND, MCP_RESOURCE_READ_COMMAND,
    MCP_RESOURCE_TEMPLATE_LIST_COMMAND, MCP_SERVER_STATUS_LIST_COMMAND, MCP_SNAPSHOT_COMMAND,
    MCP_STATUS_COMMAND, MCP_TOOL_ATTACH_COMMAND, MCP_TOOL_CATALOG_COMMAND, MCP_TOOL_INVOKE_COMMAND,
};

use super::McpSystemServiceProvider;

impl McpSystemServiceProvider {
    /// Route a decoded MCP service command to its handler after trace admission.
    pub(super) async fn dispatch_command(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        match command.name.as_str() {
            MCP_REGISTER_COMMAND => self.handle_register(command).await,
            MCP_PROBE_COMMAND => self.handle_probe(command).await,
            MCP_TOOL_CATALOG_COMMAND => self.handle_tool_catalog(command).await,
            MCP_TOOL_ATTACH_COMMAND => self.handle_tool_attach(command).await,
            MCP_TOOL_INVOKE_COMMAND => self.handle_tool_invoke(command, trace).await,
            MCP_STATUS_COMMAND => self.handle_status(command).await,
            MCP_SERVER_STATUS_LIST_COMMAND => self.handle_server_status_list(command).await,
            MCP_RELOAD_COMMAND => self.handle_reload(command).await,
            MCP_EXPOSURE_REFRESH_COMMAND => self.handle_exposure_refresh(command).await,
            MCP_OAUTH_LOGIN_COMMAND => self.handle_oauth_login(command).await,
            MCP_OAUTH_STATUS_COMMAND => self.handle_oauth_status(command).await,
            MCP_RESOURCE_LIST_COMMAND => self.handle_resource_list(command).await,
            MCP_RESOURCE_TEMPLATE_LIST_COMMAND => self.handle_resource_template_list(command).await,
            MCP_RESOURCE_READ_COMMAND => self.handle_resource_read(command).await,
            MCP_DIAGNOSTICS_SNAPSHOT_COMMAND => self.handle_diagnostics_snapshot(command).await,
            MCP_SNAPSHOT_COMMAND => self.handle_snapshot(command).await,
            MCP_CLEANUP_COMMAND => self.handle_cleanup(command).await,
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported MCP service command '{other}'"
            ))),
        }
    }
}
