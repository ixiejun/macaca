//! Bridge construction, fluent configuration, and audit replay accessors.
//!
//! Construction follows the Builder/Fluent Interface pattern at the host
//! boundary: callers inject shared runtime dependencies once, then optionally
//! attach telemetry sinks, GenUI repositories, or external audit sinks without
//! changing guest-visible ABI contracts.

use std::sync::Arc;

use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServiceCallAuditEvent, ServiceCallAuditSink, ServicePolicyEngine, ServiceRouter,
    ServiceRuntime, ServiceRuntimeError,
};

use super::super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use super::{WasmHostImportBridge, WasmHostImportBridgeConfig};

impl WasmHostImportBridge {
    /// Create a bridge over an existing host-owned `ServiceRuntime`.
    ///
    /// A fresh in-memory audit sink is allocated so unit tests and isolated
    /// providers can exercise the bridge without wiring a shared replay store.
    pub fn new(runtime: Arc<ServiceRuntime>, config: WasmHostImportBridgeConfig) -> Self {
        let audit_sink: Arc<dyn ServiceCallAuditSink> =
            Arc::new(InMemoryServiceCallAuditSink::new());
        Self::new_with_audit_sink(runtime, config, audit_sink)
    }

    /// Create a bridge with an externally provided audit sink.
    ///
    /// Host composition layers can inject a shared sink so audit replay remains
    /// generic and consistent across runtime boundaries without any
    /// application-specific coupling.
    pub fn new_with_audit_sink(
        runtime: Arc<ServiceRuntime>,
        config: WasmHostImportBridgeConfig,
        audit_sink: Arc<dyn ServiceCallAuditSink>,
    ) -> Self {
        let policy_engine = Arc::new(InMemoryServicePolicyEngine::new());
        let policy_engine_trait: Arc<dyn ServicePolicyEngine> = policy_engine.clone();
        let router = Arc::new(
            ServiceRouter::new(
                runtime,
                config.source.clone(),
                Arc::new(InMemoryServiceContractRegistry::new()),
                policy_engine_trait,
            )
            .with_audit_sink(audit_sink.clone()),
        );
        Self {
            router,
            policy_engine,
            audit_sink,
            config,
            telemetry: None,
            genui_surface_store: None,
        }
    }

    /// Return a bridge clone that stores declarative `ui.render` intents.
    ///
    /// This applies the Repository pattern at the host-import boundary. The
    /// bridge still owns import validation and audit metadata, while the shared
    /// store owns app/session/surface lookup for Application Service queries.
    pub fn with_genui_surface_store(
        mut self,
        store: crate::ApplicationGenUiSurfaceStore,
    ) -> Self {
        self.genui_surface_store = Some(store);
        self
    }

    /// Return a bridge clone that emits sanitized host-import telemetry.
    pub fn with_telemetry_sink(mut self, sink: WasmTelemetrySinkRef) -> Self {
        self.telemetry = Some(sink);
        self
    }

    /// Return the shared in-memory policy engine used by this bridge.
    ///
    /// Host composition can use this handle to install app-scoped allow/deny
    /// layers from manifest contracts during session/bootstrap phases. The
    /// bridge keeps ownership of routing; callers only provide data-driven
    /// policy layers.
    pub fn policy_engine(&self) -> Arc<InMemoryServicePolicyEngine> {
        Arc::clone(&self.policy_engine)
    }

    /// Replay service-call audit evidence chain by trace id.
    pub fn replay_service_call_audit_by_trace_id(
        &self,
        trace_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        self.router.replay_audit_by_trace_id(trace_id)
    }

    /// Replay service-call audit evidence chain by session id.
    pub fn replay_service_call_audit_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        self.router.replay_audit_by_session_id(session_id)
    }

    /// Return the audit sink bound to this bridge.
    ///
    /// Runtime hosts can re-use this sink in system-service providers so all
    /// query surfaces replay the same trace evidence chain.
    pub fn service_call_audit_sink(&self) -> Arc<dyn ServiceCallAuditSink> {
        self.audit_sink.clone()
    }

    /// Emit one sanitized telemetry event when a sink is configured.
    ///
    /// Centralizing emission here keeps dispatch modules focused on routing
    /// while preserving identical telemetry shape for admitted/denied/completed
    /// transitions.
    pub(super) fn emit_host_import_telemetry(
        &self,
        status: &str,
        trace_id: &str,
        reason_code: &str,
        import_name: Option<&str>,
        service_id: Option<&str>,
    ) {
        let mut event = WasmTelemetryEvent::new(
            WasmTelemetryStage::HostImport,
            status,
            "host_import_bridge",
        )
        .trace_id(trace_id)
        .reason_code(reason_code.to_string());
        if let Some(name) = import_name {
            event = event.metadata("import_name", name.to_string());
        }
        if let Some(id) = service_id {
            event = event.metadata("service_id", id.to_string());
        }
        emit_wasm_telemetry(self.telemetry.as_ref(), event);
    }
}
