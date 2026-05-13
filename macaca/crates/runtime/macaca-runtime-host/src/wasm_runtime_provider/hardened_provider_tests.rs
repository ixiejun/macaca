use std::sync::Arc;

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, TraceContext,
    WasmExecutionProfile, WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};
use serde_json::json;

use super::{
    HardenedOutOfProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider,
    WasmHardenedProviderResponse,
};
use crate::wasm_runtime_provider::hardened_transport::{
    InMemoryHardenedTransport, WasmHardenedHealth,
};

#[tokio::test]
async fn hardened_provider_fails_closed_when_daemon_unavailable() {
    let provider = HardenedOutOfProcessWasmRuntimeProvider::new(Arc::new(
        InMemoryHardenedTransport::unavailable("daemon offline with API_KEY"),
    ));

    let error = provider
        .create_session(traced_request("trace-hardened-unavailable"))
        .await
        .unwrap_err();
    let encoded = error.to_string().to_lowercase();

    assert!(encoded.contains("unavailable"));
    assert!(!encoded.contains("api_key"));
}

#[tokio::test]
async fn hardened_provider_rejects_malformed_daemon_response() {
    let transport = InMemoryHardenedTransport::healthy().with_response(
        WasmHardenedProviderResponse {
            trace_id: String::new(),
            request_id: "wrong-request".into(),
            status: ApplicationHostCommandStatus::Ok,
            reason_code: "malformed".into(),
            metadata: Default::default(),
            diagnostics: vec!["raw_payload secret should stay out".into()],
        },
    );
    let provider = HardenedOutOfProcessWasmRuntimeProvider::new(Arc::new(transport));
    let session = provider
        .create_session(traced_request("trace-hardened-malformed"))
        .await
        .unwrap();
    let command = traced_command("trace-hardened-malformed-command");

    let result = session.dispatch(command).await.unwrap();
    let encoded = serde_json::to_string(&result).unwrap().to_lowercase();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("malformed_response")
    );
    assert!(!encoded.contains("secret should stay out"));
}

#[tokio::test]
async fn hardened_provider_preserves_trace_for_successful_daemon_dispatch() {
    let provider = HardenedOutOfProcessWasmRuntimeProvider::new(Arc::new(
        InMemoryHardenedTransport::healthy(),
    ));
    let session = provider
        .create_session(traced_request("trace-hardened-success-provider"))
        .await
        .unwrap();
    let command = traced_command("trace-hardened-success-command");

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.trace.as_ref().map(|trace| trace.trace_id.as_str()),
        Some("trace-hardened-success-command")
    );
    assert_eq!(
        result.metadata.get("provider_class").map(String::as_str),
        Some("hardened_out_of_process")
    );
}

#[tokio::test]
async fn hardened_provider_reports_backpressure_without_raw_payload() {
    let response = WasmHardenedProviderResponse {
        trace_id: "trace-hardened-backpressure-command".into(),
        request_id: "hardened-session-trace-hardened-backpressure-provider".into(),
        status: ApplicationHostCommandStatus::DisabledByPolicy {
            reason: "daemon overloaded".into(),
        },
        reason_code: "backpressure".into(),
        metadata: Default::default(),
        diagnostics: vec!["raw_payload secret should stay out".into()],
    };
    let provider = HardenedOutOfProcessWasmRuntimeProvider::new(Arc::new(
        InMemoryHardenedTransport::healthy().with_response(response),
    ));
    let session = provider
        .create_session(traced_request("trace-hardened-backpressure-provider"))
        .await
        .unwrap();
    let command = traced_command("trace-hardened-backpressure-command");

    let result = session.dispatch(command).await.unwrap();
    let encoded = serde_json::to_string(&result).unwrap().to_lowercase();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("backpressure")
    );
    assert!(!encoded.contains("secret should stay out"));
}

#[tokio::test]
async fn hardened_provider_reports_timeout_before_daemon_dispatch() {
    let provider = HardenedOutOfProcessWasmRuntimeProvider::new(Arc::new(
        InMemoryHardenedTransport::healthy(),
    ));
    let mut profile = WasmExecutionProfile::default_wasm_component();
    profile.resources.max_wall_time_ms = Some(0);
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-hardened-timeout-provider"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        profile,
    )
    .unwrap();
    let session = provider.create_session(request).await.unwrap();
    let command = traced_command("trace-hardened-timeout-command");

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("timeout")
    );
}

#[tokio::test]
async fn hardened_transport_health_exposes_safe_state() {
    let transport = InMemoryHardenedTransport::unavailable("daemon secret should stay out");
    let health = transport.health(TraceContext::new("trace-health")).await;

    assert!(matches!(health, WasmHardenedHealth::Unavailable { .. }));
    assert!(!format!("{health:?}").to_lowercase().contains("secret"));
}

fn traced_request(trace_id: &str) -> WasmRuntimeSessionRequest {
    WasmRuntimeSessionRequest::new(
        TraceContext::new(trace_id),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap()
}

fn traced_command(trace_id: &str) -> ApplicationHostCommand {
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({"input": true}),
        TraceContext::new(trace_id),
    );
    command
        .metadata
        .insert("wasm.operation".into(), "invoke".into());
    command
}
