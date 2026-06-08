//! Adapter implementations mapping typed scheduled-agent-task DTOs to `ServiceCommand`.
//!
//! **Pattern:** Adapter — keeps serde/command-name wiring out of the pure DTO module so
//! contract types stay stable while transport encoding evolves independently. Each adapter
//! preserves the caller's `TraceContext` on the envelope for auditable service boundaries.

use crate::{MacacaResult, ServiceCommand, ServiceCommandName};

use super::{
    CreateScheduledAgentTaskCommand, RecordScheduledAgentTaskResultCommand,
    SCHEDULED_AGENT_TASK_CREATE_COMMAND, SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND,
};

impl CreateScheduledAgentTaskCommand {
    /// Convert this typed command into the generic service runtime shape.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(SCHEDULED_AGENT_TASK_CREATE_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

impl RecordScheduledAgentTaskResultCommand {
    /// Convert this typed command into the generic service runtime shape.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}
