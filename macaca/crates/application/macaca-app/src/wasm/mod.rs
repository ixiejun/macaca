//! WASM Component Application descriptor skeleton.
//!
//! This module owns application-framework semantics for WASM component
//! packages: descriptor projection, metadata-only admission, and explicit
//! unavailable execution diagnostics.  It intentionally does not load bytes,
//! instantiate components, open transports, or call providers.  A future real
//! runtime can implement the same descriptor-facing contract behind a separate
//! OpenSpec proposal.

use std::collections::BTreeMap;

use macaca_proto::{
    ApplicationAbiDeclaration, ApplicationHostCommandResult, PackageDescriptor, PackageRuntimeKind,
};
use tracing::{info, warn};

use crate::abi::{ApplicationAbiAdapter, ApplicationAbiDescriptor, WasmApplicationAbiAdapter};
use crate::certification::WasmPackageAdmissionReport;

/// Metadata-only descriptor for one WASM component application package.
///
/// The descriptor is the Application Framework's safe view of a WASM package.
/// It carries package/runtime/ABI metadata and diagnostics, but it never stores
/// raw WASM bytes, raw host payloads, environment values, secrets, API keys, or
/// provider-specific handles.  This keeps package admission and certification
/// deterministic while the real runtime remains unavailable.
#[derive(Debug, Clone, PartialEq)]
pub struct WasmComponentApplicationDescriptor {
    pub abi: ApplicationAbiDescriptor,
    pub entry_component: Option<String>,
    pub diagnostics: Vec<String>,
    pub metadata: BTreeMap<String, String>,
    pub admission_report: WasmPackageAdmissionReport,
}

impl WasmComponentApplicationDescriptor {
    /// Build a descriptor from a package without executing component code.
    ///
    /// The method uses the existing `WasmApplicationAbiAdapter` as an Adapter
    /// from package metadata to Application ABI metadata.  It rejects non-WASM
    /// runtime kinds fail-closed because this module is only responsible for
    /// WASM component application descriptors.
    pub fn from_package(package: PackageDescriptor) -> Result<Self, String> {
        if package.manifest.runtime.kind != Some(PackageRuntimeKind::WasmComponent) {
            return Err("package runtime is not wasm_component".into());
        }
        let package_id = package.manifest.id.clone();
        let admission_report =
            WasmPackageAdmissionReport::metadata_only_unavailable(&package, None);
        let load = WasmApplicationAbiAdapter::new(package)
            .load()
            .map_err(|error| error.to_string())?;
        let entry_component = load.descriptor.entry.clone();
        let mut metadata = load.descriptor.metadata.clone();
        metadata.insert("execution.status".into(), "runtime_unavailable".into());
        metadata.insert(
            "runtime.kind".into(),
            PackageRuntimeKind::WasmComponent.to_string(),
        );
        info!(
            package_id = %package_id,
            entry_component = entry_component.as_deref().unwrap_or("none"),
            "WASM component application descriptor loaded as metadata-only"
        );
        Ok(Self {
            abi: load.descriptor,
            entry_component,
            diagnostics: vec![
                "wasm_component_runtime_unavailable_until_real_runtime_proposal".into(),
            ],
            metadata,
            admission_report,
        })
    }

    /// Return the ABI declaration used by SDK fixtures and host factories.
    pub fn declaration(&self) -> &ApplicationAbiDeclaration {
        &self.abi.declaration
    }

    /// Return structured unavailable execution for any attempted export.
    ///
    /// This Null Object behavior is intentionally explicit: metadata admission
    /// is allowed, but execution must fail safely with a runtime-unavailable
    /// status until the optional runtime module is implemented and approved.
    pub fn execute_unavailable(&self) -> ApplicationHostCommandResult {
        warn!(
            application_id = %self.abi.declaration.application_id,
            package_id = ?self.abi.declaration.package_id,
            "WASM component application execution requested before runtime exists"
        );
        let mut result = ApplicationHostCommandResult::runtime_unavailable(
            "WASM component application runtime is not installed",
            None,
        );
        result.metadata.insert(
            "reason_code".into(),
            "wasm_component_runtime_unavailable".into(),
        );
        result
    }
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        DeveloperId, PackageDescriptor, PackageEntry, PackageId, PackageManifest, PackageRuntime,
        PackageRuntimeKind, PackageType,
    };

    use super::*;
    use crate::abi::is_runtime_unavailable;

    fn wasm_package() -> PackageDescriptor {
        let mut descriptor = PackageDescriptor::new(PackageManifest::new(
            PackageId::new("fixture.application.wasm"),
            PackageType::Application,
            "1.0.0",
            DeveloperId::new("developer.fixture"),
            PackageRuntime::new(PackageRuntimeKind::WasmComponent, "0"),
        ));
        descriptor.manifest.entry = Some(PackageEntry {
            kind: "component".into(),
            value: "component.wasm".into(),
        });
        descriptor
            .manifest
            .metadata
            .insert("application.id".into(), "fixture.application.wasm".into());
        descriptor
    }

    #[test]
    fn wasm_descriptor_loads_metadata_without_execution() {
        let descriptor = WasmComponentApplicationDescriptor::from_package(wasm_package()).unwrap();

        assert_eq!(
            descriptor.abi.runtime_kind,
            Some(PackageRuntimeKind::WasmComponent)
        );
        assert_eq!(
            descriptor.entry_component.as_deref(),
            Some("component.wasm")
        );
        assert!(descriptor
            .diagnostics
            .contains(&"wasm_component_runtime_unavailable_until_real_runtime_proposal".into()));
        assert!(descriptor
            .admission_report
            .reason_codes
            .contains(&"metadata_only_runtime_unavailable".into()));
    }

    #[test]
    fn wasm_descriptor_execution_is_runtime_unavailable() {
        let descriptor = WasmComponentApplicationDescriptor::from_package(wasm_package()).unwrap();
        let result = descriptor.execute_unavailable();

        assert!(is_runtime_unavailable(&result));
        assert_eq!(
            result.metadata.get("reason_code").map(String::as_str),
            Some("wasm_component_runtime_unavailable")
        );
    }
}
