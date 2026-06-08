//! Contract tests proving component-model WASM sessions route host imports through ServiceRuntime.

use std::sync::Arc;

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, KernelServiceId,
    ServiceDescriptor, ServiceType, TraceContext, TraceSchemaRef, WasmExecutionProfile,
    WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};
use serde_json::json;

use super::super::{
    ComponentModelWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use crate::{
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

use super::support::{
    component_fixture_bytes, component_fixture_bytes_with_duplicate_host_command,
    component_fixture_bytes_with_host_command, component_fixture_bytes_with_host_commands,
    register_mock_service, traced_request, write_fixture_component,
};

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
