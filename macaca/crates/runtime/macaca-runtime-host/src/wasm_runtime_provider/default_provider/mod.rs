//! Default in-process WASM runtime provider (facade).
//!
//! This provider is intentionally generic: it understands only provider-neutral
//! session requests, artifact references, and invocation metadata.  Session
//! dispatch, artifact loading, and lifecycle delegation live in sibling modules
//! so each responsibility stays under the OS file-size gate.

mod artifact_loader;
mod session;

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, PackageRuntimeKind, TraceContext, WasmEngineCapabilities,
    WasmExecutionProfile, WasmRuntimeAvailability, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest,
};
use tracing::info;

use super::compile_cache::InMemoryCompiledArtifactCache;
use super::diagnostics::session_id_from_request;
use super::engine_adapter::InProcessWasmEngineAdapter;
use super::host_import_bridge::WasmHostImportBridge;
use super::sandbox_guard::{active_resource_policy, WasmSandboxGuard};
use super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use super::traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};

pub use session::DefaultInProcessWasmExecutionSession;

/// Default provider that executes small core-WASM modules in process.
#[derive(Debug, Clone)]
pub struct DefaultInProcessWasmRuntimeProvider {
    pub(super) cache: Arc<InMemoryCompiledArtifactCache>,
    pub(super) adapter: InProcessWasmEngineAdapter,
    pub(super) sandbox_guard: WasmSandboxGuard,
    pub(super) host_import_bridge: Option<Arc<WasmHostImportBridge>>,
    pub(super) telemetry: Option<WasmTelemetrySinkRef>,
}

impl Default for DefaultInProcessWasmRuntimeProvider {
    fn default() -> Self {
        Self {
            cache: Arc::new(InMemoryCompiledArtifactCache::default()),
            adapter: InProcessWasmEngineAdapter,
            sandbox_guard: WasmSandboxGuard::default(),
            host_import_bridge: None,
            telemetry: None,
        }
    }
}

impl DefaultInProcessWasmRuntimeProvider {
    /// Return a provider clone with a ServiceRuntime-backed host import bridge.
    ///
    /// The bridge is optional so the default provider remains usable in
    /// minimal deployments.  When omitted, service imports fail closed with a
    /// structured unavailable result instead of falling through to guest export
    /// invocation or bypassing ServiceRuntime.
    pub fn with_host_import_bridge(mut self, bridge: Arc<WasmHostImportBridge>) -> Self {
        self.host_import_bridge = Some(bridge);
        self
    }

    /// Return a provider clone that emits sanitized Observer telemetry.
    ///
    /// The sink is optional and best-effort.  Runtime behavior stays identical
    /// when no sink is configured, which lets deployments enable telemetry
    /// without changing provider contracts or error handling.
    pub fn with_telemetry_sink(mut self, sink: WasmTelemetrySinkRef) -> Self {
        self.telemetry = Some(sink);
        self
    }

    fn capabilities(&self) -> WasmEngineCapabilities {
        WasmEngineCapabilities {
            can_compile: true,
            can_instantiate: true,
            can_execute: true,
            supports_component_model: false,
            supports_host_import_bridge: self.host_import_bridge.is_some(),
            supports_wasi: false,
            engine_features: vec!["core-wasm-nullary-export-v0".into()],
            metadata: Default::default(),
        }
    }

    fn descriptor_metadata(&self) -> std::collections::BTreeMap<String, String> {
        let mut metadata = std::collections::BTreeMap::new();
        metadata.insert("sandbox.raw_env".into(), "deny".into());
        metadata.insert("sandbox.raw_filesystem".into(), "deny".into());
        metadata.insert("sandbox.raw_network".into(), "deny".into());
        metadata.insert(
            "sandbox.deny_raw_wasi".into(),
            self.sandbox_guard
                .sandbox_policy()
                .denies_raw_wasi()
                .to_string(),
        );
        metadata
    }
}

#[async_trait]
impl WasmApplicationRuntimeProvider for DefaultInProcessWasmRuntimeProvider {
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
            provider_class: "default_in_process".into(),
            capabilities: self.capabilities(),
            default_profile: WasmExecutionProfile::default_wasm_component(),
            availability,
            diagnostics: None,
            metadata: self.descriptor_metadata(),
        }
    }

    async fn availability(&self, trace: Option<TraceContext>) -> WasmRuntimeAvailability {
        let trace_id = trace
            .as_ref()
            .map(|value| value.trace_id.as_str())
            .unwrap_or("none");
        info!(
            trace_id = trace_id,
            runtime_kind = %PackageRuntimeKind::WasmComponent,
            provider_class = "default_in_process",
            "WASM default in-process provider reported available"
        );
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Availability,
                "available",
                "default_in_process",
            )
            .trace_id(trace_id),
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
        let trace = request.trace.clone();
        let bytes =
            artifact_loader::load_artifact_bytes(&request).map_err(|error| error.abi_error())?;
        let (module, cache_report) = self
            .cache
            .get_or_compile(&request, &bytes, &self.adapter)
            .map_err(|error| error.abi_error())?;
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Compile,
                "completed",
                "default_in_process",
            )
            .trace_id(
                request
                    .trace
                    .as_ref()
                    .map(|value| value.trace_id.as_str())
                    .unwrap_or("none"),
            )
            .session_id(session_id.clone())
            .metadata("cache_state", format!("{:?}", cache_report.state)),
        );
        module
            .validate_resource_policy(&active_resource_policy(&request))
            .map_err(|error| error.abi_error())?;
        let instance = self
            .adapter
            .instantiate((*module).clone())
            .map_err(|error| error.abi_error())?;
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Instantiate,
                "completed",
                "default_in_process",
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
        info!(
            session_id = %session_id,
            trace_id = trace.as_ref().map(|value| value.trace_id.as_str()).unwrap_or("none"),
            application_id = %request.application_id,
            ability_id = %request.ability_id,
            runtime_kind = %request.profile.runtime_kind,
            cache_state = ?cache_report.state,
            artifact_digest_prefix = %cache_report.key.digest_value.chars().take(12).collect::<String>(),
            "WASM default in-process session created"
        );
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(WasmTelemetryStage::Session, "created", "default_in_process")
                .trace_id(
                    trace
                        .as_ref()
                        .map(|value| value.trace_id.as_str())
                        .unwrap_or("none"),
                )
                .session_id(session_id.clone())
                .metadata("cache_state", format!("{:?}", cache_report.state)),
        );
        Ok(Box::new(DefaultInProcessWasmExecutionSession {
            session_id,
            request,
            module,
            instance,
            sandbox_guard: self.sandbox_guard.clone(),
            host_import_bridge: self.host_import_bridge.clone(),
            telemetry: self.telemetry.clone(),
            lifecycle: std::sync::Mutex::new(
                macaca_proto::WasmLifecycleStateMachine::instantiated(),
            ),
            audit_events: std::sync::Mutex::new(Vec::new()),
            _permit: permit,
            cache_state: format!("{:?}", cache_report.state).to_ascii_lowercase(),
            artifact_digest: cache_report.key.digest_value,
        }))
    }
}
