//! SDK helpers for Application ABI v0.
//!
//! This module is the developer-facing builder layer for application ABI
//! declarations and host commands.  It deliberately depends only on
//! `macaca-proto` contracts so application authors do not need `macaca-web`,
//! framework runner internals, or concrete provider clients to describe how
//! their application talks to Macaca OS.

use macaca_proto::{
    ApplicationAbiDeclaration, ApplicationAbiError, ApplicationExport, ApplicationHostCommand,
    ApplicationImport, PackageId, TraceContext,
};
use serde_json::Value;

/// Fluent builder for Application ABI declarations.
pub struct ApplicationAbiBuilder {
    declaration: ApplicationAbiDeclaration,
}

impl ApplicationAbiBuilder {
    /// Start a v0 declaration with all required exports and imports.
    pub fn v0(application_id: impl Into<String>) -> Self {
        Self {
            declaration: ApplicationAbiDeclaration::v0(application_id),
        }
    }

    /// Attach the package id that owns this application declaration.
    pub fn package_id(mut self, package_id: PackageId) -> Self {
        self.declaration.package_id = Some(package_id);
        self
    }

    /// Add an explicit permission name to the declaration.
    pub fn permission(mut self, permission: impl Into<String>) -> Self {
        self.declaration.permissions.push(permission.into());
        self
    }

    /// Add an optional future export while preserving all required exports.
    pub fn export(mut self, export: ApplicationExport) -> Self {
        if !self.declaration.exports.contains(&export) {
            self.declaration.exports.push(export);
        }
        self
    }

    /// Add an optional future import while preserving all v0 imports.
    pub fn import(mut self, import: ApplicationImport) -> Self {
        if !self.declaration.imports.contains(&import) {
            self.declaration.imports.push(import);
        }
        self
    }

    /// Add non-routing metadata for diagnostics and package tooling.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.declaration.metadata.insert(key.into(), value.into());
        self
    }

    /// Validate and return the declaration.
    pub fn build(self) -> Result<ApplicationAbiDeclaration, ApplicationAbiError> {
        self.declaration.validate_required_exports()?;
        Ok(self.declaration)
    }
}

/// Builder for traced host commands.
pub struct ApplicationHostCommandBuilder {
    import: ApplicationImport,
    payload: Value,
    trace: Option<TraceContext>,
}

impl ApplicationHostCommandBuilder {
    /// Start a command for one host import.
    pub fn new(import: ApplicationImport) -> Self {
        Self {
            import,
            payload: Value::Null,
            trace: None,
        }
    }

    /// Attach JSON payload.
    pub fn payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    /// Attach trace context required by host import governance.
    pub fn trace(mut self, trace: TraceContext) -> Self {
        self.trace = Some(trace);
        self
    }

    /// Build the command.  Missing trace stays representable at the protocol
    /// layer so tests and hosts can prove untraceable commands are rejected.
    pub fn build(self) -> ApplicationHostCommand {
        ApplicationHostCommand {
            import: self.import,
            payload: self.payload,
            trace: self.trace,
            metadata: Default::default(),
        }
    }
}

/// Convenience helper for `macaca:trace/emit`.
pub fn trace_emit_command(payload: Value, trace: TraceContext) -> ApplicationHostCommand {
    ApplicationHostCommandBuilder::new(ApplicationImport::TraceEmit)
        .payload(payload)
        .trace(trace)
        .build()
}

/// Convenience helper for `macaca:service/call`.
pub fn service_call_command(payload: Value, trace: TraceContext) -> ApplicationHostCommand {
    ApplicationHostCommandBuilder::new(ApplicationImport::ServiceCall)
        .payload(payload)
        .trace(trace)
        .build()
}

/// Build a WASM guest service-call import command.
///
/// This helper is a Proxy-facing SDK convenience: guest scaffolds describe the
/// desired service id, operation, bounded JSON payload, and capability label,
/// while the runtime host remains responsible for trace validation, capability
/// checks, policy, ServiceRuntime routing, and result redaction.  The function
/// deliberately writes only provider-neutral metadata keys so it does not bind
/// a guest application to a concrete service implementation or backend.
pub fn wasm_guest_service_call_command(
    service_id: impl Into<String>,
    operation: impl Into<String>,
    payload: Value,
    capability: impl Into<String>,
    trace: TraceContext,
) -> ApplicationHostCommand {
    let mut command = service_call_command(payload, trace);
    command
        .metadata
        .insert("service.id".into(), service_id.into().trim().to_string());
    command.metadata.insert(
        "service.operation".into(),
        operation.into().trim().to_string(),
    );
    command
        .metadata
        .insert("capability".into(), capability.into().trim().to_string());
    command
}

#[cfg(test)]
mod wasm_guest_import_contract_tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn wasm_guest_import_contract_builds_typed_service_call_metadata() {
        let command = wasm_guest_service_call_command(
            "service.runtime.fixture",
            "invoke",
            json!({"bounded": true}),
            "service.call",
            TraceContext::new("trace-wasm-guest-import"),
        );

        assert_eq!(command.import, ApplicationImport::ServiceCall);
        assert_eq!(
            command.metadata.get("service.id").map(String::as_str),
            Some("service.runtime.fixture")
        );
        assert_eq!(
            command
                .metadata
                .get("service.operation")
                .map(String::as_str),
            Some("invoke")
        );
        assert_eq!(
            command.metadata.get("capability").map(String::as_str),
            Some("service.call")
        );
        assert!(command.trace.is_some());
        assert_eq!(command.payload["bounded"], json!(true));
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn application_sdk_builder_creates_v0_declaration() {
        let declaration = ApplicationAbiBuilder::v0("app.sdk")
            .package_id(PackageId::new("pkg.sdk"))
            .permission("storage.scoped")
            .metadata("source", "sdk-test")
            .build()
            .unwrap();

        assert_eq!(declaration.application_id, "app.sdk");
        assert_eq!(declaration.package_id.as_ref().unwrap().as_str(), "pkg.sdk");
        assert!(declaration
            .imports
            .contains(&ApplicationImport::ServiceCall));
        declaration.validate_required_exports().unwrap();
    }

    #[test]
    fn application_sdk_builder_creates_traced_host_commands() {
        let trace = TraceContext::new("trace-sdk");
        let command = service_call_command(json!({"name": "service.test"}), trace.clone());
        let trace_command = trace_emit_command(json!({"event": "started"}), trace);

        assert_eq!(command.import, ApplicationImport::ServiceCall);
        assert!(command.trace.is_some());
        assert_eq!(trace_command.import, ApplicationImport::TraceEmit);
        assert!(trace_command.trace.is_some());
    }
}
