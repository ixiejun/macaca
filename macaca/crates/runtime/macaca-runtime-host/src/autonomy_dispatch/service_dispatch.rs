//! Generic service and heartbeat dispatch handlers.
//!
//! **Pattern:** Adapter — translates Scheduler service/heartbeat targets into
//! `ServiceRuntime::call` syscalls with bounded timeouts and safe outcome
//! classification for run-control transitions.

use std::time::Duration;

use macaca_proto::{
    HeartbeatWakeCommand, KernelServiceId, MacacaResult, ServiceBusSource, ServiceCommand,
    ServiceCommandName, TraceContext, HEARTBEAT_SERVICE_ID,
};
use tokio::time::timeout;
use tracing::{info, warn};

use super::outcome::AutonomyDispatchOutcome;
use super::strategies::AutonomyDispatchStrategies;

/// Dispatch a generic service command through `ServiceRuntime`.
///
/// Scheduler supplies only `service_id` and `command_name`; this handler builds
/// a minimal provider-neutral payload and records success/retry/skip classes
/// from the service reply envelope.
pub(crate) async fn dispatch_service(
    strategies: &AutonomyDispatchStrategies<'_>,
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
        Duration::from_millis(strategies.timeout_ms),
        strategies.runtime.call(
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
            Ok(AutonomyDispatchOutcome::retryable(
                "service_dispatch_failed",
            ))
        }
        Err(_) => Ok(AutonomyDispatchOutcome::retryable(
            "service_dispatch_timeout",
        )),
    }
}

/// Dispatch a heartbeat wake command through `ServiceRuntime`.
///
/// Heartbeat dispatch reuses the same bounded-timeout and outcome taxonomy as
/// generic service dispatch so Scheduler run-control logic stays uniform.
pub(crate) async fn dispatch_heartbeat(
    strategies: &AutonomyDispatchStrategies<'_>,
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
        Duration::from_millis(strategies.timeout_ms),
        strategies.runtime.call(
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
            Ok(AutonomyDispatchOutcome::retryable(
                "heartbeat_dispatch_failed",
            ))
        }
        Err(_) => Ok(AutonomyDispatchOutcome::retryable(
            "heartbeat_dispatch_timeout",
        )),
    }
}
