//! WASM host import bridge backed by `ServiceRuntime`.
//!
//! This module is the Bridge between guest-facing Application ABI imports and
//! Macaca's provider-neutral service runtime.  It owns only translation,
//! validation, routing, result bounding, and audit logging.  It intentionally
//! does not know concrete service implementations, workflow names, driver
//! names, backend clients, or business behavior.

use std::fmt;
use std::sync::Arc;

use macaca_app::UiIntentValidator;
use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandResult, ApplicationHostCommandStatus,
    ApplicationImport, KernelServiceId, ServiceBusSource, ServiceCommandName, TraceContext,
    UiIntent, WasmHostImportAuditReport, WasmHostImportCategory, WasmHostImportCommand,
    WasmHostImportErrorKind, APPLICATION_AGENT_DELEGATE_COMMAND, APPLICATION_SERVICE_ID,
    DEFAULT_WASM_MAX_PAYLOAD_BYTES, WASM_HOST_IMPORT_CAPABILITY, WASM_HOST_IMPORT_OPERATION,
    WASM_HOST_IMPORT_SERVICE_ID,
};
use serde_json::{Map, Value};
use tracing::{info, warn};

use crate::ApplicationGenUiSurfaceStore;
use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServiceCallAuditEvent, ServiceCallAuditSink, ServicePolicyEngine, ServicePolicyLayer,
    ServiceRouteRequest, ServiceRouter, ServiceRuntime, ServiceRuntimeError,
};

use super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};

