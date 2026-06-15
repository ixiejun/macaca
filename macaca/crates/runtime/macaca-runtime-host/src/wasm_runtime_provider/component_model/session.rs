//! Component Model WASM execution session — core dispatch surface.
//!
//! **Pattern:** Session Object — owns per-session adapter state, sandbox permit,
//! lifecycle mutex, and routes commands to export invoke or host-import paths.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult, PackageRuntimeKind,
    TraceContext, WasmLifecycleCommand, WasmLifecycleState, WasmLifecycleStateMachine,
    WasmLifecycleTransitionResult, WasmRuntimeDiagnostics, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest,
};
use serde_json::json;
use tracing::{info, warn};

use crate::wasm_runtime_provider::component_model::provider::ComponentModelWasmRuntimeProvider;
use crate::wasm_runtime_provider::component_model::session_host::ComponentModelWasmExecutionSessionHost;
use crate::wasm_runtime_provider::component_model::session_lifecycle::ComponentModelWasmExecutionSessionLifecycle;
use crate::wasm_runtime_provider::component_model::support::is_component_export_invoke;
use crate::wasm_runtime_provider::component_model_adapter::{
    WasmComponentInstance, WasmComponentModule,
};
use crate::wasm_runtime_provider::diagnostics::non_empty_trace;
use crate::wasm_runtime_provider::host_import_bridge::WasmHostImportBridge;
use crate::wasm_runtime_provider::sandbox_guard::{WasmSandboxGuard, WasmSessionPermit};
use crate::wasm_runtime_provider::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use crate::wasm_runtime_provider::traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};

/// Execution session created by the Component Model provider.
#[derive(Debug)]
pub(super) struct ComponentModelWasmExecutionSession {
    pub(super) session_id: String,
    pub(super) request: WasmRuntimeSessionRequest,
    pub(super) module: WasmComponentModule,
    pub(super) instance: WasmComponentInstance,
    pub(super) sandbox_guard: WasmSandboxGuard,
    pub(super) host_import_bridge: Option<Arc<WasmHostImportBridge>>,
    pub(super) telemetry: Option<WasmTelemetrySinkRef>,
    pub(super) lifecycle: Mutex<WasmLifecycleStateMachine>,
    pub(super) artifact_digest: String,
    pub(super) _permit: WasmSessionPermit,
}

#[async_trait]
impl WasmExecutionSession for ComponentModelWasmExecutionSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn descriptor(&self) -> WasmRuntimeProviderDescriptor {
        ComponentModelWasmRuntimeProvider::default().descriptor()
    }

    fn diagnostics(&self) -> WasmRuntimeDiagnostics {
        WasmRuntimeDiagnostics::new(
            self.request.profile.runtime_kind.clone(),
            "ready",
            "WASM Component Model session is ready",
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
                reason_code = "missing_trace",
                "WASM Component Model session rejected untraceable command"
            );
            emit_wasm_telemetry(
                self.telemetry.as_ref(),
                WasmTelemetryEvent::new(WasmTelemetryStage::Invoke, "rejected", "component_model")
                    .session_id(self.session_id.clone())
                    .reason_code("missing_trace"),
            );
            return Err(ApplicationAbiError::MissingTraceContext);
        };
        if let Err(error) =
            self.sandbox_guard
                .check_dispatch_payload(&self.request, &command, trace.clone())
        {
            return Ok(self.error_result(error, Some(trace)));
        }
        if !is_component_export_invoke(&command) {
            return self.dispatch_host_import(command, trace).await;
        }
        let export_name = command
            .metadata
            .get("wasm.export")
            .map(String::as_str)
            .unwrap_or("app:start");
        match self.instance.invoke_export(export_name) {
            Ok(()) => {
                let host_command_results =
                    self.dispatch_declared_host_commands(&command, &trace).await;
                info!(
                    session_id = %self.session_id,
                    trace_id = %trace.trace_id,
                    application_id = %self.request.application_id,
                    ability_id = %self.request.ability_id,
                    wasm_export = export_name,
                    wit = %self.module.wit_label(),
                    "WASM Component Model session invoked export"
                );
                emit_wasm_telemetry(
                    self.telemetry.as_ref(),
                    WasmTelemetryEvent::new(
                        WasmTelemetryStage::Invoke,
                        "completed",
                        "component_model",
                    )
                    .trace_id(trace.trace_id.clone())
                    .session_id(self.session_id.clone())
                    .metadata("wasm.export", export_name),
                );
                let mut result = ApplicationHostCommandResult::ok(
                    json!({
                        "export": export_name,
                        "host_command_results": host_command_results,
                    }),
                    Some(trace),
                );
                self.attach_common_metadata(&mut result, export_name);
                Ok(result)
            }
            Err(error) => Ok(self.error_result(error, Some(trace))),
        }
    }

    fn lifecycle_state(&self) -> WasmLifecycleState {
        self.lifecycle
            .lock()
            .expect("wasm component lifecycle mutex poisoned")
            .state()
    }

    async fn transition_lifecycle(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmLifecycleTransitionResult, ApplicationAbiError> {
        let mut lifecycle = self
            .lifecycle
            .lock()
            .expect("wasm component lifecycle mutex poisoned");
        let result = lifecycle.transition(&command);
        info!(
            session_id = %self.session_id,
            operation = ?result.operation,
            status = ?result.status,
            reason_code = %result.reason_code,
            "WASM Component Model lifecycle transition evaluated"
        );
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Lifecycle,
                "evaluated",
                "component_model",
            )
            .session_id(self.session_id.clone())
            .reason_code(result.reason_code.clone()),
        );
        Ok(result)
    }

    async fn checkpoint(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<macaca_proto::WasmCheckpointMemento, ApplicationAbiError> {
        self.checkpoint_memento(command).await
    }

    async fn restore(
        &self,
        request: macaca_proto::WasmRestoreRequest,
    ) -> Result<macaca_proto::WasmRestoreReport, ApplicationAbiError> {
        self.restore_from_checkpoint(request).await
    }

    async fn upgrade(
        &self,
        request: macaca_proto::WasmUpgradeRequest,
    ) -> Result<macaca_proto::WasmUpgradeReport, ApplicationAbiError> {
        self.upgrade_artifact(request).await
    }

    async fn rollback(
        &self,
        request: macaca_proto::WasmRollbackRequest,
    ) -> Result<macaca_proto::WasmRollbackReport, ApplicationAbiError> {
        self.rollback_to_checkpoint(request).await
    }
}
