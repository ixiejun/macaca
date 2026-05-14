//! Shared runtime composition bundle for service-call audit closure.
//!
//! This module centralizes host-level audit wiring so startup code can bind a
//! single `ServiceCallAuditSink` to both:
//! - the generic replay command surface (`system.service_audit`)
//! - WASM host-import bridge routing (`service.call` audit emission)
//!
//! The bundle is intentionally provider-neutral and contains no application or
//! workflow-specific logic.

use std::sync::Arc;

use macaca_kernel::SystemService;

use crate::wasm_runtime_provider::{WasmHostImportBridge, WasmHostImportBridgeConfig};
use crate::{
    service_call_audit_service_descriptor, InMemoryServiceCallAuditSink, ServiceCallAuditSink,
    ServiceCallAuditSystemServiceProvider, ServiceProviderInstance, ServiceRuntime,
};

/// Runtime bundle that owns one shared audit sink and exposes wiring helpers.
#[derive(Clone)]
pub struct ServiceAuditRuntimeBundle {
    sink: Arc<dyn ServiceCallAuditSink>,
}

impl ServiceAuditRuntimeBundle {
    /// Build a bundle with an in-memory shared sink.
    pub fn in_memory() -> Self {
        Self {
            sink: Arc::new(InMemoryServiceCallAuditSink::new()),
        }
    }

    /// Return the shared audit sink for advanced host composition cases.
    pub fn sink(&self) -> Arc<dyn ServiceCallAuditSink> {
        Arc::clone(&self.sink)
    }

    /// Build a provider instance for the generic system replay service.
    ///
    /// Hosts can register this instance through `ServiceRuntime` startup
    /// lifecycle and expose replay commands without depending on sink internals.
    pub fn audit_service_provider_instance(&self) -> ServiceProviderInstance {
        let service: Arc<dyn SystemService> = Arc::new(ServiceCallAuditSystemServiceProvider::new(
            Arc::clone(&self.sink),
        ));
        ServiceProviderInstance::new(service_call_audit_service_descriptor(), service)
    }

    /// Build a WASM host-import bridge bound to the same shared sink.
    ///
    /// When runtime hosts enable real WASM execution, this bridge guarantees
    /// audit replay queries and host-import routing observe one evidence chain.
    pub fn wasm_host_import_bridge(
        &self,
        runtime: Arc<ServiceRuntime>,
        config: WasmHostImportBridgeConfig,
    ) -> WasmHostImportBridge {
        WasmHostImportBridge::new_with_audit_sink(runtime, config, Arc::clone(&self.sink))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use macaca_proto::{ApplicationHostCommand, ApplicationImport, TraceContext};

    use crate::{
        service_call_audit_replay_trace_command, ServiceCallAuditReplayResult,
        ServiceCallAuditReplayTraceCommand, ServiceCallAuditSystemServiceProvider, ServiceRuntime,
        ServiceRuntimeConfig,
    };

    use super::*;

    #[tokio::test]
    async fn bundle_keeps_bridge_and_replay_provider_on_same_sink() {
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let bundle = ServiceAuditRuntimeBundle::in_memory();
        let bridge = bundle
            .wasm_host_import_bridge(Arc::clone(&runtime), WasmHostImportBridgeConfig::default());

        let mut command = ApplicationHostCommand::with_trace(
            ApplicationImport::ServiceCall,
            serde_json::json!({"hello": "world"}),
            TraceContext::new("trace-bundle-shared-sink"),
        );
        command.metadata = BTreeMap::from([
            ("service.id".into(), "service.audit.bundle.missing".into()),
            ("service.operation".into(), "call".into()),
            ("capability".into(), "capability.service.call".into()),
            ("session.id".into(), "session-bundle-shared-sink".into()),
            ("app.id".into(), "app.bundle.audit".into()),
        ]);
        let _ = bridge
            .dispatch(command, TraceContext::new("trace-bundle-shared-sink"))
            .await;

        let provider = ServiceCallAuditSystemServiceProvider::new(bundle.sink());
        let result = provider
            .call(
                service_call_audit_replay_trace_command(ServiceCallAuditReplayTraceCommand {
                    trace: TraceContext::new("trace-query-bundle"),
                    trace_id: "trace-bundle-shared-sink".into(),
                })
                .expect("trace replay command should build"),
            )
            .await
            .expect("replay command should succeed");
        let view: ServiceCallAuditReplayResult =
            serde_json::from_value(result.output).expect("replay output should decode");
        assert!(
            !view.events.is_empty(),
            "bridge route events should be replayable from the same shared sink"
        );
    }
}
