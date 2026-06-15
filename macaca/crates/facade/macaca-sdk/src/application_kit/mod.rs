//! Developer-facing ApplicationKit for Macaca Application Platform.
//!
//! ApplicationKit uses Builder and Facade patterns to create Manifest v1
//! contracts.  It returns protocol DTOs only; it never constructs `AppRuntime`,
//! `Kernel`, `ServiceRuntime`, Web state, or runtime-host providers.

pub mod wasm;
pub mod wasm_bindgen;

use macaca_proto::{
    ApplicationAbilityDescriptor, ApplicationCommerceDeclaration,
    ApplicationHostRequirementDeclaration, ApplicationManifestV1, ApplicationPermissionDeclaration,
    ApplicationPluginDependency, ApplicationRuntimeProfile, ApplicationUiDeclaration, DeveloperId,
    PackageId, PackageRuntimeKind,
};

/// Facade entry point for Manifest v1 construction.
#[derive(Debug, Default, Clone, Copy)]
pub struct ApplicationKit;

impl ApplicationKit {
    /// Start a Manifest v1 builder for provider-neutral applications.
    pub fn manifest(
        package_id: impl Into<String>,
        developer_id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> ApplicationManifestBuilder {
        ApplicationManifestBuilder::new(package_id, developer_id, name, version)
    }
}

pub use wasm::{WasmComponentApplicationDescriptor, WasmComponentApplicationScaffold};
pub use wasm_bindgen::{
    generate_wasm_guest_bindings, RustWasmBindgenBackend, WasmBindgenBackend,
    WasmBindgenDiagnostic, WasmBindgenInput, WasmBindgenOutput, WasmGuestBindingPlan,
    WasmMockHostImportBinding,
};

/// Fluent Manifest v1 builder for application authors.
pub struct ApplicationManifestBuilder {
    package_id: PackageId,
    developer_id: DeveloperId,
    name: String,
    version: String,
    runtime: ApplicationRuntimeProfile,
    host_requirements: ApplicationHostRequirementDeclaration,
    abilities: Vec<ApplicationAbilityDescriptor>,
    permissions: Vec<ApplicationPermissionDeclaration>,
    ui: Option<ApplicationUiDeclaration>,
    commerce: Option<ApplicationCommerceDeclaration>,
    plugin_dependencies: Vec<ApplicationPluginDependency>,
}

impl ApplicationManifestBuilder {
    pub fn new(
        package_id: impl Into<String>,
        developer_id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            package_id: PackageId::new(package_id),
            developer_id: DeveloperId::new(developer_id),
            name: name.into(),
            version: version.into(),
            runtime: ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
            host_requirements: ApplicationHostRequirementDeclaration::new("0.1.0"),
            abilities: Vec::new(),
            permissions: Vec::new(),
            ui: None,
            commerce: None,
            plugin_dependencies: Vec::new(),
        }
    }

    pub fn runtime(mut self, kind: PackageRuntimeKind, abi_version: impl Into<String>) -> Self {
        self.runtime = ApplicationRuntimeProfile::new(kind, abi_version);
        self
    }

    pub fn min_os_version(mut self, version: impl Into<String>) -> Self {
        self.host_requirements.min_os_version = version.into();
        self
    }

    pub fn ability(mut self, ability: ApplicationAbilityDescriptor) -> Self {
        self.abilities.push(ability);
        self
    }

    pub fn permission(mut self, name: impl Into<String>, reason: impl Into<String>) -> Self {
        self.permissions
            .push(ApplicationPermissionDeclaration::required(name, reason));
        self
    }

    pub fn genui(mut self, entry: impl Into<String>) -> Self {
        let mut ui = ApplicationUiDeclaration::genui();
        ui.entry = Some(entry.into());
        self.ui = Some(ui);
        self
    }

    pub fn commerce(mut self, commerce: ApplicationCommerceDeclaration) -> Self {
        self.commerce = Some(commerce);
        self
    }

    pub fn plugin_dependency(mut self, dependency: ApplicationPluginDependency) -> Self {
        self.plugin_dependencies.push(dependency);
        self
    }

    /// Build the manifest without executing or registering anything.
    pub fn build(self) -> ApplicationManifestV1 {
        let mut manifest = ApplicationManifestV1::new(
            self.package_id,
            self.developer_id,
            self.name,
            self.version,
            self.runtime,
            self.host_requirements,
        );
        for ability in self.abilities {
            manifest = manifest.ability(ability);
        }
        for permission in self.permissions {
            manifest = manifest.permission(permission);
        }
        if let Some(ui) = self.ui {
            manifest = manifest.ui(ui);
        }
        if let Some(commerce) = self.commerce {
            manifest = manifest.commerce(commerce);
        }
        for dependency in self.plugin_dependencies {
            manifest = manifest.plugin_dependency(dependency);
        }
        manifest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability_kit::AbilityKit;

    #[test]
    fn application_kit_builds_manifest() {
        let manifest = ApplicationKit::manifest(
            "application.fixture",
            "developer.fixture",
            "Fixture",
            "1.0.0",
        )
        .ability(AbilityKit::agent("ability.fixture.agent").build())
        .permission("trace.emit", "Fixture emits trace events")
        .build();

        assert_eq!(manifest.name, "Fixture");
        assert_eq!(manifest.abilities.len(), 1);
        assert_eq!(manifest.permissions.len(), 1);
    }
}
