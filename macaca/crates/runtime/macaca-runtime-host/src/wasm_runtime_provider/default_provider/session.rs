//! Execution session for the default in-process WASM provider.
//!
//! The session owns one compiled module instance and routes traced host commands
//! either through the host-import bridge or guest export invocation.  Lifecycle
//! transitions delegate to `lifecycle_support` so checkpoint/restore logic stays
//! shared across provider strategies.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult,
    ApplicationHostCommandStatus, TraceContext, WasmCheckpointMemento, WasmLifecycleAuditEvent,
    WasmLifecycleCommand, WasmLifecycleState, WasmLifecycleStateMachine, WasmLifecycleTransitionResult,
    WasmRestoreReport, WasmRestoreRequest, WasmRollbackReport, WasmRollbackRequest,
    WasmRuntimeDiagnostics, WasmRuntimeErrorKind, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest, WasmUpgradeReport, WasmUpgradeRequest,
};
use serde_json::json;
use tracing::{info, warn};

use super::artifact_loader::is_wasm_export_invoke;
use super::DefaultInProcessWasmRuntimeProvider;
use super::super::diagnostics::non_empty_trace;
use super::super::engine_adapter::{CompiledWasmModule, InProcessWasmInstance};
use super::super::errors::{runtime_error_result, WasmRuntimeHostError};
use super::super::traits::WasmApplicationRuntimeProvider;
use super::super::host_import_bridge::WasmHostImportBridge;
use super::super::sandbox_guard::{WasmSandboxGuard, WasmSessionPermit};
use super::super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use super::super::traits::WasmExecutionSession;

/// Execution session for the default in-process provider.
#[derive(Debug)]
pub struct DefaultInProcessWasmExecutionSession {
    pub(crate) session_id: String,
    pub(crate) request: WasmRuntimeSessionRequest,
    pub(crate) module: Arc<CompiledWasmModule>,
    pub(crate) instance: InProcessWasmInstance,
    pub(crate) sandbox_guard: WasmSandboxGuard,
    pub(crate) host_import_bridge: Option<Arc<WasmHostImportBridge>>,
    pub(crate) telemetry: Option<WasmTelemetrySinkRef>,
    pub(crate) lifecycle: Mutex<WasmLifecycleStateMachine>,
    pub(crate) audit_events: Mutex<Vec<WasmLifecycleAuditEvent>>,
    pub(crate) _permit: WasmSessionPermit,
    pub(crate) cache_state: String,
    pub(crate) artifact_digest: String,
}

#[async_trait]
impl WasmExecutionSession for DefaultInProcessWasmExecutionSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn descriptor(&self) -> WasmRuntimeProviderDescriptor {
        WasmApplicationRuntimeProvider::descriptor(&DefaultInProcessWasmRuntimeProvider::default())
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
        super::super::lifecycle_support::transition_lifecycle(self, command)
    }

    async fn checkpoint(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmCheckpointMemento, ApplicationAbiError> {
        super::super::lifecycle_support::checkpoint(self, command)
    }

    async fn restore(
        &self,
        request: WasmRestoreRequest,
    ) -> Result<WasmRestoreReport, ApplicationAbiError> {
        super::super::lifecycle_support::restore(self, request)
    }

    async fn upgrade(
        &self,
        request: WasmUpgradeRequest,
    ) -> Result<WasmUpgradeReport, ApplicationAbiError> {
        super::super::lifecycle_support::upgrade(self, request)
    }

    async fn rollback(
        &self,
        request: WasmRollbackRequest,
    ) -> Result<WasmRollbackReport, ApplicationAbiError> {
        super::super::lifecycle_support::rollback(self, request)
    }
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
