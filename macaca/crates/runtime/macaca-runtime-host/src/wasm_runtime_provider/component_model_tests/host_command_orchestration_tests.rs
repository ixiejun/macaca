//! Contract tests for declared host-command deduplication and result chaining.

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
async fn component_model_provider_deduplicates_repeated_declared_host_commands() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.component.service.dedup").await;
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
    let fixture = component_fixture_bytes_with_duplicate_host_command(&host_command);
    let artifact_path = write_fixture_component("declared-host-plan-dedup", &fixture);
    let provider = ComponentModelWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request(
            "trace-component-declared-plan-dedup-provider",
            &artifact_path,
        ))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({"input": "AAPL"}),
        TraceContext::new("trace-component-declared-plan-dedup-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:start".into());

    let result = session.dispatch(command).await.unwrap();

    // The fixture encodes the same host-command twice to mirror release WASM
    // binaries that retain metadata in multiple read-only segments.  The
    // portable adapter must execute the declared service call once so audit
    // counts match the app contract rather than linker layout details.
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.output["host_command_results"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        result.output["host_command_results"][0]["output"]["input"],
        json!("AAPL")
    );
}

#[tokio::test]
async fn component_model_provider_chains_declared_host_command_results() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "wasm.component.service.chain").await;
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let mut first_command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({"symbol": "${chat.input}", "price": 100}),
        TraceContext::new("trace-placeholder-replaced-by-runtime"),
    );
    first_command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    first_command
        .metadata
        .insert("service.operation".into(), "lookup".into());
    first_command
        .metadata
        .insert("capability".into(), "service.call".into());
    let mut second_command = ApplicationHostCommand::with_trace(
        ApplicationImport::ServiceCall,
        json!({
            "market_data": "${host.results.0.output}",
            "market_symbol": "${host.results.0.output.symbol}",
            "missing_reference": "${host.results.9.output}"
        }),
        TraceContext::new("trace-placeholder-replaced-by-runtime"),
    );
    second_command
        .metadata
        .insert("service.id".into(), service_id.to_string());
    second_command
        .metadata
        .insert("service.operation".into(), "analyze".into());
    second_command
        .metadata
        .insert("capability".into(), "service.call".into());
    let fixture = component_fixture_bytes_with_host_commands(&[first_command, second_command]);
    let artifact_path = write_fixture_component("declared-host-plan-chain", &fixture);
    let provider = ComponentModelWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request(
            "trace-component-declared-plan-chain-provider",
            &artifact_path,
        ))
        .await
        .unwrap();
    let mut command = ApplicationHostCommand::with_trace(
        ApplicationImport::Custom("macaca:wasm/invoke".into()),
        json!({"input": "BTC"}),
        TraceContext::new("trace-component-declared-plan-chain-command"),
    );
    command
        .metadata
        .insert("wasm.export".into(), "app:start".into());

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.output["host_command_results"][1]["output"]["market_data"]["symbol"],
        json!("BTC")
    );
    assert_eq!(
        result.output["host_command_results"][1]["output"]["market_symbol"],
        json!("BTC")
    );
    assert_eq!(
        result.output["host_command_results"][1]["output"]["missing_reference"],
        json!("${host.results.9.output}")
    );
}
