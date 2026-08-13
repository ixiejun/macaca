//! Application ABI adapter contracts.
//!
//! This module keeps application runtime metadata behind small traits and data
//! objects.  YAML applications, future WASM components, and future hybrid apps
//! can all implement the same adapter contract without exposing internal web
//! or framework state to application code.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_proto::{
    expand_service_capabilities, ApplicationAbiDeclaration, ApplicationAbiError,
    ApplicationCheckpoint, ApplicationExport, ApplicationHostCommandResult,
    ApplicationHostCommandStatus, ApplicationLifecycleState, DomainPackCatalog,
    EffectiveServiceCapabilities, InMemoryDomainPackCatalog, PackageDescriptor, PackageRuntimeKind,
};
use tracing::{info, warn};

use crate::model::{AgentSource, AppManifest};
use crate::package::application_manifest_v1_to_package_descriptor;

/// Normalized Application ABI descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationAbiDescriptor {
    pub declaration: ApplicationAbiDeclaration,
    /// Sanitized, descriptor-owned service capabilities visible to the application.
    ///
    /// This Memento is expanded from the application declaration and catalog only.
    /// It intentionally contains command schemas, granted scopes, unavailable
    /// diagnostics, and replay references, never provider instances or raw data.
    pub service_capabilities: EffectiveServiceCapabilities,
    pub package: Option<PackageDescriptor>,
    pub runtime_kind: Option<PackageRuntimeKind>,
    pub entry: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Result returned by ABI metadata loaders.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationAbiLoadResult {
    pub descriptor: ApplicationAbiDescriptor,
    pub trace_events: Vec<String>,
}

/// Runtime-facing ABI instance contract.
pub trait ApplicationAbiInstance: Send + Sync {
    /// Return the immutable ABI descriptor for this instance.
    fn descriptor(&self) -> &ApplicationAbiDescriptor;

    /// Execute an export if the runtime supports it.
    fn execute_export(
        &self,
        export: ApplicationExport,
        _input: serde_json::Value,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        Err(ApplicationAbiError::UnsupportedExport(export.to_string()))
    }

    /// Produce a portable checkpoint for pause/resume/upgrade flows.
    fn checkpoint(&self) -> ApplicationCheckpoint {
        ApplicationCheckpoint::new(
            self.descriptor().declaration.application_id.clone(),
            ApplicationLifecycleState::Paused,
            serde_json::Value::Null,
        )
    }
}

/// Adapter contract for runtime-specific application metadata.
pub trait ApplicationAbiAdapter: Send + Sync {
    /// Convert runtime-specific metadata into a normalized ABI descriptor.
    fn load(&self) -> Result<ApplicationAbiLoadResult, ApplicationAbiError>;
}

/// In-memory ABI instance used for metadata-only runtimes.
#[derive(Debug, Clone)]
pub struct MetadataOnlyApplicationAbiInstance {
    descriptor: ApplicationAbiDescriptor,
}

impl MetadataOnlyApplicationAbiInstance {
    /// Create a metadata-only ABI instance.
    pub fn new(descriptor: ApplicationAbiDescriptor) -> Self {
        Self { descriptor }
    }
}

impl ApplicationAbiInstance for MetadataOnlyApplicationAbiInstance {
    fn descriptor(&self) -> &ApplicationAbiDescriptor {
        &self.descriptor
    }
}

/// YAML Application ABI adapter.
pub struct YamlApplicationAbiAdapter {
    manifest: AppManifest,
    package: Option<PackageDescriptor>,
    catalog: Option<Arc<dyn DomainPackCatalog>>,
}

impl YamlApplicationAbiAdapter {
    /// Create an adapter from the parsed YAML application manifest.
    pub fn new(manifest: AppManifest) -> Self {
        Self {
            manifest,
            package: None,
            catalog: None,
        }
    }

    /// Attach the Phase 04 package descriptor when the caller already has one.
    pub fn with_package(mut self, package: PackageDescriptor) -> Self {
        self.package = Some(package);
        self
    }

    /// Inject the host-installed catalog used for descriptor-only discovery.
    ///
    /// The catalog remains an abstract Strategy boundary, allowing hosts to
    /// expose installed, remote, mock, or unavailable pack descriptors without
    /// making the ABI adapter aware of any concrete provider implementation.
    pub fn with_catalog(mut self, catalog: Arc<dyn DomainPackCatalog>) -> Self {
        self.catalog = Some(catalog);
        self
    }
}

