//! Application Manifest v1 contracts for Macaca Application Platform.
//!
//! This module is intentionally protocol-only.  It describes application
//! packages, runtime profiles, abilities, permissions, services, UI surfaces,
//! commerce metadata, plugin dependencies, and compatibility constraints as
//! serializable data.  Application execution, provider construction, policy
//! enforcement, and shell rendering live in higher layers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ApplicationAbilityDescriptor, DeveloperId, PackageId, PackageRuntimeKind, PackageType,
};

/// Version of the Application Manifest schema.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ApplicationManifestVersion(String);

impl ApplicationManifestVersion {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    pub fn v1() -> Self {
        Self::new("1")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Runtime profile selected from manifest data, not application names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationRuntimeProfile {
    pub kind: PackageRuntimeKind,
    pub abi_version: String,
    pub entry: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationRuntimeProfile {
    pub fn new(kind: PackageRuntimeKind, abi_version: impl Into<String>) -> Self {
        Self {
            kind,
            abi_version: abi_version.into(),
            entry: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn entry(mut self, entry: impl Into<String>) -> Self {
        self.entry = Some(entry.into());
        self
    }
}

/// Top-level permission declaration shared by abilities.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ApplicationPermissionDeclaration {
    pub name: String,
    pub reason: String,
    pub optional: bool,
}

impl ApplicationPermissionDeclaration {
    pub fn required(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            reason: reason.into(),
            optional: false,
        }
    }

    pub fn optional(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            reason: reason.into(),
            optional: true,
        }
    }
}

/// Declares a plugin dependency without binding to a concrete plugin runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationPluginDependency {
    pub plugin_id: String,
    pub version_req: Option<String>,
    pub optional: bool,
    pub reason: String,
}

impl ApplicationPluginDependency {
    pub fn required(plugin_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            version_req: None,
            optional: false,
            reason: reason.into(),
        }
    }
}

/// Commerce metadata used by Store/Entitlement services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationCommerceDeclaration {
    pub license: String,
    pub store_required: bool,
    pub metering: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationCommerceDeclaration {
    pub fn free() -> Self {
        Self {
            license: "free".into(),
            store_required: false,
            metering: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Compatibility constraints used by package guards and certification checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationCompatibilityDeclaration {
    pub min_os_version: String,
    pub sdk_version: Option<String>,
    pub features: Vec<String>,
}

impl ApplicationCompatibilityDeclaration {
    pub fn new(min_os_version: impl Into<String>) -> Self {
        Self {
            min_os_version: min_os_version.into(),
            sdk_version: None,
            features: Vec::new(),
        }
    }
}

/// Application-owned UI declaration.  Concrete rendering belongs to GenUI/UI
/// services and shells; this declaration only advertises safe package metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationUiDeclaration {
    pub kind: String,
    pub entry: Option<String>,
    pub surfaces: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationUiDeclaration {
    pub fn genui() -> Self {
        Self {
            kind: "genui".into(),
            entry: None,
            surfaces: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Manifest v1 is the new fact source for Application Platform packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationManifestV1 {
    pub manifest_version: ApplicationManifestVersion,
    pub package_id: PackageId,
    pub package_type: PackageType,
    pub developer_id: DeveloperId,
    pub name: String,
    pub version: String,
    pub runtime: ApplicationRuntimeProfile,
    pub abilities: Vec<ApplicationAbilityDescriptor>,
    pub permissions: Vec<ApplicationPermissionDeclaration>,
    pub ui: Option<ApplicationUiDeclaration>,
    pub commerce: Option<ApplicationCommerceDeclaration>,
    pub plugin_dependencies: Vec<ApplicationPluginDependency>,
    pub compatibility: ApplicationCompatibilityDeclaration,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationManifestV1 {
    /// Create a minimal application manifest.  Callers add abilities and
    /// declarations explicitly to keep the manifest reviewable and auditable.
    pub fn new(
        package_id: PackageId,
        developer_id: DeveloperId,
        name: impl Into<String>,
        version: impl Into<String>,
        runtime: ApplicationRuntimeProfile,
        compatibility: ApplicationCompatibilityDeclaration,
    ) -> Self {
        Self {
            manifest_version: ApplicationManifestVersion::v1(),
            package_id,
            package_type: PackageType::Application,
            developer_id,
            name: name.into(),
            version: version.into(),
            runtime,
            abilities: Vec::new(),
            permissions: Vec::new(),
            ui: None,
            commerce: None,
            plugin_dependencies: Vec::new(),
            compatibility,
            metadata: BTreeMap::new(),
        }
    }

    pub fn ability(mut self, ability: ApplicationAbilityDescriptor) -> Self {
        self.abilities.push(ability);
        self
    }

    pub fn permission(mut self, permission: ApplicationPermissionDeclaration) -> Self {
        self.permissions.push(permission);
        self
    }

    pub fn ui(mut self, ui: ApplicationUiDeclaration) -> Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AbilityImplementationKind, ApplicationAbilityKind};

    #[test]
    fn application_manifest_v1_roundtrips() {
        let manifest = ApplicationManifestV1::new(
            PackageId::new("application.fixture"),
            DeveloperId::new("developer.fixture"),
            "Fixture Application",
            "1.0.0",
            ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
            ApplicationCompatibilityDeclaration::new("0.1.0"),
        )
        .ability(ApplicationAbilityDescriptor::new(
            "ability.fixture.agent",
            ApplicationAbilityKind::Agent,
            AbilityImplementationKind::Declarative,
        ))
        .permission(ApplicationPermissionDeclaration::required(
            "trace.emit",
            "Fixture emits trace events",
        ));

        let encoded = serde_json::to_string(&manifest).unwrap();
        let decoded: ApplicationManifestV1 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, manifest);
    }
}
