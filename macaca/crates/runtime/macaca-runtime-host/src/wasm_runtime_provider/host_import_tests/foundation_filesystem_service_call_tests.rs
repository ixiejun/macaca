//! WASM routing proof for foundation filesystem commands through `ServiceRuntime`.
//!
//! A concrete provider is installed exclusively in the runtime-host composition
//! root. The WASM-facing host import receives only a declared command, a logical
//! path payload, and a capability scope; it cannot access provider handles or
//! local host paths directly.

use std::sync::Arc;

use macaca_foundation_filesystem::MockFilesystemProvider;
use macaca_kernel::SystemService;
use macaca_proto::ApplicationHostCommandStatus;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use super::support::{host_import_command, traced_request};
use crate::foundation_filesystem_service_provider::FoundationFilesystemSystemServiceProvider;
use crate::{
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn wasm_filesystem_read_uses_service_runtime_and_preserves_trace_metadata() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(
        FoundationFilesystemSystemServiceProvider::new(Arc::new(MockFilesystemProvider::default())),
    );
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            macaca_proto::TraceContext::new("trace-filesystem-wasm-start"),
        )
        .await
        .unwrap();

    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-filesystem-wasm-provider"))
        .await
        .unwrap();
    let result = session
        .dispatch(host_import_command(
            "trace-filesystem-wasm-command",
            &service_id,
            "filesystem.read_file",
            serde_json::json!({"path":{"relative_path":"document.txt"}}),
            "filesystem.read",
        ))
        .await
        .unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some("service.foundation.filesystem")
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("filesystem.read_file")
    );
    assert!(!format!("{result:?}").contains("/private/host"));
}

#[tokio::test]
async fn wasm_routes_every_declared_filesystem_command_through_the_service_descriptor() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(
        FoundationFilesystemSystemServiceProvider::new(Arc::new(MockFilesystemProvider::default())),
    );
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            macaca_proto::TraceContext::new("trace-filesystem-wasm-all-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-filesystem-wasm-all-provider"))
        .await
        .unwrap();
    for operation in macaca_proto::FOUNDATION_FILESYSTEM_COMMANDS {
        let result = session
            .dispatch(host_import_command(
                &format!("trace-filesystem-wasm-{operation}"),
                &service_id,
                operation,
                filesystem_payload(operation),
                "filesystem.read",
            ))
            .await
            .unwrap();
        assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
        assert_eq!(
            result.metadata.get("service.operation").map(String::as_str),
            Some(*operation)
        );
    }
}

fn filesystem_payload(operation: &str) -> serde_json::Value {
    match operation {
        "filesystem.open_handle"
        | "filesystem.read_file"
        | "filesystem.stat_path"
        | "filesystem.list_directory"
        | "filesystem.create_directory"
        | "filesystem.delete_path"
        | "filesystem.watch_path" => serde_json::json!({"path":{"relative_path":"document.txt"}}),
        "filesystem.write_file" | "filesystem.append_file" => {
            serde_json::json!({"path":{"relative_path":"document.txt"},"content":{"content_ref":"artifact:test"}})
        }
        _ => serde_json::json!({}),
    }
}
