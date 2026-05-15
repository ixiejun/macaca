//! Component Model WASM runtime provider.
//!
//! The provider implements the same `WasmApplicationRuntimeProvider` Strategy
//! used by the default and unavailable providers.  It keeps Component Model
//! validation and invocation behind a private Adapter so public Macaca layers
//! remain provider-neutral and Route C dependency boundaries stay intact.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    ApplicationAbiError, ApplicationAbiVersion, ApplicationHostCommand,
    ApplicationHostCommandResult, ApplicationImport, PackageRuntimeKind, TraceContext,
    WasmCheckpointMemento, WasmEngineCapabilities, WasmExecutionProfile, WasmLifecycleCommand,
    WasmLifecycleOperationStatus, WasmLifecycleReasonCode, WasmLifecycleState,
    WasmLifecycleStateMachine, WasmLifecycleTransitionResult, WasmRestoreReport,
    WasmRestoreRequest, WasmRollbackReport, WasmRollbackRequest, WasmRuntimeAvailability,
    WasmRuntimeDiagnostics, WasmRuntimeErrorKind, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest, WasmUpgradeReport, WasmUpgradeRequest,
};
use serde_json::json;
use tracing::{info, warn};

use super::component_model_adapter::{
    PortableComponentModelAdapter, WasmComponentEngineAdapter, WasmComponentInstance,
    WasmComponentModule,
};
use super::diagnostics::{non_empty_trace, session_id_from_request};
use super::errors::{runtime_error_result, WasmRuntimeHostError};
use super::host_import_bridge::WasmHostImportBridge;
use super::sandbox_guard::{active_resource_policy, WasmSandboxGuard, WasmSessionPermit};
use super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use super::traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};

/// Runtime-host-owned Component Model provider Strategy.
#[derive(Debug, Clone)]
pub struct ComponentModelWasmRuntimeProvider {
    adapter: Arc<dyn WasmComponentEngineAdapter>,
    sandbox_guard: WasmSandboxGuard,
    host_import_bridge: Option<Arc<WasmHostImportBridge>>,
    telemetry: Option<WasmTelemetrySinkRef>,
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

    fn descriptor_metadata(&self) -> std::collections::BTreeMap<String, String> {
        let mut metadata = std::collections::BTreeMap::new();
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
            lifecycle: Mutex::new(WasmLifecycleStateMachine::instantiated()),
            artifact_digest,
            _permit: permit,
        }))
    }
}

