//! Adapter implementations mapping typed Heartbeat DTOs to `ServiceCommand`.
//!
//! **Pattern:** Adapter — keeps serde/command-name wiring out of the pure DTO module so
//! contract types stay stable while transport encoding evolves independently. Each adapter
//! preserves the caller's `TraceContext` on the envelope for auditable service boundaries.

use crate::{MacacaResult, ServiceCommand, ServiceCommandName};

use super::{
    HeartbeatCompleteRunCommand, HeartbeatWakeCommand, HEARTBEAT_COMPLETE_RUN_COMMAND,
    HEARTBEAT_WAKE_COMMAND,
};

impl HeartbeatWakeCommand {
    /// Convert this typed wake DTO into the generic service runtime command shape.
    ///
    /// Runtime-host and SDK clients call this adapter at the service boundary so wake intent,
    /// scope keys, and trace correlation survive JSON transport without leaking provider logic
    /// into the DTO definitions.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(HEARTBEAT_WAKE_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

impl HeartbeatCompleteRunCommand {
    /// Convert this typed completion DTO into the generic service runtime command shape.
    ///
    /// Runtime-host reports terminal wake outcomes through this adapter after delegated work
    /// finishes, keeping heartbeat run mementos auditable without storing raw execution payloads.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(HEARTBEAT_COMPLETE_RUN_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}
