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
    workbench::sandbox::SandboxRuntimeKind, ApplicationAbilityDescriptor, DeveloperId,
    ExecutionControlPolicy, PackageId, PackageRuntimeKind, PackageType,
};
use crate::{ApplicationExecutionProfileDeclaration, ApplicationWorkbenchManifestDeclaration};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_control: Option<ExecutionControlPolicy>,
    /// Provider-neutral application execution profile.
    ///
    /// This declaration is data-only manifest policy. Runtime-host may adapt it
    /// into a provider descriptor after admission, but the manifest never owns
    /// provider lifecycle, leases, EventLog persistence, or transport effects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_profile: Option<ApplicationExecutionProfileDeclaration>,
    /// Generic interactive workbench declarations owned by Application
    /// Framework.  Services and shells consume this through admission and
    /// sanitized projections; the OS must never infer these capabilities from
    /// an application name or product workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workbench: Option<ApplicationWorkbenchManifestDeclaration>,
    /// Abstract tool families requested by the application.
    ///
    /// This is data-only manifest policy. Planning services interpret these
    /// strings through generic family/toolset rules; OS code must not branch on
    /// package names or business domains.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_families: Vec<String>,
    /// Declarative toolsets requested by the application.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub toolsets: Vec<String>,
    /// Backward-compatible exact tool allowlist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    /// Permission profiles requested by the application for workbench calls.
    ///
    /// Admission and sandbox services resolve these profile refs through
    /// `service.sandbox`; the manifest only declares intent and never names a
    /// concrete provider or application workflow.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permission_profiles: Vec<String>,
    /// Runtime environment categories the application may request.
    ///
    /// These categories are provider-neutral.  Optional providers such as
    /// Docker, SSH, browser, WASM, or remote environments can be absent and
    /// still produce structured unavailable states through `service.sandbox`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sandbox_runtime_kinds: Vec<SandboxRuntimeKind>,
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
            execution_control: None,
            execution_profile: None,
            workbench: None,
            tool_families: Vec::new(),
            toolsets: Vec::new(),
            allowed_tools: Vec::new(),
            permission_profiles: Vec::new(),
            sandbox_runtime_kinds: Vec::new(),
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

    pub fn execution_control(mut self, policy: ExecutionControlPolicy) -> Self {
        self.execution_control = Some(policy);
        self
    }

    pub fn execution_profile(mut self, profile: ApplicationExecutionProfileDeclaration) -> Self {
        self.execution_profile = Some(profile);
        self
    }

    pub fn workbench(mut self, declaration: ApplicationWorkbenchManifestDeclaration) -> Self {
        self.workbench = Some(declaration);
        self
    }

    pub fn tool_family(mut self, family: impl Into<String>) -> Self {
        self.tool_families.push(family.into());
        self
    }

    pub fn toolset(mut self, toolset: impl Into<String>) -> Self {
        self.toolsets.push(toolset.into());
        self
    }

    pub fn allowed_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.allowed_tools.push(tool_name.into());
        self
    }

    pub fn permission_profile(mut self, profile_ref: impl Into<String>) -> Self {
        self.permission_profiles.push(profile_ref.into());
        self
    }

    pub fn sandbox_runtime_kind(mut self, kind: SandboxRuntimeKind) -> Self {
        self.sandbox_runtime_kinds.push(kind);
        self
    }
}

/// Contract tests for Application Manifest v1 serde round-trips (extracted for file-size gate).
#[cfg(test)]
mod tests;
