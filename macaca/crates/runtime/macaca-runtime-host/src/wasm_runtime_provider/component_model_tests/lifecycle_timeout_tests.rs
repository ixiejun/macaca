//! Contract tests for component-model session lifecycle and wall-time timeout handling.

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, TraceContext,
    WasmExecutionProfile, WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};
use serde_json::json;

use super::super::{ComponentModelWasmRuntimeProvider, WasmApplicationRuntimeProvider};

use super::support::{component_fixture_bytes, write_fixture_component};

#[tokio::test]
async fn component_model_provider_reports_timeout_without_invoking_export() {
    let artifact_path = write_fixture_component("timeout", component_fixture_bytes());
    let provider = ComponentModelWasmRuntimeProvider::default();
    let mut profile = WasmExecutionProfile::default_wasm_component();
    profile.resources.max_wall_time_ms = Some(0);
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-component-timeout-provider"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        profile,
    )
    .unwrap();
    let session = provider.create_session(request).await.unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({}),
        TraceContext::new("trace-component-timeout-command"),
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
        Some("timeout")
    );
}
