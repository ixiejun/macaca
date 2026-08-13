//! WASM contract proof for foundation-time through the generic service.call import.

use std::sync::Arc;

use macaca_proto::ApplicationHostCommandStatus;
use macaca_time::FrozenTimeProvider;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use crate::time_service_provider::TimeSystemServiceProvider;
use crate::{
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};
use macaca_kernel::SystemService;

use super::support::{host_import_command, traced_request};

#[tokio::test]
async fn wasm_time_command_uses_service_runtime_and_preserves_trace() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(TimeSystemServiceProvider::new(Arc::new(
        FrozenTimeProvider::new(42),
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
            macaca_proto::TraceContext::new("trace-time-wasm-start"),
        )
        .await
        .unwrap();

    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-time-wasm-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-time-wasm-command",
        &service_id,
        "time.now",
        serde_json::json!({}),
        "time.read",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["epoch_millis"], 42);
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some("service.foundation.time")
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("time.now")
    );
}
