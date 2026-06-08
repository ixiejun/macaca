//! Strategy dispatcher entry point for Scheduler target categories.
//!
//! **Pattern:** Strategy + Command Router — `dispatch` matches on
//! `SchedulerTargetCommand` variants and delegates execution to focused handler
//! modules while keeping Scheduler free of direct service-runtime calls.

use macaca_proto::{
    AutonomyScope, HeartbeatWakeCommand, HeartbeatWakeIntent, MacacaResult,
    SchedulerTargetCommand, TraceContext,
};
use tracing::warn;

use crate::ServiceRuntime;

use super::agent_execution_dispatch;
use super::outcome::AutonomyDispatchOutcome;
use super::service_dispatch;

/// Strategy dispatcher for Scheduler target categories.
///
/// Runtime Host owns the dispatch boundary between Scheduler durable run state
/// and `ServiceRuntime` syscall surfaces.  Handler modules receive shared
/// `runtime` and `timeout_ms` through `pub(crate)` fields so each Strategy can
/// issue bounded `ServiceRuntime::call` without duplicating construction logic.
pub struct AutonomyDispatchStrategies<'a> {
    /// Shared service runtime used for all dispatch syscall boundaries.
    pub(crate) runtime: &'a ServiceRuntime,
    /// Upper bound for each downstream service call, in milliseconds (minimum 1).
    pub(crate) timeout_ms: u64,
}

impl<'a> AutonomyDispatchStrategies<'a> {
    /// Create a dispatcher over the existing service runtime boundary.
    ///
    /// `timeout_ms` is clamped to at least 1 so `tokio::time::timeout` always
    /// receives a positive duration even when configuration supplies zero.
    pub fn new(runtime: &'a ServiceRuntime, timeout_ms: u64) -> Self {
        Self {
            runtime,
            timeout_ms: timeout_ms.max(1),
        }
    }

    /// Dispatch one provider-neutral target without inspecting business data.
    ///
    /// **Pattern:** Command Router — each `SchedulerTargetCommand` variant maps
    /// to a dedicated handler module.  Unsupported categories return explicit
    /// skipped outcomes rather than panicking or fabricating success.
    pub async fn dispatch(
        &self,
        trace: TraceContext,
        scope: AutonomyScope,
        target: SchedulerTargetCommand,
    ) -> MacacaResult<AutonomyDispatchOutcome> {
        match target {
            SchedulerTargetCommand::Service(command) => {
                service_dispatch::dispatch_service(
                    self,
                    trace,
                    command.service_id,
                    command.command_name,
                )
                .await
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
                service_dispatch::dispatch_heartbeat(self, wake).await
            }
            SchedulerTargetCommand::AgentExecution(command) => {
                agent_execution_dispatch::dispatch_agent_execution(self, trace, command).await
            }
            SchedulerTargetCommand::Application(_) => {
                warn!(
                    trace_id = trace.trace_id.as_str(),
                    "autonomy supervisor skipped application target because application dispatch strategy is not active yet"
                );
                Ok(AutonomyDispatchOutcome::skipped(
                    "application_strategy_unavailable",
                ))
            }
            SchedulerTargetCommand::Plugin(_) => {
                warn!(
                    trace_id = trace.trace_id.as_str(),
                    "autonomy supervisor skipped plugin target because plugin dispatch strategy is not active yet"
                );
                Ok(AutonomyDispatchOutcome::skipped("plugin_strategy_unavailable"))
            }
        }
    }
}
