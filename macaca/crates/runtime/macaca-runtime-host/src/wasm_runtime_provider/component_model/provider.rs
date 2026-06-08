//! Component Model WASM runtime provider Strategy.
//!
//! **Pattern:** Strategy — implements [`WasmApplicationRuntimeProvider`] for
//! component-model artifacts with declared host-command templates and lifecycle
//! hooks.  Session creation validates artifacts through a private adapter and
//! admits sessions through the sandbox guard before instantiation.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, PackageRuntimeKind, TraceContext, WasmEngineCapabilities,
    WasmExecutionProfile, WasmRuntimeAvailability, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest,
};
use tracing::info;

use crate::wasm_runtime_provider::component_model::session::ComponentModelWasmExecutionSession;
use crate::wasm_runtime_provider::component_model::support::{
    component_digest, load_artifact_bytes,
};
use crate::wasm_runtime_provider::component_model_adapter::{
    PortableComponentModelAdapter, WasmComponentEngineAdapter,
};
use crate::wasm_runtime_provider::diagnostics::session_id_from_request;
use crate::wasm_runtime_provider::host_import_bridge::WasmHostImportBridge;
use crate::wasm_runtime_provider::sandbox_guard::{active_resource_policy, WasmSandboxGuard};
use crate::wasm_runtime_provider::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use crate::wasm_runtime_provider::traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};
use macaca_proto::WasmLifecycleStateMachine;

/// Runtime-host-owned Component Model provider Strategy.
#[derive(Debug, Clone)]
pub struct ComponentModelWasmRuntimeProvider {
    pub(super) adapter: Arc<dyn WasmComponentEngineAdapter>,
    pub(super) sandbox_guard: WasmSandboxGuard,
    pub(super) host_import_bridge: Option<Arc<WasmHostImportBridge>>,
    pub(super) telemetry: Option<WasmTelemetrySinkRef>,
}

impl Default for ComponentModelWasmRuntimeProvider {
    fn default() -> Self {
        Self {
            adapter: Arc::new(PortableComponentModelAdapter),
            sandbox_guard: WasmSandboxGuard::default(),
            host_import_bridge: None,
            telemetry: None,
        }
    }
}

impl ComponentModelWasmRuntimeProvider {
    /// Return a provider clone with a ServiceRuntime-backed host import bridge.
    pub fn with_host_import_bridge(mut self, bridge: Arc<WasmHostImportBridge>) -> Self {
        self.host_import_bridge = Some(bridge);
        self
    }

    /// Return a provider clone that emits sanitized Component Model telemetry.
    pub fn with_telemetry_sink(mut self, sink: WasmTelemetrySinkRef) -> Self {
        self.telemetry = Some(sink);
        self
    }

    fn capabilities(&self) -> WasmEngineCapabilities {
        WasmEngineCapabilities {
            can_compile: true,
            can_instantiate: true,
            can_execute: true,
            supports_component_model: true,
            supports_host_import_bridge: self.host_import_bridge.is_some(),
            supports_wasi: false,
            engine_features: vec![
                "component-model-provider-v1".into(),
                "canonical-abi-adapter-boundary".into(),
            ],
            metadata: Default::default(),
        }
    }

    fn descriptor_metadata(&self) -> BTreeMap<String, String> {
        let mut metadata = BTreeMap::new();
        metadata.insert("governance.owner".into(), "runtime-host".into());
        metadata.insert(
            "governance.kernel_engine_dependency".into(),
            "forbidden".into(),
        );
        metadata.insert("adapter.visibility".into(), "private".into());
        metadata.insert("sandbox.raw_env".into(), "deny".into());
        metadata.insert("sandbox.raw_filesystem".into(), "deny".into());
        metadata.insert("sandbox.raw_network".into(), "deny".into());
        metadata
    }
}

#[async_trait]
impl WasmApplicationRuntimeProvider for ComponentModelWasmRuntimeProvider {
    fn descriptor(&self) -> WasmRuntimeProviderDescriptor {
        let availability = WasmRuntimeAvailability {
            state: "available".into(),
            reason: None,
            capabilities: self.capabilities(),
            diagnostics: None,
            metadata: Default::default(),
        };
        WasmRuntimeProviderDescriptor {
            runtime_kind: PackageRuntimeKind::WasmComponent,
            provider_class: "component_model".into(),
            capabilities: self.capabilities(),
            default_profile: WasmExecutionProfile::default_wasm_component(),
            availability,
            diagnostics: None,
            metadata: self.descriptor_metadata(),
        }
    }

    async fn availability(&self, trace: Option<TraceContext>) -> WasmRuntimeAvailability {
        info!(
            trace_id = trace.as_ref().map(|value| value.trace_id.as_str()).unwrap_or("none"),
            provider_class = "component_model",
            runtime_kind = %PackageRuntimeKind::WasmComponent,
            "WASM Component Model provider reported available"
        );
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Availability,
                "available",
                "component_model",
            )
            .trace_id(
                trace
                    .as_ref()
                    .map(|value| value.trace_id.as_str())
                    .unwrap_or("none"),
            ),
        );
        WasmRuntimeAvailability {
            state: "available".into(),
            reason: None,
            capabilities: self.capabilities(),
            diagnostics: None,
            metadata: Default::default(),
        }
    }

    async fn create_session(
        &self,
        request: WasmRuntimeSessionRequest,
    ) -> Result<Box<dyn WasmExecutionSession>, ApplicationAbiError> {
        request.validate()?;
        let session_id =
            session_id_from_request(&request).ok_or(ApplicationAbiError::MissingTraceContext)?;
        let permit = self
            .sandbox_guard
            .admit_session(&request)
            .map_err(|error| error.abi_error())?;
        let bytes = load_artifact_bytes(&request).map_err(|error| error.abi_error())?;
        let module = self
            .adapter
            .validate_component(&bytes)
            .map_err(|error| error.abi_error())?;
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(WasmTelemetryStage::Compile, "validated", "component_model")
                .trace_id(
                    request
                        .trace
                        .as_ref()
                        .map(|value| value.trace_id.as_str())
                        .unwrap_or("none"),
                )
                .session_id(session_id.clone()),
        );
        module
            .validate_resource_policy(&active_resource_policy(&request))
            .map_err(|error| error.abi_error())?;
        let instance = self
            .adapter
            .instantiate(module.clone())
            .map_err(|error| error.abi_error())?;
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Instantiate,
                "completed",
                "component_model",
            )
            .trace_id(
                request
                    .trace
                    .as_ref()
                    .map(|value| value.trace_id.as_str())
                    .unwrap_or("none"),
            )
            .session_id(session_id.clone()),
        );
        let artifact_digest = component_digest(&bytes);
        info!(
            session_id = %session_id,
            trace_id = request.trace.as_ref().map(|value| value.trace_id.as_str()).unwrap_or("none"),
            application_id = %request.application_id,
            ability_id = %request.ability_id,
            wit = %module.wit_label(),
            artifact_digest_prefix = %artifact_digest.chars().take(12).collect::<String>(),
            "WASM Component Model session created"
        );
        Ok(Box::new(ComponentModelWasmExecutionSession {
            session_id,
            request,
            module,
            instance,
            sandbox_guard: self.sandbox_guard.clone(),
            host_import_bridge: self.host_import_bridge.clone(),
            telemetry: self.telemetry.clone(),
            lifecycle: std::sync::Mutex::new(WasmLifecycleStateMachine::instantiated()),
            artifact_digest,
            _permit: permit,
        }))
    }
}
