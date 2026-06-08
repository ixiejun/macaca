//! Contract tests for component-model provider descriptor and admission guards.

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, TraceContext,
    WasmExecutionProfile, WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};
use serde_json::json;

use super::super::{ComponentModelWasmRuntimeProvider, WasmApplicationRuntimeProvider};

use super::support::{component_fixture_bytes, traced_request, write_fixture_component};

#[test]
fn component_model_provider_descriptor_reports_component_capability() {
    let provider = ComponentModelWasmRuntimeProvider::default();
    let descriptor = provider.descriptor();

    assert_eq!(descriptor.provider_class, "component_model");
    assert!(descriptor.capabilities.can_compile);
    assert!(descriptor.capabilities.can_instantiate);
    assert!(descriptor.capabilities.can_execute);
    assert!(descriptor.capabilities.supports_component_model);
    assert_eq!(
        descriptor
            .metadata
            .get("governance.owner")
            .map(String::as_str),
        Some("runtime-host")
    );
}

#[tokio::test]
async fn component_model_provider_requires_trace_context() {
    let artifact_path = write_fixture_component("missing-trace", component_fixture_bytes());
    let provider = ComponentModelWasmRuntimeProvider::default();
    let request = WasmRuntimeSessionRequest {
        trace: None,
        application_id: "fixture.application".into(),
        ability_id: "main".into(),
        artifact: WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
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
async fn component_model_provider_rejects_invalid_component_without_raw_bytes() {
    let artifact_path = write_fixture_component(
        "invalid-component",
        b"not a component with API_KEY and raw payload",
    );
    let provider = ComponentModelWasmRuntimeProvider::default();
    let request = traced_request("trace-component-invalid", &artifact_path);

    let error = provider.create_session(request).await.unwrap_err();
    let encoded = error.to_string().to_lowercase();

    assert!(encoded.contains("component"));
    assert!(!encoded.contains("api_key"));
    assert!(!encoded.contains("raw payload"));
}

#[tokio::test]
async fn component_model_provider_rejects_missing_export_without_raw_payload() {
    let artifact_path = write_fixture_component("missing-export", component_fixture_bytes());
    let provider = ComponentModelWasmRuntimeProvider::default();
    let session = provider
        .create_session(traced_request(
            "trace-component-missing-export",
            &artifact_path,
        ))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({"raw_payload": "secret should stay out"}),
        TraceContext::new("trace-component-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:missing".into());

    let result = session.dispatch(command).await.unwrap();
    let encoded = serde_json::to_string(&result).unwrap().to_lowercase();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("invoke_failed")
    );
    assert!(!encoded.contains("secret should stay out"));
}
