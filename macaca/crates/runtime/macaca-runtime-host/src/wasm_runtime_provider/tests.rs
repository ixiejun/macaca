use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, TraceContext,
    WasmExecutionProfile, WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};
use serde_json::json;

use super::{
    DefaultInProcessWasmRuntimeProvider, UnavailableWasmRuntimeProvider,
    WasmApplicationRuntimeProvider,
};

#[tokio::test]
async fn unavailable_wasm_runtime_provider_rejects_session_without_trace() {
    let provider = UnavailableWasmRuntimeProvider::default();
    let request = WasmRuntimeSessionRequest {
        trace: None,
        application_id: "fixture.application".into(),
        ability_id: "main".into(),
        artifact: WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        profile: WasmExecutionProfile::default_wasm_component(),
        metadata: Default::default(),
    };

    let error = provider.create_session(request).await.unwrap_err();

    assert_eq!(
        error,
        macaca_proto::ApplicationAbiError::MissingTraceContext
    );
}

#[tokio::test]
async fn unavailable_wasm_runtime_provider_creates_unavailable_session() {
    let provider = UnavailableWasmRuntimeProvider::default();
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-wasm-provider"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap();

    let session = provider.create_session(request).await.unwrap();
    let command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({"raw_payload": "must_not_escape"}),
        TraceContext::new("trace-wasm-command"),
    );
    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("provider_missing")
    );
    assert!(!result
        .metadata
        .values()
        .any(|value| value.contains("must_not_escape")));
}

#[tokio::test]
async fn unavailable_wasm_runtime_provider_reports_sanitized_availability() {
    let provider = UnavailableWasmRuntimeProvider::new("raw payload contains API_KEY");
    let availability = provider.availability(None).await;

    assert!(!availability.is_available());
    let diagnostics = availability.diagnostics.unwrap();
    assert!(diagnostics.is_sanitized());
    assert_eq!(diagnostics.reason_code, "provider_missing");
}

#[tokio::test]
async fn wasm_default_runtime_provider_invokes_minimal_exported_function() {
    let artifact_path = write_fixture_wasm("minimal-export", minimal_start_module());
    let provider = DefaultInProcessWasmRuntimeProvider::default();
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-default-provider"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap();

    let session = provider.create_session(request).await.unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({}),
        TraceContext::new("trace-default-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:start".into());
    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["export"], json!("app:start"));
    assert_eq!(
        result.metadata.get("runtime_kind").map(String::as_str),
        Some("wasm_component")
    );
    assert!(result.metadata.contains_key("cache_state"));
    assert!(result.metadata.contains_key("session_id"));
}

#[tokio::test]
async fn wasm_default_runtime_provider_reports_trap_without_raw_payload() {
    let artifact_path = write_fixture_wasm("trapping-export", trapping_start_module());
    let provider = DefaultInProcessWasmRuntimeProvider::default();
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-default-trap-provider"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap();

    let session = provider.create_session(request).await.unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({"raw_payload": "secret should stay out"}),
        TraceContext::new("trace-default-trap-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:start".into());
    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("trap")
    );
    assert!(!result
        .metadata
        .values()
        .any(|value| value.contains("secret should stay out")));
}

#[tokio::test]
async fn wasm_sandbox_denies_oversized_payload_without_leaking_payload() {
    let artifact_path = write_fixture_wasm("sandbox-payload", minimal_start_module());
    let provider = DefaultInProcessWasmRuntimeProvider::default();
    let mut request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-sandbox-payload-provider"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap();
    request
        .metadata
        .insert("wasm.resource.max_payload_bytes".into(), "8".into());

    let session = provider.create_session(request).await.unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({"raw_payload": "secret should stay out"}),
        TraceContext::new("trace-sandbox-payload-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:start".into());
    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("resource_exhausted")
    );
    assert!(!result
        .metadata
        .values()
        .any(|value| value.contains("secret should stay out")));
}

