//! CLI output helpers for Skill command results.
//!
//! Presents bounded JSON to stdout for operator tooling and CI diagnostics.
//! Offline diagnostics are rendered structurally rather than panicking, so
//! scripts can distinguish unavailable service runtime from transport failure.

use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use tracing::warn;

/// Render the offline Skill diagnostic path without importing host-backed SDK contracts.
pub(crate) fn print_unavailable(trace: TraceContext, command: &'static str) -> MacacaResult<()> {
    warn!(
        trace_id = %trace.trace_id,
        command,
        "CLI Skill command requires a live Skill service runtime"
    );
    print_json(serde_json::json!({
        "trace_id": trace.trace_id,
        "status": "unavailable_or_denied",
        "error": "Skill service is unavailable; pass a live application id to route through the Web API facade",
    }))
}

/// Serialize a JSON value to stdout with stable pretty-print formatting.
pub(crate) fn print_json(value: serde_json::Value) -> MacacaResult<()> {
    let rendered = serde_json::to_string_pretty(&value).map_err(MacacaError::from)?;
    println!("{rendered}");
    Ok(())
}
