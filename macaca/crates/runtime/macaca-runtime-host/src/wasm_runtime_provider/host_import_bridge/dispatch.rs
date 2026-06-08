//! Primary dispatch path for admitted WASM host imports.
//!
//! Dispatch implements the Command pattern: one Application ABI request becomes
//! either a local fast path (`trace.emit`, `ui.render`), a Task lifecycle side
//! effect (`agent_delegate`), or a ServiceRuntime route for all other imports.

use macaca_proto::{
    ApplicationHostCommand, ApplicationHostCommandResult, ApplicationImport, TraceContext,
    WasmHostImportAuditReport,
};
use tracing::info;

use crate::ServiceRouteRequest;

use super::routing_support::{
    bound_json, hydrate_orchestration_service_payload, is_safe_metadata_key, sanitize_json,
    sanitize_label,
};
use super::WasmHostImportBridge;

impl WasmHostImportBridge {
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
        self.emit_host_import_telemetry(
            "admitted",
            audit.trace_id.as_deref().unwrap_or("none"),
            &audit.reason_code,
            Some(&audit.import_name),
            audit.service_id.as_deref(),
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
                macaca_proto::WasmHostImportErrorKind::ServiceUnavailable,
                "WASM host import requires a target service",
            );
        };
        let Some(operation) = guest_command.operation.clone() else {
            return self.denied_result(
                &guest_command,
                macaca_proto::WasmHostImportErrorKind::UnsupportedImport,
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
                self.emit_host_import_telemetry(
                    "completed",
                    result
                        .trace
                        .as_ref()
                        .map(|value| value.trace_id.as_str())
                        .unwrap_or("none"),
                    "import_completed",
                    None,
                    Some(&service_id.to_string()),
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
}
