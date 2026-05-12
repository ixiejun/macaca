//! Application ABI adapter contracts.
//!
//! This module keeps application runtime metadata behind small traits and data
//! objects.  YAML applications, future WASM components, and future hybrid apps
//! can all implement the same adapter contract without exposing internal web
//! or framework state to application code.

use std::collections::BTreeMap;

use macaca_proto::{
    ApplicationAbiDeclaration, ApplicationAbiError, ApplicationCheckpoint, ApplicationExport,
    ApplicationHostCommandResult, ApplicationHostCommandStatus, ApplicationLifecycleState,
    PackageDescriptor, PackageRuntimeKind,
};
use tracing::{info, warn};

use crate::model::{AgentSource, AppManifest};
use crate::package::application_manifest_v1_to_package_descriptor;

/// Normalized Application ABI descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationAbiDescriptor {
    pub declaration: ApplicationAbiDeclaration,
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

/// YAML compatibility adapter.
pub struct YamlApplicationAbiAdapter {
    manifest: AppManifest,
    package: Option<PackageDescriptor>,
}

impl YamlApplicationAbiAdapter {
    /// Create an adapter from the parsed YAML application manifest.
    pub fn new(manifest: AppManifest) -> Self {
        Self {
            manifest,
            package: None,
        }
    }

    /// Attach the Phase 04 package descriptor when the caller already has one.
    pub fn with_package(mut self, package: PackageDescriptor) -> Self {
        self.package = Some(package);
        self
    }
}

impl ApplicationAbiAdapter for YamlApplicationAbiAdapter {
    fn load(&self) -> Result<ApplicationAbiLoadResult, ApplicationAbiError> {
        let projection =
            crate::manifest_v1::YamlApplicationManifestAdapter::new(self.manifest.clone())
                .project();
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
        let mut declaration = ApplicationAbiDeclaration::v0(application_id.clone());
        declaration.package_id = Some(projected_package.manifest.id.clone());
        declaration.permissions = projected_package
            .manifest
            .permissions
            .iter()
            .map(|permission| permission.name.clone())
            .collect();
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

/// Build an ABI descriptor from an app manifest without executing the app.
#[deprecated(
    since = "0.1.0",
    note = "use YamlApplicationManifestAdapter and YamlApplicationAbiAdapter for Manifest v1-backed descriptors"
)]
pub fn app_manifest_to_abi_descriptor(
    manifest: &AppManifest,
) -> Result<ApplicationAbiDescriptor, ApplicationAbiError> {
    Ok(YamlApplicationAbiAdapter::new(manifest.clone())
        .load()?
        .descriptor)
}

/// Report whether a host result is the expected Phase 05 WASM unavailable result.
pub fn is_runtime_unavailable(result: &ApplicationHostCommandResult) -> bool {
    matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    )
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        ApplicationImport, DeveloperId, PackageDescriptor, PackageId, PackageManifest,
        PackageRuntime, PackageRuntimeKind, PackageType,
    };

    use super::*;
    use crate::loader::AppLoader;

    fn first_example_app() -> std::path::PathBuf {
        let examples_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/apps");
        let mut paths = std::fs::read_dir(examples_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("app.yaml"))
            .filter(|path| path.exists())
            .collect::<Vec<_>>();
        paths.sort();
        paths.into_iter().next().unwrap()
    }

    #[test]
    fn application_abi_yaml_adapter_preserves_manifest_metadata() {
        let manifest = AppLoader::load_manifest(first_example_app()).unwrap();
        let descriptor = app_manifest_to_abi_descriptor(&manifest).unwrap();

        assert_eq!(descriptor.runtime_kind, Some(PackageRuntimeKind::Yaml));
        assert_eq!(
            descriptor
                .declaration
                .metadata
                .get("application.name")
                .unwrap(),
            &manifest.name
        );
        descriptor.declaration.validate_required_exports().unwrap();
        assert!(descriptor
            .declaration
            .imports
            .contains(&ApplicationImport::TaskCreateGoal));
    }

    #[test]
    fn application_abi_yaml_projection_preserves_key_fields() {
        let manifest = AppLoader::load_manifest(first_example_app()).unwrap();
        let load = YamlApplicationAbiAdapter::new(manifest.clone())
            .load()
            .unwrap();
        let descriptor = load.descriptor;

        assert_eq!(
            descriptor.declaration.application_id,
            manifest.id.to_string()
        );
        assert_eq!(descriptor.runtime_kind, Some(PackageRuntimeKind::Yaml));
        assert!(descriptor.declaration.package_id.is_some());
        assert_eq!(
            descriptor
                .declaration
                .metadata
                .get("manifest.version")
                .map(String::as_str),
            Some("1")
        );
        let expected_ability_count = manifest.agents.len().to_string();
        assert_eq!(
            descriptor.metadata.get("ability.count").map(String::as_str),
            Some(expected_ability_count.as_str())
        );
    }

    #[test]
    fn application_abi_wasm_adapter_loads_metadata_but_not_execution() {
        let package = PackageDescriptor::new(PackageManifest::new(
            PackageId::new("pkg.wasm"),
            PackageType::Application,
            "1.0.0",
            DeveloperId::new("dev.wasm"),
            PackageRuntime::new(PackageRuntimeKind::WasmComponent, "0"),
        ));
        let adapter = WasmApplicationAbiAdapter::new(package);
        let load = adapter.load().unwrap();
        let result = adapter.execute_unavailable();

        assert_eq!(
            load.descriptor.runtime_kind,
            Some(PackageRuntimeKind::WasmComponent)
        );
        assert!(is_runtime_unavailable(&result));
    }
}
