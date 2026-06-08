//! Daemon response mapping for hardened WASM execution sessions.
//!
//! Mapping stays isolated from dispatch orchestration so malformed daemon
//! payloads fail closed with sanitized metadata rather than leaking raw daemon
//! text into host command results or audit logs.

use macaca_proto::{
    ApplicationHostCommandResult, ApplicationHostCommandStatus, TraceContext,
};

use super::super::hardened_transport::{sanitize_hardened_text, sanitize_label};
use super::super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use super::super::WasmHardenedProviderResponse;
use tracing::warn;

/// Map one sanitized daemon response into a host command result.
pub(super) fn map_daemon_response(
    session_id: &str,
    telemetry: &Option<WasmTelemetrySinkRef>,
    response: WasmHardenedProviderResponse,
    trace: TraceContext,
) -> ApplicationHostCommandResult {
    let expected_trace = trace.trace_id.as_str();
    if response.trace_id != expected_trace || response.request_id != session_id {
        warn!(
            session_id = %session_id,
            trace_id = %trace.trace_id,
            provider_class = "hardened_out_of_process",
            reason_code = "malformed_response",
            "WASM hardened provider rejected malformed daemon response"
        );
        emit_wasm_telemetry(
            telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Daemon,
                "malformed_response",
                "hardened_out_of_process",
            )
            .trace_id(trace.trace_id.clone())
            .session_id(session_id.to_string())
            .reason_code("malformed_response"),
        );
        return fail_closed_result(
            session_id,
            "malformed_response",
            "hardened daemon returned malformed response",
            trace,
            ApplicationHostCommandStatus::RuntimeUnavailable {
                reason: "hardened daemon returned malformed response".into(),
            },
        );
    }
    let mut result = match response.status {
        ApplicationHostCommandStatus::Ok => {
            ApplicationHostCommandResult::ok(serde_json::Value::Null, Some(trace))
        }
        status => ApplicationHostCommandResult {
            status,
            output: serde_json::Value::Null,
            trace: Some(trace),
            policy: None,
            metadata: Default::default(),
        },
    };
    result.metadata.extend(response.metadata);
    result
        .metadata
        .insert("reason_code".into(), sanitize_label(response.reason_code));
    result
        .metadata
        .insert("provider_class".into(), "hardened_out_of_process".into());
    result
        .metadata
        .insert("session_id".into(), session_id.to_string());
    emit_wasm_telemetry(
        telemetry.as_ref(),
        WasmTelemetryEvent::new(
            WasmTelemetryStage::Daemon,
            "completed",
            "hardened_out_of_process",
        )
        .trace_id(
            result
                .trace
                .as_ref()
                .map(|value| value.trace_id.as_str())
                .unwrap_or("none"),
        )
        .session_id(session_id.to_string())
        .reason_code(
            result
                .metadata
                .get("reason_code")
                .cloned()
                .unwrap_or_else(|| "completed".into()),
        ),
    );
    if !response.diagnostics.is_empty() {
        result.metadata.insert(
            "diagnostics".into(),
            sanitize_hardened_text(response.diagnostics.join(";")),
        );
    }
    result
}

/// Build a fail-closed host result with sanitized diagnostics metadata.
pub(super) fn fail_closed_result(
    session_id: &str,
    reason_code: &str,
    reason: &str,
    trace: TraceContext,
    status: ApplicationHostCommandStatus,
) -> ApplicationHostCommandResult {
    let mut result = ApplicationHostCommandResult {
        status,
        output: serde_json::Value::Null,
        trace: Some(trace),
        policy: None,
        metadata: Default::default(),
    };
    result
        .metadata
        .insert("reason_code".into(), reason_code.into());
    result
        .metadata
        .insert("provider_class".into(), "hardened_out_of_process".into());
    result
        .metadata
        .insert("session_id".into(), session_id.to_string());
    result
        .metadata
        .insert("diagnostics".into(), sanitize_hardened_text(reason));
    result
}
