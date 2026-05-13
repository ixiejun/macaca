//! WASM host import bridge backed by `ServiceRuntime`.
//!
//! This module is the Bridge between guest-facing Application ABI imports and
//! Macaca's provider-neutral service runtime.  It owns only translation,
//! validation, routing, result bounding, and audit logging.  It intentionally
//! does not know concrete service implementations, workflow names, driver
//! names, backend clients, or business behavior.

use std::fmt;
use std::sync::Arc;

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandResult, ApplicationHostCommandStatus,
    ApplicationImport, KernelServiceId, ServiceBusSource, ServiceCommand, ServiceCommandName,
    TraceContext, WasmHostImportAuditReport, WasmHostImportCategory, WasmHostImportCommand,
    WasmHostImportErrorKind, DEFAULT_WASM_MAX_PAYLOAD_BYTES, WASM_HOST_IMPORT_CAPABILITY,
    WASM_HOST_IMPORT_OPERATION, WASM_HOST_IMPORT_SERVICE_ID,
};
use serde_json::{Map, Value};
use tracing::{info, warn};

use crate::{ServiceRuntime, ServiceRuntimeError};

use super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};

/// Runtime configuration for the host import bridge.
#[derive(Clone)]
pub struct WasmHostImportBridgeConfig {
    pub source: ServiceBusSource,
    pub max_payload_bytes: u64,
    pub max_output_bytes: u64,
}

impl Default for WasmHostImportBridgeConfig {
    fn default() -> Self {
        Self {
            source: ServiceBusSource::new("wasm.host.import"),
            max_payload_bytes: DEFAULT_WASM_MAX_PAYLOAD_BYTES,
            max_output_bytes: DEFAULT_WASM_MAX_PAYLOAD_BYTES,
        }
    }
}

/// Bridge that validates and routes WASM host imports through `ServiceRuntime`.
#[derive(Clone)]
pub struct WasmHostImportBridge {
    runtime: Arc<ServiceRuntime>,
    config: WasmHostImportBridgeConfig,
    telemetry: Option<WasmTelemetrySinkRef>,
}

impl fmt::Debug for WasmHostImportBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmHostImportBridge")
            .field("source", &self.config.source.to_string())
            .field("max_payload_bytes", &self.config.max_payload_bytes)
            .field("max_output_bytes", &self.config.max_output_bytes)
            .finish_non_exhaustive()
    }
}

impl WasmHostImportBridge {
    /// Create a bridge over an existing host-owned `ServiceRuntime`.
    pub fn new(runtime: Arc<ServiceRuntime>, config: WasmHostImportBridgeConfig) -> Self {
        Self {
            runtime,
            config,
            telemetry: None,
        }
    }

    /// Return a bridge clone that emits sanitized host-import telemetry.
    pub fn with_telemetry_sink(mut self, sink: WasmTelemetrySinkRef) -> Self {
        self.telemetry = Some(sink);
        self
    }

