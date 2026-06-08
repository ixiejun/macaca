//! Adapter implementations mapping typed execution-control DTOs to `ServiceCommand`.
//!
//! Each `into_service_command` preserves the caller's `TraceContext` on the envelope so
//! pause/resume boundaries remain auditable across SDK, runtime-host, and future remote
//! providers without duplicating serialization rules in every adapter.

use serde::Serialize;

use crate::{MacacaResult, ServiceCommand, ServiceCommandName, TraceContext};

use super::{
    ExecutionControlAwaitResumeCommand, ExecutionControlCancelWaitCommand,
    ExecutionControlCheckpointCommand, ExecutionControlPauseCommand,
    ExecutionControlRegisterExecutionCommand, ExecutionControlResolvePolicyCommand,
    ExecutionControlResumeCommand, ExecutionControlSnapshotCommand, ExecutionControlStateCommand,
    EXECUTION_CONTROL_AWAIT_RESUME_COMMAND, EXECUTION_CONTROL_CANCEL_WAIT_COMMAND,
    EXECUTION_CONTROL_QUERY_STATE_COMMAND, EXECUTION_CONTROL_RECORD_CHECKPOINT_COMMAND,
    EXECUTION_CONTROL_REGISTER_EXECUTION_COMMAND, EXECUTION_CONTROL_REQUEST_PAUSE_COMMAND,
    EXECUTION_CONTROL_REQUEST_RESUME_COMMAND, EXECUTION_CONTROL_RESOLVE_POLICY_COMMAND,
    EXECUTION_CONTROL_SNAPSHOT_COMMAND,
};

impl ExecutionControlResolvePolicyCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        service_command(
            EXECUTION_CONTROL_RESOLVE_POLICY_COMMAND,
            self.scope.trace.clone(),
            self,
        )
    }
}

impl ExecutionControlRegisterExecutionCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        service_command(
            EXECUTION_CONTROL_REGISTER_EXECUTION_COMMAND,
            self.scope.trace.clone(),
            self,
        )
    }
}

impl ExecutionControlPauseCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        service_command(
            EXECUTION_CONTROL_REQUEST_PAUSE_COMMAND,
            self.scope.trace.clone(),
            self,
        )
    }
}

impl ExecutionControlCheckpointCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        service_command(
            EXECUTION_CONTROL_RECORD_CHECKPOINT_COMMAND,
            self.scope.trace.clone(),
            self,
        )
    }
}

impl ExecutionControlAwaitResumeCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        service_command(
            EXECUTION_CONTROL_AWAIT_RESUME_COMMAND,
            self.scope.trace.clone(),
            self,
        )
    }
}

impl ExecutionControlCancelWaitCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        service_command(
            EXECUTION_CONTROL_CANCEL_WAIT_COMMAND,
            self.scope.trace.clone(),
            self,
        )
    }
}

impl ExecutionControlResumeCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        service_command(
            EXECUTION_CONTROL_REQUEST_RESUME_COMMAND,
            self.scope.trace.clone(),
            self,
        )
    }
}

impl ExecutionControlStateCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        service_command(
            EXECUTION_CONTROL_QUERY_STATE_COMMAND,
            self.scope.trace.clone(),
            self,
        )
    }
}

impl ExecutionControlSnapshotCommand {
    /// Convert this typed command into a provider-neutral service command.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        service_command(
            EXECUTION_CONTROL_SNAPSHOT_COMMAND,
            self.scope.trace.clone(),
            self,
        )
    }
}

fn service_command<T>(
    name: &'static str,
    trace: TraceContext,
    payload: T,
) -> MacacaResult<ServiceCommand>
where
    T: Serialize,
{
    Ok(ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload)?,
        trace,
    ))
}
