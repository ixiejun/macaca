//! Contract tests for app-scoped policy overrides and service-result sanitization.

use std::sync::Arc;

use macaca_proto::ApplicationHostCommandStatus;
use serde_json::json;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use crate::{ServiceRuntime, ServiceRuntimeConfig};

use super::support::{host_import_command, register_mock_service, traced_request};

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
