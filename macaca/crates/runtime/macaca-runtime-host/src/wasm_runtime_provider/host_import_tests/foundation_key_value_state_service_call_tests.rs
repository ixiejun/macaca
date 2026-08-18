//! WASM routing proof for key-value state commands through `ServiceRuntime`.
//!
//! The generic host import sees only a declared service command and scope. A KV
//! provider exists solely in runtime-host composition, so a guest cannot obtain
//! a database client, namespace backend, raw value channel, or provider handle.

use std::sync::Arc;

use macaca_foundation_key_value_state::MockKeyValueStateProvider;
use macaca_kernel::SystemService;
use macaca_proto::ApplicationHostCommandStatus;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use super::support::{host_import_command, traced_request};
use crate::foundation_key_value_state_service_provider::FoundationKeyValueStateSystemServiceProvider;
use crate::{
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn wasm_routes_every_declared_key_value_command_through_service_runtime() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> =
        Arc::new(FoundationKeyValueStateSystemServiceProvider::new(Arc::new(
            MockKeyValueStateProvider::default(),
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
            macaca_proto::TraceContext::new("trace-key-value-wasm-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-key-value-wasm-provider"))
        .await
        .unwrap();
    for operation in macaca_proto::FOUNDATION_KEY_VALUE_STATE_COMMANDS {
        let result = session
            .dispatch(host_import_command(
                &format!("trace-key-value-wasm-{operation}"),
                &service_id,
                operation,
                serde_json::json!({"namespace":"preferences","value":"private-marker"}),
                "state.read",
            ))
            .await
            .unwrap();
        assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
        assert_eq!(
            result.metadata.get("service.operation").map(String::as_str),
            Some(*operation)
        );
        assert!(!format!("{result:?}").contains("private-marker"));
    }
}
