use std::sync::Arc;

use macaca_kernel::MockSystemService;
use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, KernelServiceId,
    ServiceCommandName, ServiceDescriptor, ServiceHealth, ServiceLifecycleState, ServiceType,
    TraceContext, TraceSchemaRef, WasmExecutionProfile, WasmRuntimeArtifactRef,
    WasmRuntimeSessionRequest,
};
use serde_json::json;

use super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use crate::{
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn wasm_host_import_service_call_routes_through_service_runtime() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.allowed").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-command",
        &service_id,
        "invoke",
        json!({"input": true}),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["input"], json!(true));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("import_completed")
    );
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(service_id.as_str())
    );
}

#[tokio::test]
async fn wasm_host_import_bridge_replays_service_call_audit_chain() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.audit.replay").await;
    let bridge =
        WasmHostImportBridge::new(Arc::clone(&runtime), WasmHostImportBridgeConfig::default());
    let mut command = host_import_command(
        "trace-host-import-audit",
        &service_id,
        "invoke",
        json!({"input": true}),
        "service.call",
    );
    command
        .metadata
        .insert("session.id".into(), "session-host-import-audit".into());

    let result = bridge
        .dispatch(command, TraceContext::new("trace-host-import-audit"))
        .await;
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));

    let replay = bridge
        .replay_service_call_audit_by_trace_id("trace-host-import-audit")
        .unwrap();
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_requested"));
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_succeeded"));
    let session_replay = bridge
        .replay_service_call_audit_by_session_id("session-host-import-audit")
        .unwrap();
    assert!(!session_replay.is_empty());
}

#[tokio::test]
async fn wasm_host_import_missing_trace_is_denied_before_service_runtime() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.trace").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::without_trace(
        ApplicationImport::ServiceCall,
        json!({"input": true}),
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

    let error = session.dispatch(command).await.unwrap_err();

    assert_eq!(
        error,
        macaca_proto::ApplicationAbiError::MissingTraceContext
    );
}

#[tokio::test]
async fn wasm_host_import_missing_capability_is_denied() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.capability").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-capability",
        &service_id,
        "invoke",
        json!({"input": true}),
        "",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("capability_missing")
    );
}

#[tokio::test]
async fn wasm_host_import_oversized_payload_is_denied_before_service_runtime() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.payload").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig {
            max_payload_bytes: 8,
            ..WasmHostImportBridgeConfig::default()
        },
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-payload",
        &service_id,
        "invoke",
        json!({"raw_payload": "secret should stay out"}),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();
    let encoded = serde_json::to_string(&result).unwrap().to_lowercase();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("payload_too_large")
    );
    assert!(!encoded.contains("secret should stay out"));
}

#[tokio::test]
async fn wasm_host_import_unknown_service_is_structured_unavailable() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let service_id = KernelServiceId::new("wasm.host.service.missing");
    let command = host_import_command(
        "trace-host-import-missing",
        &service_id,
        "invoke",
        json!({"input": true}),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::Unavailable { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("service_unavailable")
    );
}

#[tokio::test]
async fn wasm_host_import_applies_app_scoped_policy_override_and_denies_service() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.policy.denied").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let mut command = host_import_command(
        "trace-host-import-policy-override",
        &service_id,
        "invoke",
        json!({"input": true}),
        "service.call",
    );
    command
        .metadata
        .insert("app.id".into(), "app-policy-a".into());
    command
        .metadata
        .insert("policy.deny_services".into(), service_id.to_string());

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(
        result.status,
        ApplicationHostCommandStatus::DisabledByPolicy { .. }
    ));
    assert_eq!(
        result.metadata.get("reason_code").map(String::as_str),
        Some("policy_denied")
    );
}

#[tokio::test]
async fn wasm_host_import_sanitizes_service_result_metadata() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.host.service.sanitize").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-sanitize",
        &service_id,
        "invoke",
        json!({"raw_prompt": "secret prompt must not escape"}),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();
    let encoded = serde_json::to_string(&result).unwrap().to_lowercase();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert!(!encoded.contains("secret prompt must not escape"));
    assert!(!encoded.contains("raw_prompt"));
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
        .start(&service_id, TraceContext::new("trace-host-import-start"))
        .await
        .unwrap();
    let snapshot = runtime.snapshot().unwrap();
    assert_eq!(
        snapshot.services[0].lifecycle_state,
        ServiceLifecycleState::Running
    );
    assert_eq!(snapshot.services[0].health, ServiceHealth::Healthy);
    service_id
}

fn traced_request(trace_id: &str) -> WasmRuntimeSessionRequest {
    let artifact_path = write_fixture_wasm("host-import", minimal_start_module());
    WasmRuntimeSessionRequest::new(
        TraceContext::new(trace_id),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap()
}

fn host_import_command(
    trace_id: &str,
    service_id: &KernelServiceId,
    operation: &str,
    payload: serde_json::Value,
    capability: &str,
) -> ApplicationHostCommand {
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        payload,
        TraceContext::new(trace_id),
    );
    command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    command.metadata.insert(
        "service.operation".into(),
        ServiceCommandName::new(operation).to_string(),
    );
    if !capability.is_empty() {
        command
            .metadata
            .insert("capability".into(), capability.to_string());
    }
    command
}

fn write_fixture_wasm(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let directory = tempfile::Builder::new()
        .prefix("macaca-wasm-host-import-")
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