    /// Dispatch one Application ABI command through the controlled service portal.
    pub async fn dispatch(
        &self,
        command: ApplicationHostCommand,
        trace: TraceContext,
    ) -> ApplicationHostCommandResult {
        let guest_command = match self.validate(command, trace.clone()) {
            Ok(command) => command,
            Err(result) => return result,
        };
        let audit = WasmHostImportAuditReport::new(
            "allow",
            "import_allowed",
            &guest_command,
            "WASM host import admitted for ServiceRuntime dispatch",
        );
        info!(
            trace_id = audit.trace_id.as_deref().unwrap_or("none"),
            import_name = %audit.import_name,
            service_id = audit.service_id.as_deref().unwrap_or("none"),
            operation = audit.operation.as_deref().unwrap_or("none"),
            capability = audit.capability.as_deref().unwrap_or("none"),
            payload_bytes = audit.payload_bytes,
            reason_code = %audit.reason_code,
            "WASM host import bridge admitted command"
        );
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::HostImport,
                "admitted",
                "host_import_bridge",
            )
            .trace_id(audit.trace_id.as_deref().unwrap_or("none"))
            .reason_code(audit.reason_code.clone())
            .metadata("import_name", audit.import_name.clone()),
        );

        let Some(service_id) = guest_command.target_service.clone() else {
            return self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::ServiceUnavailable,
                "WASM host import requires a target service",
            );
        };
        let Some(operation) = guest_command.operation.clone() else {
            return self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::UnsupportedImport,
                "WASM host import requires a service operation",
            );
        };
        let mut service_command =
            ServiceCommand::with_trace(operation, guest_command.payload.clone(), trace.clone());
        service_command.metadata = guest_command.metadata.clone();
        match self
            .runtime
            .call(&service_id, self.config.source.clone(), service_command)
            .await
        {
            Ok(reply) => {
                let output = bound_json(
                    sanitize_json(reply.output.unwrap_or(Value::Null)),
                    self.config.max_output_bytes,
                );
                let mut result = ApplicationHostCommandResult::ok(output, Some(trace));
                result
                    .metadata
                    .insert("reason_code".into(), "import_completed".into());
                result
                    .metadata
                    .insert("service_id".into(), service_id.to_string());
                result
                    .metadata
                    .insert("service_status".into(), sanitize_label(reply.status));
                self.attach_common_metadata(&guest_command, &mut result);
                info!(
                    trace_id = result
                        .trace
                        .as_ref()
                        .map(|value| value.trace_id.as_str())
                        .unwrap_or("none"),
                    service_id = %service_id,
                    reason_code = "import_completed",
                    "WASM host import bridge completed service call"
                );
                emit_wasm_telemetry(
                    self.telemetry.as_ref(),
                    WasmTelemetryEvent::new(
                        WasmTelemetryStage::HostImport,
                        "completed",
                        "host_import_bridge",
                    )
                    .trace_id(
                        result
                            .trace
                            .as_ref()
                            .map(|value| value.trace_id.as_str())
                            .unwrap_or("none"),
                    )
                    .reason_code("import_completed")
                    .metadata("service_id", service_id.to_string()),
                );
                result
            }
            Err(error) => self.error_result(&guest_command, error),
        }
    }

    fn validate(
        &self,
        command: ApplicationHostCommand,
        trace: TraceContext,
    ) -> Result<WasmHostImportCommand, ApplicationHostCommandResult> {
        let import_name = command.import.as_name().to_string();
        let category = WasmHostImportCategory::from_application_import(&command.import);
        if command.import != ApplicationImport::ServiceCall {
            let guest_command = self.command_shell(command, trace, category, import_name, 0);
            return Err(self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::UnsupportedImport,
                "WASM host import is not supported by this bridge",
            ));
        }
        let payload_bytes = serde_json::to_vec(&command.payload)
            .map(|payload| payload.len() as u64)
            .unwrap_or(u64::MAX);
        let guest_command =
            self.command_shell(command, trace, category, import_name, payload_bytes);
        if payload_bytes > self.config.max_payload_bytes {
            return Err(self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::PayloadTooLarge,
                "WASM host import payload exceeded the bridge limit",
            ));
        }
        if guest_command
            .capability
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            return Err(self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::CapabilityMissing,
                "WASM host import requires capability metadata",
            ));
        }
        Ok(guest_command)
    }

    fn command_shell(
        &self,
        command: ApplicationHostCommand,
        trace: TraceContext,
        category: WasmHostImportCategory,
        import_name: String,
        payload_bytes: u64,
    ) -> WasmHostImportCommand {
        let target_service = command
            .metadata
            .get(WASM_HOST_IMPORT_SERVICE_ID)
            .filter(|value| !value.trim().is_empty())
            .map(KernelServiceId::new);
        let operation = command
            .metadata
            .get(WASM_HOST_IMPORT_OPERATION)
            .filter(|value| !value.trim().is_empty())
            .map(ServiceCommandName::new);
        let capability = command
            .metadata
            .get(WASM_HOST_IMPORT_CAPABILITY)
            .filter(|value| !value.trim().is_empty())
            .cloned();
        WasmHostImportCommand {
            category,
            import_name,
            target_service,
            operation,
            capability,
            payload: command.payload,
            payload_bytes,
            trace,
            metadata: command.metadata,
        }
    }

    fn denied_result(
        &self,
        command: &WasmHostImportCommand,
        kind: WasmHostImportErrorKind,
        message: &str,
    ) -> ApplicationHostCommandResult {
        let audit = WasmHostImportAuditReport::new("deny", kind.as_code(), command, message);
        warn!(
            trace_id = audit.trace_id.as_deref().unwrap_or("none"),
            import_name = %audit.import_name,
            service_id = audit.service_id.as_deref().unwrap_or("none"),
            reason_code = %audit.reason_code,
            payload_bytes = audit.payload_bytes,
            "WASM host import bridge denied command"
        );
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::HostImport,
                "denied",
                "host_import_bridge",
            )
            .trace_id(audit.trace_id.as_deref().unwrap_or("none"))
            .reason_code(audit.reason_code.clone())
            .metadata("import_name", audit.import_name.clone()),
        );
        let status = match kind {
            WasmHostImportErrorKind::ServiceUnavailable => {
                ApplicationHostCommandStatus::Unavailable {
                    reason: "WASM host import service unavailable".into(),
                }
            }
            WasmHostImportErrorKind::UnsupportedImport => {
                ApplicationHostCommandStatus::Unsupported {
                    reason: "WASM host import unsupported".into(),
                }
            }
            _ => ApplicationHostCommandStatus::DisabledByPolicy {
                reason: "WASM host import denied by policy".into(),
            },
        };
        let mut result = ApplicationHostCommandResult {
            status,
            output: Value::Null,
            trace: Some(command.trace.clone()),
            policy: None,
            metadata: Default::default(),
        };
        result
            .metadata
            .insert("reason_code".into(), kind.as_code().into());
        self.attach_common_metadata(command, &mut result);
        result
    }

    fn error_result(
        &self,
        command: &WasmHostImportCommand,
        error: ServiceRuntimeError,
    ) -> ApplicationHostCommandResult {
        match error {
            ServiceRuntimeError::UnknownService(_) => self.denied_result(
                command,
                WasmHostImportErrorKind::ServiceUnavailable,
                "WASM host import target service is unavailable",
            ),
            ServiceRuntimeError::PolicyDenied(_) => self.denied_result(
                command,
                WasmHostImportErrorKind::PolicyDenied,
                "WASM host import was denied by ServiceRuntime policy",
            ),
            ServiceRuntimeError::MissingTraceContext => self.denied_result(
                command,
                WasmHostImportErrorKind::MissingTrace,
                "WASM host import is missing trace context",
            ),
            _ => self.denied_result(
                command,
                WasmHostImportErrorKind::ServiceFailed,
                "WASM host import service call failed",
            ),
        }
    }

    fn attach_common_metadata(
        &self,
        command: &WasmHostImportCommand,
        result: &mut ApplicationHostCommandResult,
    ) {
        result
            .metadata
            .insert("import_name".into(), command.import_name.clone());
        if let Some(service_id) = &command.target_service {
            result
                .metadata
                .insert("service_id".into(), service_id.to_string());
        }
        if let Some(operation) = &command.operation {
            result
                .metadata
                .insert("service.operation".into(), operation.to_string());
        }
        if let Some(capability) = &command.capability {
            result
                .metadata
                .insert("capability".into(), sanitize_label(capability));
        }
        result
            .metadata
            .insert("payload_bytes".into(), command.payload_bytes.to_string());
    }
}

fn sanitize_json(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut sanitized = Map::new();
            for (key, value) in object {
                let lower = key.to_ascii_lowercase();
                if lower.contains("raw")
                    || lower.contains("prompt")
                    || lower.contains("secret")
                    || lower.contains("payload")
                {
                    continue;
                }
                sanitized.insert(key, sanitize_json(value));
            }
            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sanitize_json).collect()),
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            if lower.contains("secret") || lower.contains("prompt") || lower.contains("api_key") {
                Value::String("[redacted]".into())
            } else {
                Value::String(text)
            }
        }
        other => other,
    }
}

fn bound_json(value: Value, max_bytes: u64) -> Value {
    let size = serde_json::to_vec(&value)
        .map(|payload| payload.len() as u64)
        .unwrap_or(u64::MAX);
    if size <= max_bytes {
        value
    } else {
        serde_json::json!({
            "truncated": true,
            "reason": "host_import_output_too_large",
            "bytes": size
        })
    }
}

fn sanitize_label(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
                ch
            } else if ch == '/' {
                ':'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}
