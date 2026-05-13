use std::sync::Arc;

use macaca_kernel::MockSystemService;
use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, KernelServiceId,
    ServiceDescriptor, ServiceType, TraceContext, TraceSchemaRef, WasmExecutionProfile,
    WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};
use serde_json::json;

use super::{
    ComponentModelWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use crate::{
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

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

#[tokio::test]
async fn component_model_provider_routes_host_imports_through_service_portal() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.component.service.allowed").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let artifact_path = write_fixture_component("host-import", component_fixture_bytes());
    let provider = ComponentModelWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request(
            "trace-component-host-import-provider",
            &artifact_path,
        ))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({"input": true}),
        TraceContext::new("trace-component-host-import-command"),
    );
    command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    command
        .metadata
        .insert("service.operation".into(), "invoke".into());
    command
        .metadata
        .insert("capability".into(), "service.call".into());

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["input"], json!(true));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("import_completed")
    );
    assert_eq!(
        result.metadata.get("provider_class").map(String::as_str),
        Some("component_model")
    );
}

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

async fn register_mock_service(runtime: &ServiceRuntime, service_id: &str) -> KernelServiceId {
    let descriptor = ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new("test.service"),
        TraceSchemaRef::new("trace.test.service.v1"),
    );
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                descriptor.clone(),
                Arc::new(MockSystemService::new(descriptor)),
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            TraceContext::new("trace-component-service-start"),
        )
        .await
        .unwrap();
    service_id
}

fn traced_request(trace_id: &str, artifact_path: &std::path::Path) -> WasmRuntimeSessionRequest {
    WasmRuntimeSessionRequest::new(
        TraceContext::new(trace_id),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap()
}

fn write_fixture_component(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let directory = tempfile::Builder::new()
        .prefix("macaca-wasm-component-provider-")
        .tempdir()
        .unwrap()
        .keep();
    let path = directory.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn component_fixture_bytes() -> &'static [u8] {
    b"\0asm\x01\0\0\0macaca:component-model:v1\0export=app:start\0wit=macaca:application/runtime@1"
}
