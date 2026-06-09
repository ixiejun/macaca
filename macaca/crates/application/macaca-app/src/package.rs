//! Package descriptor adapter for Macaca applications.
//!
//! This module adapts existing YAML application manifests into the canonical
//! Route C Package Manifest v0 contract.  It uses the Adapter + Builder
//! patterns so current `app.yaml` files remain first-class inputs while future
//! WASM/package loaders can consume the same descriptor shape.

use std::path::Path;

use macaca_proto::{
    ApplicationManifestV1, MacacaResult, PackageCapability, PackageDescriptor, PackageEntry,
    PackageManifest, PackagePermission, PackageRuntime, PackageServiceRequirement,
};
use macaca_proto::AgentConfig;

use crate::loader::AppLoader;
use crate::manifest_v1::YamlApplicationManifestAdapter;
use crate::model::AppManifest;

/// Builder for application package descriptors.
///
/// The builder keeps manifest normalization explicit instead of scattering
/// package-field assembly across loaders.  That makes later WASM, GUI, and
/// paid application adapters easier to add without changing existing callers.
pub struct AppPackageDescriptorBuilder {
    manifest: AppManifest,
    resolved_agents: Vec<AgentConfig>,
}

impl AppPackageDescriptorBuilder {
    /// Start from a parsed application manifest.
    pub fn new(manifest: AppManifest) -> Self {
        Self {
            manifest,
            resolved_agents: Vec::new(),
        }
    }

    /// Add resolved agent configurations when the caller has an app base path.
    pub fn with_resolved_agents(mut self, resolved_agents: Vec<AgentConfig>) -> Self {
        self.resolved_agents = resolved_agents;
        self
    }

    /// Build the canonical package descriptor.
    pub fn build(self) -> PackageDescriptor {
        let projection = YamlApplicationManifestAdapter::new(self.manifest)
            .with_resolved_agents(self.resolved_agents)
            .project();
        let mut descriptor = application_manifest_v1_to_package_descriptor(&projection.manifest);
        descriptor
            .trace_events
            .push("application_package.yaml_manifest_v1_projection.loaded".into());
        descriptor
    }
}

/// Convert a parsed YAML app manifest into a package descriptor.
#[deprecated(
    since = "0.1.0",
    note = "use YamlApplicationManifestAdapter plus application_manifest_v1_to_package_descriptor"
)]
pub fn app_manifest_to_package_descriptor(manifest: &AppManifest) -> PackageDescriptor {
    AppPackageDescriptorBuilder::new(manifest.clone()).build()
}

/// Convert Application Manifest v1 into the canonical Package Descriptor.
///
/// Package generation now depends on Manifest v1 facts instead of YAML-only
/// fields.  This keeps YAML first-class while making future manifest authors,
/// WASM packages, and generated SDK packages share the same package projection
/// contract.
pub fn application_manifest_v1_to_package_descriptor(
    manifest: &ApplicationManifestV1,
) -> PackageDescriptor {
    let mut package = PackageManifest::new(
        manifest.package_id.clone(),
        manifest.package_type.clone(),
        manifest.version.clone(),
        manifest.developer_id.clone(),
        PackageRuntime::new(
            manifest.runtime.kind.clone(),
            manifest.runtime.abi_version.clone(),
        ),
    );
    package.entry = manifest.runtime.entry.as_ref().map(|entry| PackageEntry {
        kind: manifest
            .runtime
            .metadata
            .get("entry.kind")
            .cloned()
            .unwrap_or_else(|| "agent".into()),
        value: entry.clone(),
    });
    package
        .metadata
        .insert("application.name".into(), manifest.name.clone());
    package.metadata.extend(manifest.metadata.clone());
    package.permissions = manifest
        .permissions
        .iter()
        .filter(|permission| !permission.optional)
        .map(|permission| PackagePermission {
            name: permission.name.clone(),
            reason: permission.reason.clone(),
        })
        .collect();
    package.required_services = manifest
        .abilities
        .iter()
        .flat_map(|ability| ability.services.iter())
        .filter(|service| !service.optional)
        .map(|service| PackageServiceRequirement {
            service: service.service.clone(),
            capability: service.capability.clone(),
            reason: service.reason.clone(),
        })
        .collect();
    package.provides = manifest
        .abilities
        .iter()
        .flat_map(|ability| ability.capabilities.iter())
        .map(|capability| PackageCapability {
            id: capability.id.clone(),
            description: capability.description.clone(),
        })
        .collect();
    package
        .provides
        .sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    package.provides.dedup_by(|a, b| a.id == b.id);
    PackageDescriptor::new(package)
}

/// Load a YAML app manifest and convert it into a package descriptor.
pub fn load_yaml_app_package_descriptor(path: impl AsRef<Path>) -> MacacaResult<PackageDescriptor> {
    let path = path.as_ref();
    let manifest = AppLoader::load_manifest(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let agents = AppLoader::resolve_agent_configs(&manifest, base_dir)?;
    Ok(AppPackageDescriptorBuilder::new(manifest)
        .with_resolved_agents(agents)
        .build())
}

#[cfg(test)]
mod tests {
    use macaca_proto::{PackageRuntimeKind, PackageType};

    use super::*;

    fn example_app_paths() -> Vec<std::path::PathBuf> {
        let examples_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/apps");
        let mut paths = std::fs::read_dir(examples_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("app.yaml"))
            .filter(|path| path.exists())
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    #[test]
    fn yaml_app_descriptor_preserves_application_metadata() {
        let descriptor =
            load_yaml_app_package_descriptor(example_app_paths().into_iter().next().unwrap())
                .unwrap();

        assert_eq!(descriptor.manifest.package_type, PackageType::Application);
        assert_eq!(
            descriptor.manifest.runtime.kind,
            Some(PackageRuntimeKind::Yaml)
        );
        assert!(descriptor.manifest.entry.is_some());
        assert!(descriptor
            .manifest
            .provides
            .iter()
            .any(|capability| capability.id.as_str().starts_with("tool.")));
    }

    #[test]
    fn yaml_app_descriptor_preserves_agent_capabilities() {
        let descriptors = example_app_paths()
            .into_iter()
            .map(load_yaml_app_package_descriptor)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(descriptors.iter().any(|descriptor| descriptor
            .manifest
            .provides
            .iter()
            .any(|capability| capability.id.as_str().contains("agent."))));
    }
}