/// Execution session created by the Component Model provider.
#[derive(Debug)]
struct ComponentModelWasmExecutionSession {
    session_id: String,
    request: WasmRuntimeSessionRequest,
    module: WasmComponentModule,
    instance: WasmComponentInstance,
    sandbox_guard: WasmSandboxGuard,
    host_import_bridge: Option<Arc<WasmHostImportBridge>>,
    telemetry: Option<WasmTelemetrySinkRef>,
    lifecycle: Mutex<WasmLifecycleStateMachine>,
    artifact_digest: String,
    _permit: WasmSessionPermit,
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
    ) -> Result<WasmCheckpointMemento, ApplicationAbiError> {
        let trace = command
            .require_trace()
            .map_err(|_| ApplicationAbiError::MissingTraceContext)?;
        Ok(WasmCheckpointMemento {
            checkpoint_id: format!("component-checkpoint-{}", self.session_id),
            session_id: self.session_id.clone(),
            application_id: self.request.application_id.clone(),
            ability_id: self.request.ability_id.clone(),
            lifecycle_state: self.lifecycle_state(),
            artifact: self.request.artifact.clone(),
            artifact_digest_prefix: self.artifact_digest.chars().take(12).collect(),
            abi_version: ApplicationAbiVersion::v0(),
            created_at: Utc::now(),
            trace: Some(trace),
            metadata: Default::default(),
        })
    }

    async fn restore(
        &self,
        request: WasmRestoreRequest,
    ) -> Result<WasmRestoreReport, ApplicationAbiError> {
        let compatible = request.checkpoint.artifact == self.request.artifact;
        Ok(WasmRestoreReport {
            status: if compatible {
                WasmLifecycleOperationStatus::Completed
            } else {
                WasmLifecycleOperationStatus::Rejected
            },
            reason_code: if compatible {
                "completed"
            } else {
                "abi_mismatch"
            }
            .into(),
            checkpoint_id: request.checkpoint.checkpoint_id,
            lifecycle_state: self.lifecycle_state(),
            trace: request.trace,
            metadata: Default::default(),
        })
    }

    async fn upgrade(
        &self,
        request: WasmUpgradeRequest,
    ) -> Result<WasmUpgradeReport, ApplicationAbiError> {
        let compatible = request.target_abi_version == ApplicationAbiVersion::v0();
        Ok(WasmUpgradeReport {
            status: if compatible {
                WasmLifecycleOperationStatus::Completed
            } else {
                WasmLifecycleOperationStatus::Rejected
            },
            reason_code: if compatible {
                "completed"
            } else {
                "abi_mismatch"
            }
            .into(),
            source_artifact: self.request.artifact.clone(),
            source_artifact_digest_prefix: self.artifact_digest.chars().take(12).collect(),
            target_artifact: request.target_artifact,
            target_artifact_digest_prefix: request
                .target_artifact_digest
                .chars()
                .take(12)
                .collect(),
            abi_compatible: compatible,
            trace: request.trace,
            metadata: Default::default(),
        })
    }

    async fn rollback(
        &self,
        request: WasmRollbackRequest,
    ) -> Result<WasmRollbackReport, ApplicationAbiError> {
        Ok(WasmRollbackReport {
            status: WasmLifecycleOperationStatus::Completed,
            reason_code: WasmLifecycleReasonCode::Completed.as_code().into(),
            checkpoint_id: request.checkpoint.checkpoint_id,
            restored_lifecycle_state: request.checkpoint.lifecycle_state,
            trace: request.trace,
            metadata: Default::default(),
        })
    }
}

impl ComponentModelWasmExecutionSession {
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

    fn attach_common_metadata(&self, result: &mut ApplicationHostCommandResult, export_name: &str) {
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

fn apply_host_command_template(
    value: serde_json::Value,
    export_payload: &serde_json::Value,
    host_results: &[serde_json::Value],
) -> serde_json::Value {
    match value {
        serde_json::Value::String(text) => resolve_chat_payload_template(&text, export_payload)
            .or_else(|| resolve_host_result_template(&text, host_results))
            .unwrap_or_else(|| {
                serde_json::Value::String(interpolate_host_command_template(
                    &text,
                    export_payload,
                    host_results,
                ))
            }),
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(|item| apply_host_command_template(item, export_payload, host_results))
                .collect(),
        ),
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .map(|(key, item)| {
                    (
                        key,
                        apply_host_command_template(item, export_payload, host_results),
                    )
                })
                .collect(),
        ),
        other => other,
    }
}

/// Interpolate embedded placeholders inside human-facing strings.
///
/// Exact placeholders are still resolved by `resolve_*_template` first so JSON
/// payload fields keep their native value type.  This helper is only for mixed
/// text such as `"${chat.symbol} Signal Analysis"`, where a string result is
/// already the correct output type for GenUI labels, Markdown, or display text.
fn interpolate_host_command_template(
    text: &str,
    export_payload: &serde_json::Value,
    host_results: &[serde_json::Value],
) -> String {
    let mut rendered = String::with_capacity(text.len());
    let mut remainder = text;
    while let Some(start) = remainder.find("${") {
        rendered.push_str(&remainder[..start]);
        let after_start = &remainder[start..];
        let Some(end) = after_start.find('}') else {
            rendered.push_str(after_start);
            return rendered;
        };
        let placeholder = &after_start[..=end];
        if let Some(value) = resolve_chat_payload_template(placeholder, export_payload)
            .or_else(|| resolve_host_result_template(placeholder, host_results))
        {
            rendered.push_str(&template_value_to_display_text(&value));
        } else {
            rendered.push_str(placeholder);
        }
        remainder = &after_start[end + 1..];
    }
    rendered.push_str(remainder);
    rendered
}

