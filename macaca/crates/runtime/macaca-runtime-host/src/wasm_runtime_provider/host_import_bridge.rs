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
    ApplicationImport, KernelServiceId, ServiceBusSource, ServiceCommandName, TraceContext,
    WasmHostImportAuditReport, WasmHostImportCategory, WasmHostImportCommand,
    WasmHostImportErrorKind, DEFAULT_WASM_MAX_PAYLOAD_BYTES, WASM_HOST_IMPORT_CAPABILITY,
    WASM_HOST_IMPORT_OPERATION, WASM_HOST_IMPORT_SERVICE_ID,
};
use serde_json::{Map, Value};
use tracing::{info, warn};

use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServiceCallAuditEvent, ServiceCallAuditSink, ServicePolicyEngine, ServicePolicyLayer,
    ServiceRouteRequest, ServiceRouter, ServiceRuntime, ServiceRuntimeError,
};

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
    router: Arc<ServiceRouter>,
    policy_engine: Arc<InMemoryServicePolicyEngine>,
    audit_sink: Arc<dyn ServiceCallAuditSink>,
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
        }
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
        self.apply_policy_overrides(&guest_command);
        match self
            .router
            .route(ServiceRouteRequest {
                app_id: guest_command.metadata.get("app.id").cloned(),
                tenant_id: guest_command.metadata.get("tenant.id").cloned(),
                session_id: guest_command.metadata.get("session.id").cloned(),
                service_id: service_id.clone(),
                operation,
                payload: guest_command.payload.clone(),
                metadata: guest_command.metadata.clone(),
                trace: trace.clone(),
            })
            .await
        {
            Ok(reply) => {
                let output = bound_json(sanitize_json(reply.output), self.config.max_output_bytes);
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
                for (key, value) in reply.metadata {
                    if is_safe_metadata_key(&key) {
                        result.metadata.insert(key, sanitize_label(value));
                    }
                }
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

    /// Apply optional app-scoped policy overrides carried by trusted host-side
    /// metadata. The bridge intentionally supports only bounded primitives:
    /// allow-list, deny-list, timeout, and retry count.
    ///
    /// Metadata contract:
    /// - `app.id`: required to scope policy.
    /// - `policy.allow_services`: comma-separated service ids.
    /// - `policy.deny_services`: comma-separated service ids.
    /// - `policy.timeout_ms`: u64.
    /// - `policy.max_retries`: u32.
    fn apply_policy_overrides(&self, command: &WasmHostImportCommand) {
        let Some(app_id) = command.metadata.get("app.id").map(String::as_str) else {
            return;
        };
        let allow_services = parse_csv_services(
            command
                .metadata
                .get("policy.allow_services")
                .map(String::as_str),
        );
        let deny_services = parse_csv_services(
            command
                .metadata
                .get("policy.deny_services")
                .map(String::as_str),
        );
        let timeout_ms = command
            .metadata
            .get("policy.timeout_ms")
            .and_then(|value| value.parse::<u64>().ok());
        let max_retries = command
            .metadata
            .get("policy.max_retries")
            .and_then(|value| value.parse::<u32>().ok());
        if allow_services.is_empty()
            && deny_services.is_empty()
            && timeout_ms.is_none()
            && max_retries.is_none()
        {
            return;
        }
        self.policy_engine.set_app_override(
            app_id.to_string(),
            ServicePolicyLayer {
                allow_services,
                deny_services,
                timeout_ms,
                max_retries,
            },
        );
        info!(
            app_id,
            timeout_ms = timeout_ms.unwrap_or_default(),
            max_retries = max_retries.unwrap_or_default(),
            "WASM host import applied app-scoped policy override"
        );
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

fn is_safe_metadata_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    !(lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("credential"))
}

fn parse_csv_services(value: Option<&str>) -> std::collections::BTreeSet<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
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
