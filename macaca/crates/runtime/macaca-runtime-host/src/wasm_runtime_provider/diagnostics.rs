//! Shared diagnostic helpers for runtime provider modules.
//!
//! Helpers in this file centralize trace extraction and session metadata so
//! providers emit consistent audit fields without logging guest payloads or raw
//! WASM bytes.

use macaca_proto::{TraceContext, WasmRuntimeSessionRequest};

/// Build the stable session id used by providers and execution sessions.
pub(crate) fn session_id_from_request(request: &WasmRuntimeSessionRequest) -> Option<String> {
    request.trace.as_ref().map(|trace| {
        format!(
            "{}:{}:{}",
            trace.trace_id, request.application_id, request.ability_id
        )
    })
}

/// Return the trace id when it exists and is not only whitespace.
pub(crate) fn non_empty_trace(trace: Option<TraceContext>) -> Option<TraceContext> {
    trace.filter(|value| !value.trace_id.trim().is_empty())
}