/// Convert an interpolated JSON value into stable display text.
///
/// Scalar values render without JSON quotes so labels stay clean; structured
/// values use compact JSON so audit text remains deterministic if an app embeds
/// a non-scalar placeholder in a display string.
fn template_value_to_display_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
        }
    }
}

/// Resolve an exact chat-payload placeholder without interpreting its content.
///
/// This is intentionally a structural lookup only.  `${chat.symbol}` means the
/// app/coordinator already supplied a typed `symbol` field in the export
/// payload; the runtime must never parse `${chat.input}` or infer domain data
/// from free-form prose.
fn resolve_chat_payload_template(
    text: &str,
    export_payload: &serde_json::Value,
) -> Option<serde_json::Value> {
    let path = text.strip_prefix("${chat.")?.strip_suffix('}')?;
    let mut current = export_payload.clone();
    for part in path.split('.') {
        current = match current {
            serde_json::Value::Object(map) => map.get(part)?.clone(),
            serde_json::Value::Array(items) => {
                let item_index = part.parse::<usize>().ok()?;
                items.get(item_index)?.clone()
            }
            _ => return None,
        };
    }
    Some(current)
}

/// Resolve a full-value placeholder against prior declared host-command results.
///
/// The portable Component Model adapter cannot yet execute guest bytecode, so
/// it models guest orchestration through declarative host-command metadata.  A
/// later command may reference a prior command result with a bounded JSON path
/// such as `${host.results.0.output}` or `${host.results.2.output.analysis}`.
/// The runtime resolves only exact full-value placeholders; it deliberately
/// avoids string interpolation so payload types remain JSON-native and audit
/// output stays predictable.
fn resolve_host_result_template(
    text: &str,
    host_results: &[serde_json::Value],
) -> Option<serde_json::Value> {
    let path = text.strip_prefix("${host.results.")?.strip_suffix('}')?;
    let mut parts = path.split('.');
    let index = parts.next()?.parse::<usize>().ok()?;
    let mut current = host_results.get(index)?.clone();
    for part in parts {
        current = match current {
            serde_json::Value::Object(map) => map.get(part)?.clone(),
            serde_json::Value::Array(items) => {
                let item_index = part.parse::<usize>().ok()?;
                items.get(item_index)?.clone()
            }
            _ => return None,
        };
    }
    Some(current)
}

fn is_component_export_invoke(command: &ApplicationHostCommand) -> bool {
    command.metadata.contains_key("wasm.export")
        || matches!(
            &command.import,
            ApplicationImport::Custom(value) if value == "macaca:wasm/invoke"
        )
}

fn load_artifact_bytes(
    request: &WasmRuntimeSessionRequest,
) -> Result<Vec<u8>, WasmRuntimeHostError> {
    let path = artifact_path(request.artifact.as_str())?;
    fs::read(&path).map_err(|error| {
        WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            format!("WASM component artifact could not be loaded: {error}"),
        )
    })
}

fn artifact_path(reference: &str) -> Result<PathBuf, WasmRuntimeHostError> {
    let trimmed = reference.trim();
    let path = trimmed.strip_prefix("file://").unwrap_or(trimmed);
    if path.is_empty() {
        return Err(WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            "WASM component artifact reference is empty",
        ));
    }
    let candidate = Path::new(path);
    if candidate.components().next().is_none() {
        return Err(WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            "WASM component artifact reference does not resolve to a file",
        ));
    }
    Ok(candidate.to_path_buf())
}

fn component_digest(bytes: &[u8]) -> String {
    let sum = bytes.iter().fold(0u64, |accumulator, byte| {
        accumulator.wrapping_add(*byte as u64)
    });
    format!("portable-component-{sum:016x}")
}
