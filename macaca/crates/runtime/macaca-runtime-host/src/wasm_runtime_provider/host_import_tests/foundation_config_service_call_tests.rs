//! WASM routing proof for foundation-config through the generic service boundary.
//!
//! The test installs a deterministic provider only in the runtime composition root,
//! then invokes the normal host import. It proves applications receive no provider
//! handle and that trace evidence is preserved by `ServiceRuntime`.

use std::sync::Arc;

use macaca_foundation_config::MockConfigProvider;
use macaca_kernel::SystemService;
use macaca_proto::ApplicationHostCommandStatus;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use super::support::{host_import_command, traced_request};
use crate::foundation_config_service_provider::FoundationConfigSystemServiceProvider;
use crate::{
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn wasm_config_command_uses_service_runtime_and_preserves_trace() {
    // Composition owns the concrete provider. The application only supplies a declared
    // command, permission scope, and bounded request payload to the generic host bridge.
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let config = Arc::new(MockConfigProvider::default());
    config
        .insert_reference("runtime.setting", "artifact:config-reference")
        .unwrap();
    let provider: Arc<dyn SystemService> =
        Arc::new(FoundationConfigSystemServiceProvider::new(config));
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
            macaca_proto::TraceContext::new("trace-config-wasm-start"),
        )
        .await
        .unwrap();

    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-config-wasm-provider"))
        .await
        .unwrap();
    let result = session
        .dispatch(host_import_command(
            "trace-config-wasm-command",
            &service_id,
            "config.get",
            serde_json::json!({"key":{"namespace":"runtime","key":"runtime.setting"}}),
            "config.read",
        ))
        .await
        .unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(result.output["value_ref"], "artifact:config-reference");
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some("service.foundation.config")
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("config.get")
    );
}
