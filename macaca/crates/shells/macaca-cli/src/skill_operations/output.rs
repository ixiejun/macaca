//! CLI output helpers for Skill command results.
//!
//! Presents bounded JSON to stdout for operator tooling and CI diagnostics.
//! Errors from the SDK Null Object path are rendered structurally rather than
//! panicking, so scripts can distinguish unavailable from transport failure.

use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use tracing::warn;

/// Render a successful or unavailable SDK result as pretty JSON on stdout.
pub(crate) fn print_sdk_result<T: serde::Serialize>(
    trace: TraceContext,
    result: MacacaResult<T>,
) -> MacacaResult<()> {
    match result {
        Ok(result) => print_json(serde_json::json!({
            "trace_id": trace.trace_id,
            "status": "ok",
            "result": result,
        })),
        Err(error) => {
            warn!(
                trace_id = %trace.trace_id,
                error_class = "unavailable_or_denied",
                "CLI Skill command returned structured service error"
            );
            print_json(serde_json::json!({
                "trace_id": trace.trace_id,
                "status": "unavailable_or_denied",
                "error": error.to_string(),
            }))
        }
    }
}

/// Serialize a JSON value to stdout with stable pretty-print formatting.
pub(crate) fn print_json(value: serde_json::Value) -> MacacaResult<()> {
    let rendered = serde_json::to_string_pretty(&value).map_err(MacacaError::from)?;
    println!("{rendered}");
    Ok(())
}
