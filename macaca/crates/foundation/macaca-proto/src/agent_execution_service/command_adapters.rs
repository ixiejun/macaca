//! Adapter implementations mapping typed agent-execution DTOs to `ServiceCommand`.
//!
//! **Pattern:** Adapter — keeps serde/command-name wiring out of the pure DTO module so
//! contract types stay stable while transport encoding evolves independently. Each adapter
//! preserves the caller's `TraceContext` on the envelope for auditable service boundaries.

use crate::{MacacaResult, ServiceCommand, ServiceCommandName};

use super::{
    AgentContextBuildCommand, AgentExecutionCommand, AGENT_CONTEXT_BUILD_COMMAND,
    AGENT_EXECUTE_COMMAND,
};

impl AgentExecutionCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(AGENT_EXECUTE_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

impl AgentContextBuildCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(AGENT_CONTEXT_BUILD_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}
