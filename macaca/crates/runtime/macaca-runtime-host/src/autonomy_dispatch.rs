//! Provider-neutral dispatch strategies used by the autonomy supervisor.
//!
//! Scheduler owns durable run state, but it must not execute target commands
//! directly.  This module keeps dispatch as runtime-host strategy code that
//! routes through existing service boundaries and records only safe status
//! classes.  Unsupported target categories are explicit skipped outcomes rather
//! than panics or fake success.

use std::time::Duration;

use macaca_proto::{
    AutonomyScope, HeartbeatWakeCommand, HeartbeatWakeIntent, KernelServiceId, MacacaResult,
    SchedulerTargetCommand, ServiceBusSource, ServiceCommand, ServiceCommandName, TraceContext,
    HEARTBEAT_SERVICE_ID,
};
use tokio::time::timeout;
use tracing::{info, warn};

use crate::ServiceRuntime;

/// Safe result class returned to Scheduler run-control transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyDispatchOutcome {
    pub succeeded: bool,
    pub retryable: bool,
    pub reason_code: &'static str,
}

impl AutonomyDispatchOutcome {
    /// Build a successful dispatch outcome.
    pub fn succeeded() -> Self {
        Self {
            succeeded: true,
            retryable: false,
            reason_code: "dispatch_succeeded",
        }
    }

    /// Build a bounded failure outcome that Scheduler may retry.
    pub fn retryable(reason_code: &'static str) -> Self {
        Self {
            succeeded: false,
            retryable: true,
            reason_code,
        }
    }

    /// Build a final skipped outcome for unsupported generic categories.
    pub fn skipped(reason_code: &'static str) -> Self {
        Self {
            succeeded: false,
            retryable: false,
            reason_code,
        }
    }
}

/// Strategy dispatcher for Scheduler target categories.
pub struct AutonomyDispatchStrategies<'a> {
    runtime: &'a ServiceRuntime,
    timeout_ms: u64,
}

impl<'a> AutonomyDispatchStrategies<'a> {
    /// Create a dispatcher over the existing service runtime boundary.
    pub fn new(runtime: &'a ServiceRuntime, timeout_ms: u64) -> Self {
        Self {
            runtime,
            timeout_ms: timeout_ms.max(1),
        }
    }

    /// Dispatch one provider-neutral target without inspecting business data.
    pub async fn dispatch(
        &self,
        trace: TraceContext,
        scope: AutonomyScope,
        target: SchedulerTargetCommand,
    ) -> MacacaResult<AutonomyDispatchOutcome> {
        match target {
            SchedulerTargetCommand::Service(command) => {
                self.dispatch_service(trace, command.service_id, command.command_name).await
            }
            SchedulerTargetCommand::HeartbeatWake(command) => {
                let mut wake = HeartbeatWakeCommand::new(
                    trace,
                    scope,
                    command.wake_scope_key,
                    HeartbeatWakeIntent::ScheduledTick,
                )?;
                wake.payload_ref = command.payload_ref;
                wake.metadata = command.metadata;
                self.dispatch_heartbeat(wake).await
            }
            SchedulerTargetCommand::AgentExecution(_) => {
                warn!("autonomy supervisor skipped agent execution target because no generic execution dispatch strategy is active yet");
                Ok(AutonomyDispatchOutcome::skipped(
                    "agent_execution_strategy_unavailable",
                ))
            }
            SchedulerTargetCommand::Application(_) => {
                warn!("autonomy supervisor skipped application target because application dispatch strategy is not active yet");
                Ok(AutonomyDispatchOutcome::skipped(
                    "application_strategy_unavailable",
                ))
            }
            SchedulerTargetCommand::Plugin(_) => {
                warn!("autonomy supervisor skipped plugin target because plugin dispatch strategy is not active yet");
                Ok(AutonomyDispatchOutcome::skipped(
                    "plugin_strategy_unavailable",
                ))
            }
        }
    }

    async fn dispatch_service(
        &self,
        trace: TraceContext,
        service_id: KernelServiceId,
        command_name: ServiceCommandName,
    ) -> MacacaResult<AutonomyDispatchOutcome> {
        info!(
            service_id = %service_id,
            command = %command_name,
            trace_id = trace.trace_id.as_str(),
            "autonomy supervisor dispatching service command through ServiceRuntime"
        );
        let command = ServiceCommand::with_trace(
            command_name,
            serde_json::json!({"payload_ref": null}),
            trace,
        );
        let result = timeout(
            Duration::from_millis(self.timeout_ms),
            self.runtime.call(
                &service_id,
                ServiceBusSource::new("runtime.autonomy_supervisor"),
                command,
            ),
        )
        .await;
        match result {
            Ok(Ok(reply)) if reply.success => Ok(AutonomyDispatchOutcome::succeeded()),
            Ok(Ok(_)) => Ok(AutonomyDispatchOutcome::retryable("service_reply_failed")),
            Ok(Err(error)) => {
                warn!(
                    service_id = %service_id,
                    error = %error,
                    "autonomy supervisor service dispatch failed"
                );
                Ok(AutonomyDispatchOutcome::retryable("service_dispatch_failed"))
            }
            Err(_) => Ok(AutonomyDispatchOutcome::retryable("service_dispatch_timeout")),
        }
    }

    async fn dispatch_heartbeat(
        &self,
        wake: HeartbeatWakeCommand,
    ) -> MacacaResult<AutonomyDispatchOutcome> {
        let trace_id = wake.trace.trace_id.clone();
        let command = wake.into_service_command()?;
        let service_id = KernelServiceId::new(HEARTBEAT_SERVICE_ID);
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            trace_id = trace_id.as_str(),
            "autonomy supervisor dispatching heartbeat wake through ServiceRuntime"
        );
        match timeout(
            Duration::from_millis(self.timeout_ms),
            self.runtime.call(
                &service_id,
                ServiceBusSource::new("runtime.autonomy_supervisor"),
                command,
            ),
        )
        .await
        {
            Ok(Ok(reply)) if reply.success => Ok(AutonomyDispatchOutcome::succeeded()),
            Ok(Ok(_)) => Ok(AutonomyDispatchOutcome::retryable("heartbeat_reply_failed")),
            Ok(Err(error)) => {
                warn!(
                    service_id = HEARTBEAT_SERVICE_ID,
                    error = %error,
                    "autonomy supervisor heartbeat dispatch failed"
                );
                Ok(AutonomyDispatchOutcome::retryable("heartbeat_dispatch_failed"))
            }
            Err(_) => Ok(AutonomyDispatchOutcome::retryable("heartbeat_dispatch_timeout")),
        }
    }
}
