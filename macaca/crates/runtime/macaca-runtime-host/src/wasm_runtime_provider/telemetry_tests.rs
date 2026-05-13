use std::sync::Arc;

use macaca_proto::{
    ApplicationHostCommand, ApplicationImport, TraceContext, WasmExecutionProfile,
    WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};
use serde_json::json;

use super::{
    InMemoryWasmTelemetrySink, UnavailableWasmRuntimeProvider, WasmApplicationRuntimeProvider,
    WasmCertificationFixtureSet, WasmCertificationHarness, WasmCertificationProfile,
    WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};

#[test]
fn wasm_telemetry_event_redacts_sensitive_markers() {
    let event = WasmTelemetryEvent::new(
        WasmTelemetryStage::Invoke,
        "raw_payload secret API_KEY private key",
        "provider",
    )
    .metadata("raw_manifest", "prompt env=VALUE");

    assert!(event.is_sanitized());
    assert!(!format!("{event:?}")
        .to_ascii_lowercase()
        .contains("api_key"));
}

#[test]
fn wasm_telemetry_stage_labels_cover_industrial_paths() {
    let stages = [
        WasmTelemetryStage::Admission,
        WasmTelemetryStage::Compile,
        WasmTelemetryStage::Instantiate,
        WasmTelemetryStage::Invoke,
        WasmTelemetryStage::Resource,
        WasmTelemetryStage::HostImport,
        WasmTelemetryStage::Lifecycle,
        WasmTelemetryStage::Daemon,
        WasmTelemetryStage::Certification,
        WasmTelemetryStage::SupplyChain,
    ];

    assert_eq!(stages[0].as_str(), "admission");
    assert_eq!(stages[9].as_str(), "supply_chain");
}

#[tokio::test]
async fn wasm_telemetry_unavailable_provider_emits_availability_session_and_invoke_events() {
    let sink = Arc::new(InMemoryWasmTelemetrySink::default());
    let provider_sink: WasmTelemetrySinkRef = sink.clone();
    let provider = UnavailableWasmRuntimeProvider::default().with_telemetry_sink(provider_sink);

    provider
        .availability(Some(TraceContext::new("trace-telemetry-availability")))
        .await;
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-telemetry-session"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap();
    let session = provider.create_session(request).await.unwrap();
    let command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({"raw_payload": "secret should not be recorded"}),
        TraceContext::new("trace-telemetry-command"),
    );

    let _ = session.dispatch(command).await.unwrap();
    let events = sink.events();

    assert!(events
        .iter()
        .any(|event| event.stage == WasmTelemetryStage::Availability));
    assert!(events
        .iter()
        .any(|event| event.stage == WasmTelemetryStage::Session));
    assert!(events
        .iter()
        .any(|event| event.stage == WasmTelemetryStage::Invoke));
    assert!(events.iter().all(WasmTelemetryEvent::is_sanitized));
}

#[test]
fn wasm_telemetry_certification_harness_emits_profile_events() {
    let sink = Arc::new(InMemoryWasmTelemetrySink::default());
    let harness_sink: WasmTelemetrySinkRef = sink.clone();
    let harness =
        WasmCertificationHarness::new("fixture.telemetry").with_telemetry_sink(harness_sink);

    let report = harness.certify_profile(
        WasmCertificationProfile::Hardened,
        WasmCertificationFixtureSet::security_negative_matrix("fixture.telemetry"),
    );
    let events = sink.events();

    assert!(!report.industrial_ready);
    assert!(events.iter().any(
        |event| event.stage == WasmTelemetryStage::Certification && event.outcome == "started"
    ));
    assert!(events.iter().any(
        |event| event.stage == WasmTelemetryStage::Certification && event.outcome == "rejected"
    ));
    assert!(events.iter().all(WasmTelemetryEvent::is_sanitized));
}
