//! Default in-process WASM runtime provider.
//!
//! This provider is intentionally generic: it understands only provider-neutral
//! session requests, artifact references, and invocation metadata.  It does not
//! hard-code application names, workflows, or business behavior.  The provider
//! composes loader, cache, and private engine-adapter responsibilities so each
//! part can be replaced without changing the public trait implementation.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult, ApplicationImport,
    PackageRuntimeKind, TraceContext, WasmCheckpointMemento, WasmEngineCapabilities,
    WasmExecutionProfile, WasmLifecycleAuditEvent, WasmLifecycleCommand, WasmLifecycleState,
    WasmLifecycleStateMachine, WasmLifecycleTransitionResult, WasmRestoreReport,
    WasmRestoreRequest, WasmRollbackReport, WasmRollbackRequest, WasmRuntimeAvailability,
    WasmRuntimeDiagnostics, WasmRuntimeErrorKind, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest, WasmUpgradeReport, WasmUpgradeRequest,
};
use serde_json::json;
use tracing::{info, warn};

use super::compile_cache::InMemoryCompiledArtifactCache;
use super::diagnostics::{non_empty_trace, session_id_from_request};
use super::engine_adapter::{
    CompiledWasmModule, InProcessWasmEngineAdapter, InProcessWasmInstance,
};
use super::errors::{runtime_error_result, WasmRuntimeHostError};
use super::host_import_bridge::WasmHostImportBridge;
use super::sandbox_guard::{active_resource_policy, WasmSandboxGuard, WasmSessionPermit};
use super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use super::traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};

/// Default provider that executes small core-WASM modules in process.
#[derive(Debug, Clone)]
pub struct DefaultInProcessWasmRuntimeProvider {
    cache: Arc<InMemoryCompiledArtifactCache>,
    adapter: InProcessWasmEngineAdapter,
    sandbox_guard: WasmSandboxGuard,
    host_import_bridge: Option<Arc<WasmHostImportBridge>>,
    telemetry: Option<WasmTelemetrySinkRef>,
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
        let bytes = load_artifact_bytes(&request).map_err(|error| error.abi_error())?;
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
            lifecycle: Mutex::new(WasmLifecycleStateMachine::instantiated()),
            audit_events: Mutex::new(Vec::new()),
            _permit: permit,
            cache_state: format!("{:?}", cache_report.state).to_ascii_lowercase(),
            artifact_digest: cache_report.key.digest_value,
        }))
    }
}

/// Execution session for the default in-process provider.
#[derive(Debug)]
pub(super) struct DefaultInProcessWasmExecutionSession {
    pub(super) session_id: String,
    pub(super) request: WasmRuntimeSessionRequest,
    pub(super) module: Arc<CompiledWasmModule>,
    pub(super) instance: InProcessWasmInstance,
    pub(super) sandbox_guard: WasmSandboxGuard,
    pub(super) host_import_bridge: Option<Arc<WasmHostImportBridge>>,
    pub(super) telemetry: Option<WasmTelemetrySinkRef>,
    pub(super) lifecycle: Mutex<WasmLifecycleStateMachine>,
    pub(super) audit_events: Mutex<Vec<WasmLifecycleAuditEvent>>,
    pub(super) _permit: WasmSessionPermit,
    pub(super) cache_state: String,
    pub(super) artifact_digest: String,
}