#[tokio::test]
async fn wasm_sandbox_enforces_session_concurrency_per_quota_key() {
    let artifact_path = write_fixture_wasm("sandbox-concurrency", minimal_start_module());
    let provider = DefaultInProcessWasmRuntimeProvider::default();
    let mut first_request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-sandbox-concurrency-first"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap();
    first_request
        .metadata
        .insert("wasm.resource.max_concurrent_sessions".into(), "1".into());
    let mut second_request = first_request.clone();
    second_request.trace = Some(TraceContext::new("trace-sandbox-concurrency-second"));

    let first_session = provider.create_session(first_request).await.unwrap();
    let error = provider
        .create_session(second_request.clone())
        .await
        .unwrap_err();

    assert!(error.to_string().contains("concurrency limit"));
    drop(first_session);
    assert!(provider.create_session(second_request).await.is_ok());
}

#[tokio::test]
async fn wasm_sandbox_rejects_declared_memory_over_policy() {
    let artifact_path = write_fixture_wasm("sandbox-memory", memory_start_module());
    let provider = DefaultInProcessWasmRuntimeProvider::default();
    let mut profile = WasmExecutionProfile::default_wasm_component();
    profile.resources.max_memory_bytes = Some(64 * 1024);
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-sandbox-memory"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        profile,
    )
    .unwrap();

    let error = provider.create_session(request).await.unwrap_err();

    assert!(error.to_string().contains("memory"));
}

#[tokio::test]
async fn wasm_sandbox_rejects_estimated_fuel_over_policy() {
    let artifact_path = write_fixture_wasm("sandbox-fuel", minimal_start_module());
    let provider = DefaultInProcessWasmRuntimeProvider::default();
    let mut profile = WasmExecutionProfile::default_wasm_component();
    profile.resources.max_fuel = Some(0);
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-sandbox-fuel"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        profile,
    )
    .unwrap();

    let error = provider.create_session(request).await.unwrap_err();

    assert!(error.to_string().contains("fuel"));
}

#[tokio::test]
async fn wasm_sandbox_reports_wall_clock_timeout_without_invocation() {
    let artifact_path = write_fixture_wasm("sandbox-timeout", minimal_start_module());
    let provider = DefaultInProcessWasmRuntimeProvider::default();
    let mut profile = WasmExecutionProfile::default_wasm_component();
    profile.resources.max_wall_time_ms = Some(0);
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-sandbox-timeout-provider"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        profile,
    )
    .unwrap();

    let session = provider.create_session(request).await.unwrap();
    let result = session
        .dispatch(ApplicationHostCommand::with_trace(
            ApplicationImport::Custom("macaca:wasm/invoke".into()),
            json!({}),
            TraceContext::new("trace-sandbox-timeout-command"),
        ))
        .await
        .unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("timeout")
    );
}

#[test]
fn wasm_sandbox_descriptor_reports_raw_wasi_denied_by_default() {
    let provider = DefaultInProcessWasmRuntimeProvider::default();
    let descriptor = provider.descriptor();

    assert_eq!(
        descriptor
            .metadata
            .get("sandbox.raw_env")
            .map(String::as_str),
        Some("deny")
    );
    assert_eq!(
        descriptor
            .metadata
            .get("sandbox.raw_filesystem")
            .map(String::as_str),
        Some("deny")
    );
    assert_eq!(
        descriptor
            .metadata
            .get("sandbox.raw_network")
            .map(String::as_str),
        Some("deny")
    );
    assert_eq!(
        descriptor
            .metadata
            .get("sandbox.deny_raw_wasi")
            .map(String::as_str),
        Some("true")
    );
}

fn write_fixture_wasm(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let directory = tempfile::Builder::new()
        .prefix("macaca-wasm-runtime-provider-")
        .tempdir()
        .unwrap()
        .keep();
    let path = directory.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn minimal_start_module() -> &'static [u8] {
    &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x0d, 0x01, 0x09, b'a', b'p', b'p', b':', b's', b't', b'a', b'r',
        b't', 0x00, 0x00, 0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
    ]
}

fn trapping_start_module() -> &'static [u8] {
    &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x0d, 0x01, 0x09, b'a', b'p', b'p', b':', b's', b't', b'a', b'r',
        b't', 0x00, 0x00, 0x0a, 0x05, 0x01, 0x03, 0x00, 0x00, 0x0b,
    ]
}

fn memory_start_module() -> &'static [u8] {
    &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x05, 0x03, 0x01, 0x00, 0x02, 0x07, 0x0d, 0x01, 0x09, b'a', b'p', b'p',
        b':', b's', b't', b'a', b'r', b't', 0x00, 0x00, 0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
    ]
}
