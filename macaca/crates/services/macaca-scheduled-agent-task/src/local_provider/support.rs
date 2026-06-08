//! Shared constants and structured error builders for the local provider.

use std::collections::BTreeMap;

use macaca_proto::{
    AutonomyAuditCorrelation, AutonomyServiceErrorKind, AutonomyStructuredError, MacacaResult,
    ScheduledAgentTaskCommandResult, TraceContext,
};

/// Local provider identifier used in sanitized snapshots.
pub(super) const LOCAL_PROVIDER_ID: &str = "local.in_memory";

/// Build a structured autonomy error envelope from trace + reason metadata.
pub(super) fn structured_error(
    trace: TraceContext,
    kind: AutonomyServiceErrorKind,
    reason_code: impl Into<String>,
    safe_message: impl Into<String>,
) -> MacacaResult<AutonomyStructuredError> {
    Ok(AutonomyStructuredError {
        kind,
        reason_code: reason_code.into(),
        safe_message: safe_message.into(),
        correlation: AutonomyAuditCorrelation::from_trace(trace)?,
        metadata: BTreeMap::new(),
    })
}

/// Build a rejected command result carrying a structured error payload.
pub(super) fn error_result(
    trace: TraceContext,
    kind: AutonomyServiceErrorKind,
    reason_code: impl Into<String>,
    safe_message: impl Into<String>,
) -> MacacaResult<ScheduledAgentTaskCommandResult> {
    let error = structured_error(trace.clone(), kind, reason_code, safe_message)?;
    Ok(ScheduledAgentTaskCommandResult::rejected(trace, error))
}