const TASK_SERVICE_ID: &str = "service.task";
const TASK_CREATE_GOAL_OPERATION: &str = "task.create_goal";
const TASK_CREATE_ASSIGNMENT_OPERATION: &str = "task.create_assignment";
const TASK_QUERY_OPERATION: &str = "task.query";
const TASK_CLAIM_OPERATION: &str = "task.claim";
const TASK_START_OPERATION: &str = "task.start";
const TASK_SUBMIT_REVIEW_OPERATION: &str = "task.submit_review";
const TASK_REVIEW_OPERATION: &str = "task.review";
const APPLICATION_EXECUTION_GRAPH_OWNER: &str = "application_execution";

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
    genui_surface_store: Option<ApplicationGenUiSurfaceStore>,
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
            genui_surface_store: None,
        }
    }

    /// Return a bridge clone that stores declarative `ui.render` intents.
    ///
    /// This applies the Repository pattern at the host-import boundary.  The
    /// bridge still owns import validation and audit metadata, while the shared
    /// store owns app/session/surface lookup for Application Service queries.
    pub fn with_genui_surface_store(mut self, store: ApplicationGenUiSurfaceStore) -> Self {
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

        if guest_command.import_name == ApplicationImport::TraceEmit.as_name() {
            let mut result = ApplicationHostCommandResult::ok(
                serde_json::json!({ "emitted": true }),
                Some(trace),
            );
            result
                .metadata
                .insert("reason_code".into(), "trace_emit_recorded".into());
            self.attach_common_metadata(&guest_command, &mut result);
            info!(
                trace_id = result
                    .trace
                    .as_ref()
                    .map(|value| value.trace_id.as_str())
                    .unwrap_or("none"),
                "WASM trace.emit recorded by host import bridge"
            );
            return result;
        }

        if guest_command.import_name == ApplicationImport::UiRender.as_name() {
            return self.dispatch_ui_render(guest_command, trace).await;
        }
        let task_lifecycle =
            if guest_command.import_name == ApplicationImport::AgentDelegate.as_name() {
                match self
                    .open_agent_delegate_task_lifecycle(&guest_command, &trace)
                    .await
                {
                    Ok(task_id) => task_id,
                    Err(result) => return result,
                }
            } else {
                None
            };

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
                payload: hydrate_orchestration_service_payload(&guest_command, &trace),
                metadata: guest_command.metadata.clone(),
                trace: trace.clone(),
            })
            .await
        {
            Ok(reply) => {
                if let Some(task_id) = task_lifecycle.as_deref() {
                    if let Err(result) = self
                        .close_agent_delegate_task_lifecycle(
                            &guest_command,
                            &trace,
                            task_id,
                            true,
                            "agent delegate completed",
                        )
                        .await
                    {
                        return result;
                    }
                }
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
                if let Some(task_id) = task_lifecycle.as_deref() {
                    // Record the durable task id on the host-command result so
                    // replay consumers can correlate import completion with the
                    // Task Service lifecycle opened earlier in this bridge.
                    result.metadata.insert("task_id".into(), task_id.into());
                }
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
            Err(error) => {
                if let Some(task_id) = task_lifecycle.as_deref() {
                    let _ = self
                        .close_agent_delegate_task_lifecycle(
                            &guest_command,
                            &trace,
                            task_id,
                            false,
                            "agent delegate failed before completion",
                        )
                        .await;
                }
                self.error_result(&guest_command, error)
            }
        }
    }

    /// Create and start a Task Service record for an `agent_delegate` import.
    ///
    /// `agent_delegate` is a generic Application ABI operation: a guest asks the
    /// host to run one app-scoped agent.  The OS must therefore own the durable
    /// task-board lifecycle for that delegation instead of leaving application
    /// UIs to synthesize state from trace events.  This Adapter writes only
    /// provider-neutral task metadata and avoids any application-name or
    /// workflow-specific branching.
    async fn open_agent_delegate_task_lifecycle(
        &self,
        command: &WasmHostImportCommand,
        trace: &TraceContext,
    ) -> Result<Option<String>, ApplicationHostCommandResult> {
        let Some(app_id) = command.metadata.get("app.id").cloned() else {
            return Ok(None);
        };
        let Some(session_id) = command
            .metadata
            .get("session.id")
            .cloned()
            .or_else(|| trace.session_id.clone())
        else {
            return Ok(None);
        };
        let target_agent = command
            .payload
            .get("target_agent")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("agent");
        let created_by = command
            .metadata
            .get("agent.name")
            .map(String::as_str)
            .unwrap_or("wasm-guest");
        let priority = command
            .metadata
            .get("priority")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(5);
        let prompt = command
            .payload
            .get("prompt")
            .and_then(Value::as_str)
            .unwrap_or("Application requested delegated agent execution.");
        let task_id = self
            .route_task_lifecycle_command(
                command,
                TASK_CREATE_ASSIGNMENT_OPERATION,
                serde_json::json!({
                    "app_id": app_id,
                    "session_id": session_id,
                    "agent_name": target_agent,
                    "created_by": created_by,
                    "title": format!("Delegate task to {target_agent}"),
                    "description": prompt,
                    "acceptance_criteria": [
                        "The delegated agent reports a traceable completion result.",
                        "The Task Service records the lifecycle transition for audit and replay."
                    ],
                    "priority": priority,
                    "depends_on": [],
                    "parent_task": null,
                    "graph_owner": APPLICATION_EXECUTION_GRAPH_OWNER,
                    "trace": trace,
                }),
                trace,
            )
            .await?
            .and_then(extract_task_id_from_assignment);
        let Some(task_id) = task_id else {
            return Ok(None);
        };
        self.route_task_lifecycle_command(
            command,
            TASK_CLAIM_OPERATION,
            task_lifecycle_payload(&app_id, &session_id, target_agent, &task_id, trace),
            trace,
        )
        .await?;
        self.route_task_lifecycle_command(
            command,
            TASK_START_OPERATION,
            task_lifecycle_payload(&app_id, &session_id, target_agent, &task_id, trace),
            trace,
        )
        .await?;
        Ok(Some(task_id))
    }

    /// Submit and review the Task Service record for a completed delegation.
    async fn close_agent_delegate_task_lifecycle(
        &self,
        command: &WasmHostImportCommand,
        trace: &TraceContext,
        task_id: &str,
        passed: bool,
        summary: &str,
    ) -> Result<(), ApplicationHostCommandResult> {
        let Some(app_id) = command.metadata.get("app.id").cloned() else {
            return Ok(());
        };
        let Some(session_id) = command
            .metadata
            .get("session.id")
            .cloned()
            .or_else(|| trace.session_id.clone())
        else {
            return Ok(());
        };
        let target_agent = command
            .payload
            .get("target_agent")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("agent");
        let mut submit_payload =
            task_lifecycle_payload(&app_id, &session_id, target_agent, task_id, trace);
        submit_payload["summary"] = Value::String(summary.to_string());
        self.route_task_lifecycle_command(
            command,
            TASK_SUBMIT_REVIEW_OPERATION,
            submit_payload,
            trace,
        )
        .await?;
        let review_payload = serde_json::json!({
            "app_id": app_id,
            "session_id": session_id,
            "agent_name": target_agent,
            "task_id": task_id,
            "result": {
                "passed": passed,
                "feedback": summary,
                "verified_criteria": [
                    ["The delegated agent reports a traceable completion result.", passed],
                    ["The Task Service records the lifecycle transition for audit and replay.", true]
                ]
            },
            "trace": trace,
        });
        self.route_task_lifecycle_command(command, TASK_REVIEW_OPERATION, review_payload, trace)
            .await?;
        Ok(())
    }

    /// Route one internal Task Service lifecycle command.
    ///
    /// Internal lifecycle calls reuse the same ServiceRuntime decorators as
    /// guest calls, so policy, audit, trace, and structured unavailable behavior
    /// stay centralized.  The source command is passed only for policy scope and
    /// error shaping; no application-specific behavior is inspected here.
    async fn route_task_lifecycle_command(
        &self,
        source: &WasmHostImportCommand,
        operation: &str,
        payload: Value,
        trace: &TraceContext,
    ) -> Result<Option<Value>, ApplicationHostCommandResult> {
        match self
            .router
            .route(ServiceRouteRequest {
                app_id: source.metadata.get("app.id").cloned(),
                tenant_id: source.metadata.get("tenant.id").cloned(),
                session_id: source.metadata.get("session.id").cloned(),
                service_id: KernelServiceId::new(TASK_SERVICE_ID),
                operation: ServiceCommandName::new(operation),
                payload,
                metadata: source.metadata.clone(),
                trace: trace.clone(),
            })
            .await
        {
            Ok(reply) => Ok(Some(reply.output)),
            Err(error) => Err(self.error_result(source, error)),
        }
    }

    /// Store a declarative GenUI render intent without routing through services.
    ///
    /// `ui.render` is a presentation import, not a data/service import.  The
    /// bridge validates the schema and writes it to the shared repository so the
    /// shell can query it later through the Application Service.  This keeps UI
    /// rendering generic, traceable, and independent from application-specific
    /// code paths.
    async fn dispatch_ui_render(
        &self,
        guest_command: WasmHostImportCommand,
        trace: TraceContext,
    ) -> ApplicationHostCommandResult {
        let Some(store) = self.genui_surface_store.as_ref() else {
            return self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::ServiceUnavailable,
                "WASM ui.render store is unavailable",
            );
        };
        let mut intent = match serde_json::from_value::<UiIntent>(guest_command.payload.clone()) {
            Ok(intent) => intent,
            Err(error) => {
                warn!(
                    trace_id = %trace.trace_id,
                    error = %error,
                    "WASM ui.render payload failed to decode"
                );
                return self.denied_result(
                    &guest_command,
                    WasmHostImportErrorKind::PolicyDenied,
                    "WASM ui.render payload is invalid",
                );
            }
        };
        hydrate_ui_render_scope(&mut intent, &guest_command, &trace);
        if let Err(error) = UiIntentValidator.validate_intent(&intent) {
            warn!(
                trace_id = %trace.trace_id,
                error = %error,
                "WASM ui.render intent failed validation"
            );
            return self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::PolicyDenied,
                "WASM ui.render intent failed validation",
            );
        }
        if let Err(error) = store.store(intent.clone()).await {
            warn!(
                trace_id = %trace.trace_id,
                error = %error,
                "WASM ui.render intent failed to store"
            );
            return self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::ServiceFailed,
                "WASM ui.render store failed",
            );
        }
        let mut result =
            ApplicationHostCommandResult::ok(serde_json::json!({ "stored": true }), Some(trace));
        result
            .metadata
            .insert("reason_code".into(), "ui_render_stored".into());
        result
            .metadata
            .insert("surface_id".into(), intent.surface_id.to_string());
        self.attach_common_metadata(&guest_command, &mut result);
        info!(
            app_id = %intent.app_id,
            session_id = %intent.session_id,
            surface_id = %intent.surface_id,
            "WASM ui.render intent stored"
        );
        result
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
        if !is_supported_portal_import(&command.import) {
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
        if guest_command.import_name == ApplicationImport::UiRender.as_name() {
            return Ok(guest_command);
        }
        if guest_command.import_name == ApplicationImport::TraceEmit.as_name() {
            return Ok(guest_command);
        }
        if is_orchestration_import_name(&guest_command.import_name)
            && !has_non_empty_scope(&guest_command.metadata)
        {
            return Err(self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::ScopeMissing,
                "WASM orchestration import requires app and session scope",
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
        let default_target_service = default_service_for_import(&command.import);
        let default_operation = default_operation_for_import(&command.import);
        let target_service = command
            .metadata
            .get(WASM_HOST_IMPORT_SERVICE_ID)
            .filter(|value| !value.trim().is_empty())
            .map(KernelServiceId::new)
            .or_else(|| default_target_service.map(KernelServiceId::new));
        let operation = command
            .metadata
            .get(WASM_HOST_IMPORT_OPERATION)
            .filter(|value| !value.trim().is_empty())
            .map(ServiceCommandName::new)
            .or_else(|| default_operation.map(ServiceCommandName::new));
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
            WasmHostImportErrorKind::ServiceUnavailable
            | WasmHostImportErrorKind::ServiceFailed => ApplicationHostCommandStatus::Unavailable {
                reason: "WASM host import service unavailable".into(),
            },
            WasmHostImportErrorKind::UnsupportedImport => {
                ApplicationHostCommandStatus::Unsupported {
                    reason: "WASM host import unsupported".into(),
                }
            }
            WasmHostImportErrorKind::InvalidArgument => ApplicationHostCommandStatus::Rejected {
                reason: "WASM host import payload is invalid".into(),
            },
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
            ServiceRuntimeError::InvalidArgument(_) => self.denied_result(
                command,
                WasmHostImportErrorKind::InvalidArgument,
                "WASM host import payload is invalid",
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

/// Hydrate host-owned scope fields on a declarative UI intent.
///
/// Guests often cannot know the concrete application id or session id before
/// the host creates an execution session.  Web shells attach the user-visible
/// session id to the trace, while lower-level component sessions also attach an
/// internal runtime id to metadata.  The bridge prefers the trace session for UI
/// surfaces so frontend queries and `ui.render` storage use the same key, then
/// falls back to metadata for non-web hosts.  This keeps WASM artifacts
/// portable and prevents application code from hardcoding runtime identities.
fn hydrate_ui_render_scope(
    intent: &mut UiIntent,
    command: &WasmHostImportCommand,
    trace: &TraceContext,
) {
    if intent.app_id.trim().is_empty() || intent.app_id == "${app.id}" {
        if let Some(app_id) = command.metadata.get("app.id") {
            intent.app_id = app_id.clone();
        }
    }
    if intent.session_id.trim().is_empty() || intent.session_id == "${session.id}" {
        if let Some(session_id) = trace.session_id.as_ref() {
            intent.session_id = session_id.clone();
        } else if let Some(session_id) = command.metadata.get("session.id") {
            intent.session_id = session_id.clone();
        }
    }
    if intent.trace.is_none() {
        intent.trace = Some(trace.clone());
    }
}

fn is_supported_portal_import(import: &ApplicationImport) -> bool {
    matches!(
        import,
        ApplicationImport::ServiceCall
            | ApplicationImport::TraceEmit
            | ApplicationImport::UiRender
            | ApplicationImport::TaskCreateGoal
            | ApplicationImport::TaskQuery
            | ApplicationImport::AgentDelegate
    )
}

fn is_orchestration_import_name(import_name: &str) -> bool {
    import_name == ApplicationImport::TaskCreateGoal.as_name()
        || import_name == ApplicationImport::TaskQuery.as_name()
        || import_name == ApplicationImport::AgentDelegate.as_name()
}

fn has_non_empty_scope(metadata: &std::collections::BTreeMap<String, String>) -> bool {
    metadata
        .get("app.id")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        && metadata
            .get("session.id")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

fn default_service_for_import(import: &ApplicationImport) -> Option<&'static str> {
    match import {
        ApplicationImport::TaskCreateGoal | ApplicationImport::TaskQuery => Some(TASK_SERVICE_ID),
        ApplicationImport::AgentDelegate => Some(APPLICATION_SERVICE_ID),
        _ => None,
    }
}

fn default_operation_for_import(import: &ApplicationImport) -> Option<&'static str> {
    match import {
        ApplicationImport::TaskCreateGoal => Some(TASK_CREATE_GOAL_OPERATION),
        ApplicationImport::TaskQuery => Some(TASK_QUERY_OPERATION),
        ApplicationImport::AgentDelegate => Some(APPLICATION_AGENT_DELEGATE_COMMAND),
        _ => None,
    }
}

/// Convert compact guest orchestration payloads into typed service commands.
///
/// Declarative WASM metadata should not need to know host-owned app/session
/// identity or trace shapes.  The bridge acts as an Adapter at the ABI boundary:
/// guests declare provider-neutral work intent, while trusted host metadata
/// supplies application/session identity, trace, and command envelopes required
/// by Macaca services.  This keeps demo and third-party WASM applications honest:
/// they can use `macaca:task/*` and `macaca:agent/delegate` without bypassing the
/// OS task and agent execution services or hardcoding runtime internals.
fn hydrate_orchestration_service_payload(
    command: &WasmHostImportCommand,
    trace: &TraceContext,
) -> Value {
    let app_id = command.metadata.get("app.id").cloned().unwrap_or_default();
    let session_id = command
        .metadata
        .get("session.id")
        .cloned()
        .or_else(|| trace.session_id.clone())
        .unwrap_or_default();
    let agent_name = command
        .metadata
        .get("agent.name")
        .cloned()
        .unwrap_or_else(|| "wasm-guest".into());
    if command.import_name == ApplicationImport::TaskCreateGoal.as_name() {
        if command.payload.get("app_id").is_some() && command.payload.get("description").is_some() {
            return command.payload.clone();
        }
        return serde_json::json!({
            "app_id": app_id,
            "session_id": session_id,
            "description": command.payload.get("description")
                .or_else(|| command.payload.get("goal"))
                .or_else(|| command.payload.get("task"))
                .cloned()
                .unwrap_or(Value::Null),
            "trace": trace,
        });
    }
    if command.import_name == ApplicationImport::TaskQuery.as_name() {
        if command.payload.get("app_id").is_some() && command.payload.get("session_id").is_some() {
            return command.payload.clone();
        }
        return serde_json::json!({
            "app_id": app_id,
            "session_id": session_id,
            "trace": trace,
        });
    }
    if command.import_name != ApplicationImport::AgentDelegate.as_name() {
        return command.payload.clone();
    }
    if command.payload.get("trace").is_some() && command.payload.get("scope").is_some() {
        return command.payload.clone();
    }
    serde_json::json!({
        "trace": trace,
        "scope": {
            "application_id": app_id,
            "application_name": null,
            "session_id": session_id,
            "agent_name": agent_name
        },
        "target_agent": command.payload.get("target_agent").cloned().unwrap_or(Value::Null),
        "prompt": command.payload.get("prompt").cloned().unwrap_or(Value::Null),
        "context": command.payload.get("context").cloned().unwrap_or_else(|| serde_json::json!({})),
        "metadata": command.metadata
    })
}

fn task_lifecycle_payload(
    app_id: &str,
    session_id: &str,
    agent_name: &str,
    task_id: &str,
    trace: &TraceContext,
) -> Value {
    serde_json::json!({
        "app_id": app_id,
        "session_id": session_id,
        "agent_name": agent_name,
        "task_id": task_id,
        "trace": trace,
    })
}

fn extract_task_id_from_assignment(output: Value) -> Option<String> {
    output
        .get("task")
        .and_then(|task| task.get("id"))
        .and_then(extract_task_id_value)
}

fn extract_task_id_value(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    value
        .as_object()
        .and_then(|object| object.get("value").or_else(|| object.get("0")))
        .and_then(Value::as_str)
        .map(str::to_string)
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
