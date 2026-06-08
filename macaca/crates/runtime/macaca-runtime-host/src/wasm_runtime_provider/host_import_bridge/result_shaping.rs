//! Guest-visible result shaping for denied, failed, and successful imports.
//!
//! This module centralizes metadata attachment and error-kind translation so
//! dispatch and lifecycle modules can return consistent, auditable outcomes
//! without duplicating status mapping logic.

use macaca_proto::{
    ApplicationHostCommandResult, ApplicationHostCommandStatus, WasmHostImportAuditReport,
    WasmHostImportCommand, WasmHostImportErrorKind,
};
use serde_json::Value;
use tracing::warn;

use crate::ServiceRuntimeError;

use super::routing_support::sanitize_label;
use super::WasmHostImportBridge;

impl WasmHostImportBridge {
    /// Build a policy- or validation-denied host command result with audit metadata.
    pub(super) fn denied_result(
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
        self.emit_host_import_telemetry(
            "denied",
            audit.trace_id.as_deref().unwrap_or("none"),
            &audit.reason_code,
            Some(&audit.import_name),
            None,
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

    /// Translate internal ServiceRuntime failures into guest-visible host results.
    pub(super) fn error_result(
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

    /// Attach stable import metadata that replay and shell consumers expect.
    pub(super) fn attach_common_metadata(
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