impl ApplicationAbiAdapter for YamlApplicationAbiAdapter {
    fn load(&self) -> Result<ApplicationAbiLoadResult, ApplicationAbiError> {
        let default_catalog = InMemoryDomainPackCatalog::with_builtin_defaults();
        let catalog = self.catalog.as_deref().unwrap_or(&default_catalog);
        let projection =
            crate::manifest_v1::YamlApplicationManifestAdapter::new(self.manifest.clone())
                .project_with_catalog(catalog);
        let projected_package = self
            .package
            .clone()
            .unwrap_or_else(|| application_manifest_v1_to_package_descriptor(&projection.manifest));
        let application_id = projection
            .manifest
            .metadata
            .get("application.id")
            .cloned()
            .unwrap_or_else(|| self.manifest.id.to_string());
        // Keep service discovery at the ABI boundary data-only. The catalog owns
        // pack resolution, so this adapter neither constructs providers nor embeds
        // pack-specific routing rules in application-framework code.
        let service_capabilities =
            expand_service_capabilities(self.manifest.service_contract.as_ref(), catalog);
        let mut declaration = ApplicationAbiDeclaration::v0(application_id.clone());
        declaration.package_id = Some(projected_package.manifest.id.clone());
        declaration.permissions = projected_package
            .manifest
            .permissions
            .iter()
            .map(|permission| permission.name.clone())
            .collect();
        declaration.permissions.extend(
            service_capabilities
                .granted_pack_permission_scopes
                .values()
                .flatten()
                .cloned(),
        );
        declaration.permissions.sort();
        declaration.permissions.dedup();
        declaration
            .metadata
            .insert("application.name".into(), projection.manifest.name.clone());
        declaration.metadata.insert(
            "application.version".into(),
            projection.manifest.version.clone(),
        );
        declaration
            .metadata
            .insert("runtime.adapter".into(), "yaml".into());
        declaration
            .metadata
            .insert("manifest.version".into(), "1".into());
        declaration.metadata.insert(
            "ability.count".into(),
            projection.manifest.abilities.len().to_string(),
        );
        declaration.metadata.insert(
            "service.capabilities.hash".into(),
            service_capabilities.capabilities_hash.clone(),
        );
        declaration.metadata.insert(
            "service.capabilities.count".into(),
            service_capabilities.services.len().to_string(),
        );
        declaration.metadata.insert(
            "service.unavailable_pack_count".into(),
            service_capabilities
                .unavailable_pack_reasons
                .len()
                .to_string(),
        );

        let entry = projection.manifest.runtime.entry.clone();
        let mut metadata = BTreeMap::new();
        metadata.insert("agent.count".into(), self.manifest.agents.len().to_string());
        metadata.insert(
            "inline.agent.count".into(),
            self.manifest
                .agents
                .iter()
                .filter(|source| matches!(source, AgentSource::Inline(_)))
                .count()
                .to_string(),
        );
        if let Some(entry) = &entry {
            metadata.insert("entry".into(), entry.clone());
        }
        metadata.insert(
            "manifest.version".into(),
            projection.manifest.manifest_version.as_str().into(),
        );
        metadata.insert(
            "ability.count".into(),
            projection.manifest.abilities.len().to_string(),
        );

        info!(
            application_id = %application_id,
            package_id = %projected_package.manifest.id,
            ability_count = projection.manifest.abilities.len(),
            "YAML application projected through Manifest v1 for Application ABI v0 descriptor"
        );
        Ok(ApplicationAbiLoadResult {
            descriptor: ApplicationAbiDescriptor {
                declaration,
                service_capabilities,
                package: Some(projected_package),
                runtime_kind: Some(PackageRuntimeKind::Yaml),
                entry,
                metadata,
            },
            trace_events: vec![
                "application_abi.yaml_manifest_v1_projection.loaded".into(),
                "application_abi.yaml_adapter.loaded".into(),
            ],
        })
    }
}

/// WASM metadata-only adapter for Phase 05.
pub struct WasmApplicationAbiAdapter {
    package: PackageDescriptor,
}

impl WasmApplicationAbiAdapter {
    /// Create a metadata-only WASM adapter from a guarded package descriptor.
    pub fn new(package: PackageDescriptor) -> Self {
        Self { package }
    }

    /// Return the explicit Phase 05 runtime-unavailable execution result.
    pub fn execute_unavailable(&self) -> ApplicationHostCommandResult {
        warn!(
            package_id = %self.package.manifest.id,
            "WASM Application ABI execution requested before runtime exists"
        );
        ApplicationHostCommandResult::runtime_unavailable(
            "WASM Application ABI execution is intentionally unavailable in Phase 05",
            None,
        )
    }
}

impl ApplicationAbiAdapter for WasmApplicationAbiAdapter {
    fn load(&self) -> Result<ApplicationAbiLoadResult, ApplicationAbiError> {
        let application_id = self
            .package
            .manifest
            .metadata
            .get("application.id")
            .cloned()
            .unwrap_or_else(|| self.package.manifest.id.to_string());
        let declaration = ApplicationAbiDeclaration::v0(application_id)
            .with_package_id(self.package.manifest.id.clone());

        info!(
            package_id = %self.package.manifest.id,
            "WASM Application ABI metadata loaded without execution"
        );
        Ok(ApplicationAbiLoadResult {
            descriptor: ApplicationAbiDescriptor {
                declaration,
                service_capabilities: EffectiveServiceCapabilities::default(),
                package: Some(self.package.clone()),
                runtime_kind: Some(PackageRuntimeKind::WasmComponent),
                entry: self
                    .package
                    .manifest
                    .entry
                    .as_ref()
                    .map(|entry| entry.value.clone()),
                metadata: self.package.manifest.metadata.clone(),
            },
            trace_events: vec!["application_abi.wasm_metadata.loaded".into()],
        })
    }
}

/// Report whether a host result is the expected Phase 05 WASM unavailable result.
pub fn is_runtime_unavailable(result: &ApplicationHostCommandResult) -> bool {
    matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    )
}

#[cfg(test)]
#[path = "abi_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "abi_calendar_tests.rs"]
mod calendar_tests;

#[cfg(test)]
#[path = "abi_email_tests.rs"]
mod email_tests;

#[cfg(test)]
#[path = "abi_messaging_tests.rs"]
mod messaging_tests;

#[cfg(test)]
#[path = "abi_random_tests.rs"]
mod random_tests;
