//! Declared host-command orchestration and host-import bridge dispatch.
//!
//! **Pattern:** Bridge delegation — routes declared host commands through the
//! [`WasmHostImportBridge`] while preserving parent trace correlation for audit
//! and UI follow-up queries.

use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult, TraceContext,
};
use serde_json::json;
use tracing::{info, warn};

use crate::wasm_runtime_provider::component_model::session::ComponentModelWasmExecutionSession;
use crate::wasm_runtime_provider::component_model::template_engine::apply_host_command_template;
use crate::wasm_runtime_provider::errors::{runtime_error_result, WasmRuntimeHostError};
use crate::wasm_runtime_provider::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetryStage,
};

/// Host-command orchestration and result shaping for component-model sessions.
pub(super) trait ComponentModelWasmExecutionSessionHost {
    async fn dispatch_declared_host_commands(
        &self,
        export_command: &ApplicationHostCommand,
        parent_trace: &TraceContext,
    ) -> Vec<serde_json::Value>;

    async fn dispatch_host_import(
        &self,
        command: ApplicationHostCommand,
        trace: TraceContext,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError>;

    fn error_result(
        &self,
        error: WasmRuntimeHostError,
        trace: Option<TraceContext>,
    ) -> ApplicationHostCommandResult;

    fn attach_common_metadata(
        &self,
        result: &mut ApplicationHostCommandResult,
        export_name: &str,
    );
}

impl ComponentModelWasmExecutionSessionHost for ComponentModelWasmExecutionSession {
    async fn dispatch_declared_host_commands(
        &self,
        export_command: &ApplicationHostCommand,
        parent_trace: &TraceContext,
    ) -> Vec<serde_json::Value> {
        let mut results = Vec::new();
        for (index, template) in self.module.host_commands().iter().enumerate() {
            let mut command = template.clone();
            let mut trace = TraceContext::new(format!(
                "{}:host-command:{}",
                parent_trace.trace_id,
                index + 1
            ));
            // Declared host commands are child operations of the export
            // dispatch.  Preserve the parent session link so UI imports,
            // service audit, and Web-shell follow-up queries all share the
            // same user-visible correlation key instead of falling back to the
            // provider-private component session id.
            trace.session_id = parent_trace.session_id.clone();
            command.trace = Some(trace.clone());
            command
                .metadata
                .insert("app.id".into(), self.request.application_id.clone());
            // The user-visible Web session is carried by the parent trace.
            // Use that id for child host-import metadata as well; imports such
            // as `macaca:agent/delegate` hydrate their service envelope from
            // metadata before consulting trace fields.  Falling back to the
            // provider-private component session would split skill snapshots,
            // delegate traces, and UI follow-up queries across two sessions.
            let command_session_id = parent_trace
                .session_id
                .clone()
                .unwrap_or_else(|| self.session_id.clone());
            command
                .metadata
                .insert("session.id".into(), command_session_id);
            command.payload =
                apply_host_command_template(command.payload, &export_command.payload, &results);
            info!(
                session_id = %self.session_id,
                trace_id = %trace.trace_id,
                import = %command.import,
                service_id = command.metadata.get("service.id").map(String::as_str).unwrap_or("none"),
                operation = command.metadata.get("service.operation").map(String::as_str).unwrap_or("none"),
                "WASM Component Model dispatching declared host command"
            );
            match self.dispatch_host_import(command, trace).await {
                Ok(result) => {
                    results.push(json!({
                        "index": index,
                        "status": result.status,
                        "output": result.output,
                        "metadata": result.metadata,
                        "trace": result.trace,
                    }));
                }
                Err(error) => {
                    results.push(json!({
                        "index": index,
                        "status": "error",
                        "error": error.to_string(),
                    }));
                }
            }
        }
        results
    }

    async fn dispatch_host_import(
        &self,
        command: ApplicationHostCommand,
        trace: TraceContext,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        if let Some(bridge) = &self.host_import_bridge {
            emit_wasm_telemetry(
                self.telemetry.as_ref(),
                WasmTelemetryEvent::new(
                    WasmTelemetryStage::HostImport,
                    "dispatching",
                    "component_model",
                )
                .trace_id(trace.trace_id.clone())
                .session_id(self.session_id.clone()),
            );
            let mut result = bridge.dispatch(command, trace).await;
            self.attach_common_metadata(&mut result, "");
            return Ok(result);
        }
        let mut result = ApplicationHostCommandResult::unavailable(
            "WASM Component Model host import bridge is not installed",
            Some(trace),
        );
        result
            .metadata
            .insert("reason_code".into(), "host_import_bridge_missing".into());
        self.attach_common_metadata(&mut result, "");
        Ok(result)
    }

    fn error_result(
        &self,
        error: WasmRuntimeHostError,
        trace: Option<TraceContext>,
    ) -> ApplicationHostCommandResult {
        let report = error.report(self.request.profile.runtime_kind.clone(), trace.clone());
        warn!(
            session_id = %self.session_id,
            trace_id = report.trace_id.as_deref().unwrap_or("none"),
            reason_code = %report.reason_code,
            "WASM Component Model session returned runtime error"
        );
        let stage = if report.reason_code == "resource_exhausted" {
            WasmTelemetryStage::Resource
        } else {
            WasmTelemetryStage::Invoke
        };
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(stage, "failed", "component_model")
                .trace_id(report.trace_id.as_deref().unwrap_or("none"))
                .session_id(self.session_id.clone())
                .reason_code(report.reason_code.clone()),
        );
        let mut result = runtime_error_result(report, trace);
        self.attach_common_metadata(&mut result, "");
        result
    }

    fn attach_common_metadata(
        &self,
        result: &mut ApplicationHostCommandResult,
        export_name: &str,
    ) {
        result
            .metadata
            .insert("provider_class".into(), "component_model".into());
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
            .insert("wit".into(), self.module.wit_label());
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
