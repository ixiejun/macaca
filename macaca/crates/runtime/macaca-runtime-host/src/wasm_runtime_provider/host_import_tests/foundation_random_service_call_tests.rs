//! WASM routing proof for foundation-random through the generic service boundary.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::ApplicationHostCommandStatus;
use macaca_random::HostRandomProvider;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use super::support::{host_import_command, traced_request};
use crate::random_service_provider::RandomSystemServiceProvider;
use crate::{
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn wasm_random_command_uses_service_runtime_and_preserves_trace() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(RandomSystemServiceProvider::new(Arc::new(
        HostRandomProvider,
    )));
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
            macaca_proto::TraceContext::new("trace-random-wasm-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-random-wasm-provider"))
        .await
        .unwrap();
    let result = session
        .dispatch(host_import_command(
            "trace-random-wasm-command",
            &service_id,
            "random.bytes",
            serde_json::json!({"length":16}),
            "random.generate",
        ))
        .await
        .unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["status"], "success");
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some("service.foundation.random")
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("random.bytes")
    );
}
