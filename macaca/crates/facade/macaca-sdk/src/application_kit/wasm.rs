//! WASM Component Application SDK scaffold helpers.
//!
//! These helpers are intentionally metadata-only.  They teach application
//! authors how to declare a WASM component application's Manifest v1, ABI
//! imports/exports, permissions, and trace-required host commands without
//! executing guest code or constructing any runtime-host provider.

use macaca_proto::{
    ApplicationAbiDeclaration, ApplicationExport, ApplicationHostCommand, ApplicationImport,
    ApplicationManifestV1, PackageRuntimeKind, TraceContext,
};
use serde_json::json;

use crate::ability_kit::AbilityKit;
use crate::application::ApplicationHostCommandBuilder;
use crate::application_kit::ApplicationKit;

/// SDK-facing scaffold descriptor for a WASM component application.
///
/// The descriptor is safe to serialize in examples and tests because it holds
/// only bounded metadata: manifest fields, ABI declarations, and example host
/// commands with synthetic payloads.  It must not include raw WASM bytes,
/// prompts, raw manifests, secrets, environment values, or provider output.
#[derive(Debug, Clone, PartialEq)]
pub struct WasmComponentApplicationDescriptor {
    pub manifest: ApplicationManifestV1,
    pub abi: ApplicationAbiDeclaration,
    pub sample_commands: Vec<ApplicationHostCommand>,
    pub diagnostics: Vec<String>,
}

/// Builder-style scaffold for WASM component application fixtures.
#[derive(Debug, Clone)]
pub struct WasmComponentApplicationScaffold {
    package_id: String,
    developer_id: String,
    name: String,
    version: String,
    component_entry: String,
}

impl WasmComponentApplicationScaffold {
    /// Create a scaffold with stable package/developer identity metadata.
    pub fn new(
        package_id: impl Into<String>,
        developer_id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            package_id: package_id.into(),
            developer_id: developer_id.into(),
            name: name.into(),
            version: version.into(),
            component_entry: "component.wasm".into(),
        }
    }

    /// Override the component entry path used in metadata.
    ///
    /// The path remains metadata only.  The SDK never opens or executes it.
    pub fn component_entry(mut self, component_entry: impl Into<String>) -> Self {
        self.component_entry = component_entry.into();
        self
    }

    /// Build a contract-valid metadata fixture.
    ///
    /// The Builder pattern keeps fixture creation deterministic while the ABI
    /// declaration comes from `ApplicationAbiDeclaration::v0`, the canonical
    /// Rust DTO source.  Host commands include trace context so TestKit can
    /// validate trace invariants without invoking a host.
    pub fn build(self) -> WasmComponentApplicationDescriptor {
        let ability_id = format!("{}.component", self.package_id);
        let manifest = ApplicationKit::manifest(
            self.package_id.clone(),
            self.developer_id,
            self.name,
            self.version,
        )
        .runtime(PackageRuntimeKind::WasmComponent, "0")
        .ability(
            AbilityKit::wasm(ability_id, macaca_proto::ApplicationAbilityKind::Agent)
                .permission(
                    "trace.emit",
                    "WASM components must emit traceable host calls",
                )
                .permission(
                    "service.call",
                    "WASM components access host services only through traced ABI imports",
                )
                .metadata("component.entry", self.component_entry)
                .build(),
        )
        .permission(
            "trace.emit",
            "WASM application host calls are trace-required",
        )
        .permission(
            "service.call",
            "WASM application service access uses provider-neutral host imports",
        )
        .build();
        let mut abi = ApplicationAbiDeclaration::v0(self.package_id.clone());
        abi.permissions = manifest
            .permissions
            .iter()
            .map(|permission| permission.name.clone())
            .collect();
        abi.metadata.insert(
            "runtime.kind".into(),
            PackageRuntimeKind::WasmComponent.to_string(),
        );
        abi.metadata
            .insert("execution.status".into(), "runtime_unavailable".into());
        let sample_commands = vec![
            ApplicationHostCommandBuilder::new(ApplicationImport::TraceEmit)
                .payload(json!({"event": "wasm_scaffold_trace"}))
                .trace(TraceContext::new("sdk-wasm-scaffold-trace"))
                .build(),
            ApplicationHostCommandBuilder::new(ApplicationImport::ServiceCall)
                .payload(json!({"service": "application", "operation": "metadata"}))
                .trace(TraceContext::new("sdk-wasm-scaffold-service-call"))
                .build(),
        ];
        WasmComponentApplicationDescriptor {
            manifest,
            abi,
            sample_commands,
            diagnostics: vec![
                "wasm_component_execution_unavailable_until_runtime_module_is_installed".into(),
            ],
        }
    }
}

/// Return the canonical import labels used by WASM scaffold documentation.
pub fn wasm_scaffold_import_names() -> Vec<String> {
    ApplicationImport::v0()
        .into_iter()
        .map(|import| import.as_name().to_string())
        .collect()
}

/// Return the canonical export labels used by WASM scaffold documentation.
pub fn wasm_scaffold_export_names() -> Vec<String> {
    ApplicationExport::required_v0()
        .into_iter()
        .map(|export| export.as_name().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application_testkit::ApplicationContractTestKit;

    #[test]
    fn wasm_scaffold_builds_contract_valid_fixture() {
        let descriptor = WasmComponentApplicationScaffold::new(
            "fixture.application.wasm",
            "developer.fixture",
            "WASM Fixture",
            "1.0.0",
        )
        .build();
        let testkit = ApplicationContractTestKit;

        assert_eq!(
            descriptor.manifest.runtime.kind,
            PackageRuntimeKind::WasmComponent
        );
        assert!(testkit.validate_manifest(&descriptor.manifest).is_success());
        assert!(testkit
            .validate_abi_declaration(&descriptor.abi)
            .is_success());
        for command in &descriptor.sample_commands {
            assert!(testkit
                .validate_trace_required_command("fixture.application.wasm", command)
                .is_success());
        }
    }
}
