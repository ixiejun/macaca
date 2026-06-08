//! Contract tests for declared host-command plans and payload placeholder resolution.

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
async fn component_model_provider_dispatches_declared_host_command_plan() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.component.service.plan").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let mut host_command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({"input": "${chat.input}"}),
        TraceContext::new("trace-placeholder-replaced-by-runtime"),
    );
    host_command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    host_command
        .metadata
        .insert("service.operation".into(), "invoke".into());
    host_command
        .metadata
        .insert("capability".into(), "service.call".into());
    let fixture = component_fixture_bytes_with_host_command(&host_command);
    let artifact_path = write_fixture_component("declared-host-plan", &fixture);
    let provider = ComponentModelWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request(
            "trace-component-declared-plan-provider",
            &artifact_path,
        ))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({"input": "AAPL"}),
        TraceContext::new("trace-component-declared-plan-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:start".into());

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["export"], json!("app:start"));
    assert_eq!(
        result.output["host_command_results"][0]["status"],
        json!("Ok")
    );
    assert_eq!(
        result.output["host_command_results"][0]["output"]["input"],
        json!("AAPL")
    );
}

#[tokio::test]
async fn component_model_provider_resolves_typed_chat_payload_fields() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.component.service.typed-chat").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let mut host_command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({
            "input": "${chat.input}",
            "symbol": "${chat.symbol}"
        }),
        TraceContext::new("trace-placeholder-replaced-by-runtime"),
    );
    host_command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    host_command
        .metadata
        .insert("service.operation".into(), "invoke".into());
    host_command
        .metadata
        .insert("capability".into(), "service.call".into());
    let fixture = component_fixture_bytes_with_host_command(&host_command);
    let artifact_path = write_fixture_component("declared-host-plan-typed-chat", &fixture);
    let provider = ComponentModelWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request(
            "trace-component-declared-plan-typed-chat-provider",
            &artifact_path,
        ))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({
            "input": "Analyze BTC/USDT now",
            "symbol": "BTC"
        }),
        TraceContext::new("trace-component-declared-plan-typed-chat-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:start".into());

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.output["host_command_results"][0]["output"]["input"],
        json!("Analyze BTC/USDT now")
    );
    assert_eq!(
        result.output["host_command_results"][0]["output"]["symbol"],
        json!("BTC")
    );
}

#[tokio::test]
async fn component_model_provider_resolves_nested_chat_payload_fields() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.component.service.nested-chat").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let mut host_command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({
            "input": "${chat.input}",
            "run_id": "${chat.run_id}",
            "workspace_ref": "${chat.workspace_ref}"
        }),
        TraceContext::new("trace-nested-placeholder-replaced-by-runtime"),
    );
    host_command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    host_command
        .metadata
        .insert("service.operation".into(), "invoke".into());
    host_command
        .metadata
        .insert("capability".into(), "service.call".into());
    let fixture = component_fixture_bytes_with_host_command(&host_command);
    let artifact_path = write_fixture_component("declared-host-plan-nested-chat", &fixture);
    let provider = ComponentModelWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request(
            "trace-component-declared-plan-nested-chat-provider",
            &artifact_path,
        ))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({
            "chat": {
                "input": "Create a Rust hello world project",
                "run_id": "run-nested-chat",
                "workspace_ref": "workspace://demo/shared"
            }
        }),
        TraceContext::new("trace-component-declared-plan-nested-chat-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:start".into());

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.output["host_command_results"][0]["output"]["input"],
        json!("Create a Rust hello world project")
    );
    assert_eq!(
        result.output["host_command_results"][0]["output"]["run_id"],
        json!("run-nested-chat")
    );
    assert_eq!(
        result.output["host_command_results"][0]["output"]["workspace_ref"],
        json!("workspace://demo/shared")
    );
}

#[tokio::test]
async fn component_model_provider_interpolates_embedded_display_placeholders() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.component.service.display-chat").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let mut host_command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({
            "title": "${chat.symbol} Signal Analysis",
            "native_symbol": "${chat.symbol}"
        }),
        TraceContext::new("trace-display-placeholder-replaced-by-runtime"),
    );
    host_command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    host_command
        .metadata
        .insert("service.operation".into(), "invoke".into());
    host_command
        .metadata
        .insert("capability".into(), "service.call".into());
    let fixture = component_fixture_bytes_with_host_command(&host_command);
    let artifact_path = write_fixture_component("declared-host-plan-display-chat", &fixture);
    let provider = ComponentModelWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request(
            "trace-component-declared-plan-display-chat-provider",
            &artifact_path,
        ))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({
            "input": "Analyze ETH/USDT now",
            "symbol": "ETH"
        }),
        TraceContext::new("trace-component-declared-plan-display-chat-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:start".into());

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.output["host_command_results"][0]["output"]["title"],
        json!("ETH Signal Analysis")
    );
    assert_eq!(
        result.output["host_command_results"][0]["output"]["native_symbol"],
        json!("ETH")
    );
}
