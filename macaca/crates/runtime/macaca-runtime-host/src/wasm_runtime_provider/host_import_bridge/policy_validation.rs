//! Admission validation and app-scoped policy override installation.
//!
//! Validation runs before any service routing occurs. Policy overrides are
//! interpreted only from trusted host metadata and never from guest payloads,
//! preserving the OS boundary between application manifests and WASM guests.

use macaca_proto::{
    ApplicationHostCommand, ApplicationImport, TraceContext, WasmHostImportCategory,
    WasmHostImportCommand, WasmHostImportErrorKind,
};
use tracing::info;

use crate::ServicePolicyLayer;

use super::routing_support::{
    command_shell, has_non_empty_scope, is_orchestration_import_name, is_supported_portal_import,
    parse_csv_services,
};
use super::WasmHostImportBridge;

impl WasmHostImportBridge {
    /// Validate one Application ABI command before ServiceRuntime dispatch.
    pub(super) fn validate(
        &self,
        command: ApplicationHostCommand,
        trace: TraceContext,
    ) -> Result<WasmHostImportCommand, macaca_proto::ApplicationHostCommandResult> {
        let import_name = command.import.as_name().to_string();
        let category = WasmHostImportCategory::from_application_import(&command.import);
        if !is_supported_portal_import(&command.import) {
            let guest_command = command_shell(command, trace, category, import_name, 0);
            return Err(self.denied_result(
                &guest_command,
                WasmHostImportErrorKind::UnsupportedImport,
                "WASM host import is not supported by this bridge",
            ));
        }
        let payload_bytes = serde_json::to_vec(&command.payload)
            .map(|payload| payload.len() as u64)
            .unwrap_or(u64::MAX);
        let guest_command = command_shell(command, trace, category, import_name, payload_bytes);
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
    pub(super) fn apply_policy_overrides(&self, command: &WasmHostImportCommand) {
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
}
