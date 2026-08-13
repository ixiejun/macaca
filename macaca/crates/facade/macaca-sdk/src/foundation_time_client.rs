//! SDK Facade helpers for `pack.foundation.time.v1`.
//!
//! The helper composes already-evaluated admission evidence with a canonical
//! traced service command. It does not read clocks, create timers, or expose a
//! native host handle, preserving the provider-neutral SDK boundary.

use macaca_proto::{TimeAdmissionFailure, TimeResourceReservation, TraceContext};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use crate::service_client::ServiceCallCommand;

const SERVICE_ID: &str = "service.foundation.time";

/// Outcome of a time command admission and canonical SDK construction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeDomainPackCommandBuildOutcome {
    Ready(ServiceCallCommand),
    Rejected(TimeAdmissionFailure),
}

/// Provider-neutral Facade for typed time command construction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeDomainPackCommandBuilder {
    command_name: String,
    payload: serde_json::Value,
    decision: Result<TimeResourceReservation, TimeAdmissionFailure>,
    trace: TraceContext,
}

impl TimeDomainPackCommandBuilder {
    /// Build a time helper from preflight evidence supplied by the caller's policy layer.
    pub fn new(
        command_name: impl Into<String>,
        payload: serde_json::Value,
        decision: Result<TimeResourceReservation, TimeAdmissionFailure>,
        trace: TraceContext,
    ) -> Self {
        Self {
            command_name: command_name.into(),
            payload,
            decision,
            trace,
        }
    }

    /// Produce only an admitted traced service call. Rejections remain side-effect free.
    pub fn build(
        self,
        resolved: &DomainPackResolveResult,
    ) -> macaca_proto::MacacaResult<TimeDomainPackCommandBuildOutcome> {
        match self.decision {
            Ok(_) => {
                info!(service_id = SERVICE_ID, command = %self.command_name, trace_id = %self.trace.trace_id, "time_pack_sdk_preflight_allowed");
                Ok(TimeDomainPackCommandBuildOutcome::Ready(
                    DomainPackServiceCallBuilder::new(
                        SERVICE_ID,
                        self.command_name,
                        self.payload,
                        self.trace,
                    )?
                    .build(resolved)?,
                ))
            }
            Err(reason) => {
                warn!(service_id = SERVICE_ID, trace_id = %self.trace.trace_id, status = ?reason, "time_pack_sdk_preflight_rejected");
                Ok(TimeDomainPackCommandBuildOutcome::Rejected(reason))
            }
        }
    }
}

/// Create a monotonic-timeout command through the common runtime path.
pub fn monotonic_timeout_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> TimeDomainPackCommandBuilder {
    TimeDomainPackCommandBuilder::new(
        "time.evaluate_deadline",
        payload,
        Ok(default_reservation()),
        trace,
    )
}
/// Create timezone conversion through the common runtime path.
pub fn timezone_conversion_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> TimeDomainPackCommandBuilder {
    TimeDomainPackCommandBuilder::new(
        "time.convert_timezone",
        payload,
        Ok(default_reservation()),
        trace,
    )
}
/// Create localized formatting through the common runtime path.
pub fn localized_format_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> TimeDomainPackCommandBuilder {
    TimeDomainPackCommandBuilder::new("time.format", payload, Ok(default_reservation()), trace)
}
/// Create strict parsing through the common runtime path.
pub fn strict_parse_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> TimeDomainPackCommandBuilder {
    TimeDomainPackCommandBuilder::new("time.parse", payload, Ok(default_reservation()), trace)
}
/// Create a timer after policy admission reserves a timer slot.
pub fn timer_create_command(
    payload: serde_json::Value,
    decision: Result<TimeResourceReservation, TimeAdmissionFailure>,
    trace: TraceContext,
) -> TimeDomainPackCommandBuilder {
    TimeDomainPackCommandBuilder::new("time.create_timer", payload, decision, trace)
}
/// Cancel a timer through the common runtime path.
pub fn timer_cancel_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> TimeDomainPackCommandBuilder {
    TimeDomainPackCommandBuilder::new(
        "time.cancel_timer",
        payload,
        Ok(default_reservation()),
        trace,
    )
}
/// Read clock health or mock-clock diagnostics through the common runtime path.
pub fn clock_health_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> TimeDomainPackCommandBuilder {
    TimeDomainPackCommandBuilder::new(
        "time.clock_health",
        payload,
        Ok(default_reservation()),
        trace,
    )
}

fn default_reservation() -> TimeResourceReservation {
    TimeResourceReservation {
        reservation_id: "read-only".into(),
        timer_count: 0,
        duration_ms: 0,
    }
}

#[cfg(test)]
#[path = "foundation_time_client_tests.rs"]
mod tests;
