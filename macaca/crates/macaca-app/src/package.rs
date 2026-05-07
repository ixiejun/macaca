//! Package descriptor adapter for Macaca applications.
//!
//! This module adapts existing YAML application manifests into the canonical
//! Route C Package Manifest v0 contract.  It uses the Adapter + Builder
//! patterns so current `app.yaml` files remain first-class inputs while future
//! WASM/package loaders can consume the same descriptor shape.

use std::path::Path;

use macaca_proto::{
    CapabilityId, DeveloperId, KernelServiceId, MacacaResult, PackageCapability, PackageDescriptor,
    PackageEntry, PackageId, PackageManifest, PackagePermission, PackageRuntime,
    PackageRuntimeKind, PackageServiceRequirement, PackageType,
};
use macaca_sdk::AgentConfig;

use crate::loader::AppLoader;
use crate::model::{AgentSource, AppManifest, EntrypointType};

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
        let mut package = PackageManifest::new(
            PackageId::new(format!("application.{}", self.manifest.id)),
            PackageType::Application,
            self.manifest.version.clone(),
            DeveloperId::new("local.application"),
            PackageRuntime::new(PackageRuntimeKind::Yaml, "1"),
        );
        package.entry = app_entry(&self.manifest);
        package
            .metadata
            .insert("application.name".into(), self.manifest.name.clone());
        package
            .metadata
            .insert("application.id".into(), self.manifest.id.to_string());
        package
            .metadata
            .insert("agent.count".into(), self.manifest.agents.len().to_string());
        package.permissions = app_permissions(&self.resolved_agents);
        package.provides = app_capabilities(&self.manifest, &self.resolved_agents);
        package.required_services = app_required_services(&self.resolved_agents);
        PackageDescriptor::new(package)
    }
}

/// Convert a parsed YAML app manifest into a package descriptor.
pub fn app_manifest_to_package_descriptor(manifest: &AppManifest) -> PackageDescriptor {
    AppPackageDescriptorBuilder::new(manifest.clone()).build()
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

fn app_entry(manifest: &AppManifest) -> Option<PackageEntry> {
    if let Some(entry_agent) = &manifest.entry_agent {
        return Some(PackageEntry {
            kind: "agent".into(),
            value: entry_agent.clone(),
        });
    }
    manifest.entrypoint.as_ref().map(|entrypoint| PackageEntry {
        kind: match entrypoint.type_ {
            EntrypointType::Workflow => "workflow",
            EntrypointType::Agent => "agent",
            EntrypointType::Custom => "custom",
        }
        .into(),
        value: entrypoint.name.clone(),
    })
}

fn app_permissions(agents: &[AgentConfig]) -> Vec<PackagePermission> {
    let mut permissions = Vec::new();
    if agents.iter().any(|agent| agent.network_access) {
        permissions.push(PackagePermission {
            name: "network.access".into(),
            reason: "At least one application agent declares network access".into(),
        });
    }
    if agents.iter().any(|agent| !agent.allowed_paths.is_empty()) {
        permissions.push(PackagePermission {
            name: "filesystem.scoped".into(),
            reason: "At least one application agent declares allowed paths".into(),
        });
    }
    permissions
}

fn app_capabilities(manifest: &AppManifest, agents: &[AgentConfig]) -> Vec<PackageCapability> {
    let mut capabilities = Vec::new();
    for source in &manifest.agents {
        if let AgentSource::Inline(inline) = source {
            for capability in &inline.capabilities {
                capabilities.push(PackageCapability {
                    id: CapabilityId::new(format!("agent.{}.{}", inline.name, capability.name)),
                    description: capability.description.clone(),
                });
            }
            for tool in &inline.allowed_tools {
                capabilities.push(PackageCapability {
                    id: CapabilityId::new(format!("tool.{tool}")),
                    description: format!("Allowed tool declared by agent {}", inline.name),
                });
            }
        }
    }
    for agent in agents {
        for capability in &agent.capabilities {
            capabilities.push(PackageCapability {
                id: CapabilityId::new(format!("agent.{}.{}", agent.name, capability.name)),
                description: capability.description.clone(),
            });
        }
        for tool in &agent.allowed_tools {
            capabilities.push(PackageCapability {
                id: CapabilityId::new(format!("tool.{tool}")),
                description: format!("Allowed tool declared by agent {}", agent.name),
            });
        }
    }
    capabilities.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    capabilities.dedup_by(|a, b| a.id == b.id);
    capabilities
}

fn app_required_services(agents: &[AgentConfig]) -> Vec<PackageServiceRequirement> {
    if agents.is_empty() {
        return Vec::new();
    }
    vec![PackageServiceRequirement {
        service: KernelServiceId::new("service.agent.runtime"),
        capability: Some(CapabilityId::new("capability.agent.execute")),
        reason: "YAML application declares agents that need an agent runtime".into(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_app_paths() -> Vec<std::path::PathBuf> {
        let examples_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/apps");
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
