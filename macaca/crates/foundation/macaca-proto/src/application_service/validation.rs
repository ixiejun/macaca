//! Internal validation helpers shared across Application Service command builders.
//!
//! These guards enforce fail-closed trace and non-empty string contracts before
//! commands cross the service boundary, keeping audit envelopes trustworthy for
//! SDK clients, runtime-host providers, and future remote transports.

use crate::{MacacaError, MacacaResult, TraceContext};

/// Rejects commands whose trace envelope lacks a non-empty `trace_id`.
///
/// Every Application Service command must carry a trace so providers can emit
/// correlated audit events without synthesizing identifiers at the boundary.
pub(crate) fn validate_trace(trace: &TraceContext, message: &'static str) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(())
}

/// Trims and rejects empty strings used as required command fields.
///
/// Scope identifiers such as `session_id` must be explicit so providers never
/// infer defaults that could cross tenant or application isolation boundaries.
pub(crate) fn non_empty(value: String, message: &'static str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