#[async_trait]
impl WasmExecutionSession for DefaultInProcessWasmExecutionSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn descriptor(&self) -> WasmRuntimeProviderDescriptor {
        DefaultInProcessWasmRuntimeProvider::default().descriptor()
    }

    fn diagnostics(&self) -> WasmRuntimeDiagnostics {
        WasmRuntimeDiagnostics::new(
            self.request.profile.runtime_kind.clone(),
            "ready",
            "WASM default in-process session is ready",
            self.request.trace.clone(),
        )
    }

    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        let Some(trace) = non_empty_trace(command.trace.clone()) else {
            warn!(
                session_id = %self.session_id,
                import = %command.import,
                reason_code = "missing_trace",
                "WASM default in-process session rejected untraceable command"
            );
            emit_wasm_telemetry(
                self.telemetry.as_ref(),
                WasmTelemetryEvent::new(
                    WasmTelemetryStage::Invoke,
                    "rejected",
                    "default_in_process",
                )
                .session_id(self.session_id.clone())
                .reason_code("missing_trace"),
            );
            return Err(ApplicationAbiError::MissingTraceContext);
        };
        let export_name = command
            .metadata
            .get("wasm.export")
            .map(String::as_str)
            .unwrap_or("app:start");
        if let Err(error) =
            self.sandbox_guard
                .check_dispatch_payload(&self.request, &command, trace.clone())
        {
            return Ok(self.error_result(error, Some(trace)));
        }
        if !is_wasm_export_invoke(&command) {
            if let Some(bridge) = &self.host_import_bridge {
                emit_wasm_telemetry(
                    self.telemetry.as_ref(),
                    WasmTelemetryEvent::new(
                        WasmTelemetryStage::HostImport,
                        "dispatching",
                        "default_in_process",
                    )
                    .trace_id(trace.trace_id.clone())
                    .session_id(self.session_id.clone()),
                );
                let mut result = bridge.dispatch(command, trace).await;
                self.attach_common_metadata(&mut result, "");
                return Ok(result);
            }
            let mut result = ApplicationHostCommandResult::unavailable(
                "WASM host import bridge is not installed",
                Some(trace),
            );
            result
                .metadata
                .insert("reason_code".into(), "host_import_bridge_missing".into());
            self.attach_common_metadata(&mut result, "");
            return Ok(result);
        }
        if !self.module.has_export(export_name) {
            let error = WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::InvokeFailed,
                format!("WASM export '{export_name}' is not available"),
            );
            return Ok(self.error_result(error, Some(trace)));
        }
        match self.instance.invoke_export(export_name) {
            Ok(()) => {
                info!(
                    session_id = %self.session_id,
                    trace_id = %trace.trace_id,
                    application_id = %self.request.application_id,
                    ability_id = %self.request.ability_id,
                    wasm_export = export_name,
                    "WASM default in-process session invoked exported function"
                );
                emit_wasm_telemetry(
                    self.telemetry.as_ref(),
                    WasmTelemetryEvent::new(
                        WasmTelemetryStage::Invoke,
                        "completed",
                        "default_in_process",
                    )
                    .trace_id(trace.trace_id.clone())
                    .session_id(self.session_id.clone())
                    .metadata("wasm.export", export_name),
                );
                let mut result =
                    ApplicationHostCommandResult::ok(json!({ "export": export_name }), Some(trace));
                self.attach_common_metadata(&mut result, export_name);
                Ok(result)
            }
            Err(error) => Ok(self.error_result(error, Some(trace))),
        }
    }

    fn lifecycle_state(&self) -> WasmLifecycleState {
        self.lifecycle
            .lock()
            .expect("wasm lifecycle mutex poisoned")
            .state()
    }

    async fn transition_lifecycle(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmLifecycleTransitionResult, ApplicationAbiError> {
        super::lifecycle_support::transition_lifecycle(self, command)
    }

    async fn checkpoint(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmCheckpointMemento, ApplicationAbiError> {
        super::lifecycle_support::checkpoint(self, command)
    }

    async fn restore(
        &self,
        request: WasmRestoreRequest,
    ) -> Result<WasmRestoreReport, ApplicationAbiError> {
        super::lifecycle_support::restore(self, request)
    }

    async fn upgrade(
        &self,
        request: WasmUpgradeRequest,
    ) -> Result<WasmUpgradeReport, ApplicationAbiError> {
        super::lifecycle_support::upgrade(self, request)
    }

    async fn rollback(
        &self,
        request: WasmRollbackRequest,
    ) -> Result<WasmRollbackReport, ApplicationAbiError> {
        super::lifecycle_support::rollback(self, request)
    }
}

fn is_wasm_export_invoke(command: &ApplicationHostCommand) -> bool {
    command.metadata.contains_key("wasm.export")
        || matches!(
            &command.import,
            ApplicationImport::Custom(value) if value == "macaca:wasm/invoke"
        )
}

impl DefaultInProcessWasmExecutionSession {
    fn error_result(
        &self,
        error: WasmRuntimeHostError,
        trace: Option<TraceContext>,
    ) -> ApplicationHostCommandResult {
        let report = error.report(self.request.profile.runtime_kind.clone(), trace.clone());
        warn!(
            session_id = %self.session_id,
            trace_id = report.trace_id.as_deref().unwrap_or("none"),
            application_id = %self.request.application_id,
            ability_id = %self.request.ability_id,
            reason_code = %report.reason_code,
            "WASM default in-process session returned runtime error"
        );
        let stage = if report.reason_code == "resource_exhausted" {
            WasmTelemetryStage::Resource
        } else {
            WasmTelemetryStage::Invoke
        };
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(stage, "failed", "default_in_process")
                .trace_id(report.trace_id.as_deref().unwrap_or("none"))
                .session_id(self.session_id.clone())
                .reason_code(report.reason_code.clone()),
        );
        let mut result = runtime_error_result(report, trace);
        self.attach_common_metadata(&mut result, "");
        result
    }

    fn attach_common_metadata(&self, result: &mut ApplicationHostCommandResult, export_name: &str) {
        result.metadata.insert(
            "runtime_kind".into(),
            self.request.profile.runtime_kind.to_string(),
        );
        result
            .metadata
            .insert("session_id".into(), self.session_id.clone());
        result
            .metadata
            .insert("application_id".into(), self.request.application_id.clone());
        result
            .metadata
            .insert("ability_id".into(), self.request.ability_id.clone());
        result
            .metadata
            .insert("cache_state".into(), self.cache_state.clone());
        result.metadata.insert(
            "artifact_digest_prefix".into(),
            self.artifact_digest.chars().take(12).collect(),
        );
        if !export_name.is_empty() {
            result
                .metadata
                .insert("wasm.export".into(), export_name.to_string());
        }
    }
}

fn load_artifact_bytes(
    request: &WasmRuntimeSessionRequest,
) -> Result<Vec<u8>, WasmRuntimeHostError> {
    let path = artifact_path(request.artifact.as_str())?;
    fs::read(&path).map_err(|error| {
        WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            format!("WASM artifact could not be loaded: {error}"),
        )
    })
}

fn artifact_path(reference: &str) -> Result<PathBuf, WasmRuntimeHostError> {
    let trimmed = reference.trim();
    let path = trimmed.strip_prefix("file://").unwrap_or(trimmed);
    if path.is_empty() {
        return Err(WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            "WASM artifact reference is empty",
        ));
    }
    let candidate = Path::new(path);
    if candidate.components().next().is_none() {
        return Err(WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            "WASM artifact reference does not resolve to a file",
        ));
    }
    Ok(candidate.to_path_buf())
}
