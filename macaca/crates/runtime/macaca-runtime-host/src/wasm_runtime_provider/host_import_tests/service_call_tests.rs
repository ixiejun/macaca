//! Contract tests proving generic `service.call` host imports route through ServiceRuntime.

use std::sync::Arc;

use macaca_proto::{ApplicationHostCommandStatus, MCP_SERVICE_ID, MCP_TOOL_INVOKE_COMMAND};
use serde_json::json;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use crate::{ServiceRuntime, ServiceRuntimeConfig};

use super::support::{host_import_command, register_mock_service, traced_request};

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
async fn wasm_host_import_mcp_tool_invoke_uses_generic_service_call_path() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, MCP_SERVICE_ID).await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-host-import-mcp-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-host-import-mcp-command",
        &service_id,
        MCP_TOOL_INVOKE_COMMAND,
        json!({
            "server_id": "server-a",
            "backend_tool_name": "lookup",
            "visible_tool_name": "mcp_lookup"
        }),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["server_id"], json!("server-a"));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(MCP_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some(MCP_TOOL_INVOKE_COMMAND)
    );
}
