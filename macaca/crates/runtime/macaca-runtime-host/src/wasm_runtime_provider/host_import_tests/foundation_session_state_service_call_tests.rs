//! WASM routing proof for foundation session-state commands through `ServiceRuntime`.
//!
//! The guest supplies only a declared service operation, opaque payload, trace,
//! and capability label. Provider construction remains in runtime-host
//! composition, so WASM code cannot obtain a persistence client or provider.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::ApplicationHostCommandStatus;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use super::support::{host_import_command, traced_request};
use crate::foundation_session_state_service_provider::FoundationSessionStateSystemServiceProvider;
use crate::{
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn wasm_routes_every_declared_session_state_command_through_service_runtime() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> =
        Arc::new(FoundationSessionStateSystemServiceProvider::mock());
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
            macaca_proto::TraceContext::new("trace-session-state-wasm-start"),
        )
        .await
        .unwrap();

    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-session-state-wasm-provider"))
        .await
        .unwrap();

    for operation in macaca_proto::FOUNDATION_SESSION_STATE_COMMANDS {
        let result = session
            .dispatch(host_import_command(
                &format!("trace-session-state-wasm-{operation}"),
                &service_id,
                operation,
                serde_json::json!({"raw_state":"private-marker","provider_payload":"raw"}),
                "session_state.read",
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
