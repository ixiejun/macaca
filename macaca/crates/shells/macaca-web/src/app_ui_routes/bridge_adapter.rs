//! Bridge envelope **Adapter** — trace ids, responses, and structured rejections.
//!
//! Rejections always return provider-neutral `DisabledByPolicy` status with
//! `reason_code` metadata so UI clients can render failures without parsing
//! application-specific error shapes.

use std::collections::BTreeSet;

use macaca_proto::{
    ApplicationHostCommandResult, ApplicationHostCommandStatus, ApplicationUiBridgeRequest,
    ApplicationUiBridgeResponse, TraceContext,
};

/// Build a trace context from the bridge request (command_id fallback).
pub(crate) fn bridge_trace(request: &ApplicationUiBridgeRequest) -> TraceContext {
    TraceContext::new(
        request
            .trace_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(request.command_id.as_str()),
    )
}

/// Wrap a host command result in the stable bridge response envelope.
pub(crate) fn bridge_response(
    request: &ApplicationUiBridgeRequest,
    accepted: bool,
    result: ApplicationHostCommandResult,
) -> ApplicationUiBridgeResponse {
    ApplicationUiBridgeResponse {
        bridge_version: "macaca.ui.bridge.v1".into(),
        command_id: request.command_id.clone(),
        accepted,
        result,
    }
}

/// Build a structured bridge rejection result with trace correlation.
pub(crate) fn rejected_bridge_result(
    reason: impl Into<String>,
    trace: TraceContext,
) -> ApplicationHostCommandResult {
    let reason = reason.into();
    let mut result = ApplicationHostCommandResult {
        status: ApplicationHostCommandStatus::DisabledByPolicy {
            reason: reason.clone(),
        },
        output: serde_json::Value::Null,
        trace: Some(trace),
        policy: None,
        metadata: Default::default(),
    };
    result
        .metadata
        .insert("reason_code".into(), "ui_bridge_rejected".into());
    result.metadata.insert("reason".into(), reason);
    result
}

/// Reject service calls to targets not declared in the application manifest.
pub(crate) fn rejected_undeclared_service_result(
    service_id: &str,
    declared_services: &BTreeSet<String>,
    trace: TraceContext,
) -> Option<ApplicationHostCommandResult> {
    if declared_services.contains(service_id) {
        return None;
    }
    tracing::warn!(
        service_id,
        trace_id = %trace.trace_id,
        declared_service_count = declared_services.len(),
        command = "app.ui.bridge.service.call",
        "application-owned UI bridge rejected undeclared service call"
    );
    let mut result = rejected_bridge_result(
        "service.call target service is not declared by the application manifest",
        trace,
    );
    result
        .metadata
        .insert("service_id".into(), service_id.to_string());
    Some(result)
}
