//! Command Router for the built-in Application system service provider.
//!
//! **Pattern:** Command — each `APPLICATION_*` command name maps to a focused
//! handler method.  The router stays free of business logic so new commands can
//! be added without growing a monolithic `call` body.

use macaca_proto::{
    ServiceCallResult, ServiceCommand, ServiceError, ServiceResult, TraceContext,
    APPLICATION_AGENT_DELEGATE_COMMAND, APPLICATION_DISCOVER_COMMAND,
    APPLICATION_GENUI_SURFACE_COMMAND, APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND,
    APPLICATION_HOST_DISPATCH_COMMAND, APPLICATION_LOAD_COMMAND,
    APPLICATION_METADATA_QUERY_COMMAND, APPLICATION_REMOVE_COMMAND,
    APPLICATION_SESSION_RESUME_COMMAND, APPLICATION_SESSION_START_COMMAND,
    APPLICATION_SESSION_STOP_COMMAND, APPLICATION_SNAPSHOT_COMMAND, APPLICATION_START_COMMAND,
    APPLICATION_STATUS_COMMAND, APPLICATION_STOP_COMMAND,
};

use super::ApplicationSystemServiceProvider;

impl ApplicationSystemServiceProvider {
    /// Route a decoded Application service command to its handler after trace admission.
    pub(super) async fn dispatch_command(
        &self,
        command: ServiceCommand,
        _trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        match command.name.as_str() {
            APPLICATION_DISCOVER_COMMAND => self.handle_discover(command).await,
            APPLICATION_START_COMMAND => self.handle_start(command).await,
            APPLICATION_STATUS_COMMAND => self.handle_status(command).await,
            APPLICATION_METADATA_QUERY_COMMAND => self.handle_metadata_query(command).await,
            APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND => {
                self.handle_heartbeat_agents_query(command).await
            }
            APPLICATION_SNAPSHOT_COMMAND => self.handle_snapshot(command).await,
            APPLICATION_SESSION_START_COMMAND => self.handle_session_start(command).await,
            APPLICATION_SESSION_RESUME_COMMAND => self.handle_session_resume(command).await,
            APPLICATION_SESSION_STOP_COMMAND => self.handle_session_stop(command).await,
            APPLICATION_HOST_DISPATCH_COMMAND => self.handle_host_dispatch(command).await,
            APPLICATION_AGENT_DELEGATE_COMMAND => self.handle_agent_delegate(command).await,
            APPLICATION_GENUI_SURFACE_COMMAND => self.handle_genui_surface(command).await,
            APPLICATION_LOAD_COMMAND => self.handle_load(command).await,
            APPLICATION_STOP_COMMAND => self.handle_stop(command).await,
            APPLICATION_REMOVE_COMMAND => self.handle_remove(command).await,
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Application service command '{other}'"
            ))),
        }
    }
}
