//! Contract tests for trace, capability, payload, and service admission guards.

use std::sync::Arc;

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, KernelServiceId,
};
use serde_json::json;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use crate::{ServiceRuntime, ServiceRuntimeConfig};

use super::support::{
    host_import_command, register_mock_service, register_mock_service_with_failure, traced_request,
};

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
async fn wasm_host_import_service_failure_is_structured_unavailable_not_policy_denied() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id =
        register_mock_service_with_failure(&runtime, "wasm.host.service.failing", true).await;
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
        "trace-host-import-service-failure",
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
        Some("service_failed")
    );
}
