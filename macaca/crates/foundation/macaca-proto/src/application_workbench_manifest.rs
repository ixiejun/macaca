//! Application-owned workbench capability declarations.
//!
//! This module is protocol-only.  It lets an application package declare the
//! generic workbench substrate it needs without making Macaca branch on app
//! names, product categories, or business workflows.  Admission specs and
//! service providers interpret these declarations through policy, entitlement,
//! sandbox, plugin, MCP, skill, and app-protocol boundaries before any side
//! effect runs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CapabilityId, KernelServiceId};

/// Complete workbench declaration carried by an application manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchManifestDeclaration {
    /// Capability families requested by the application, such as file,
    /// process, sandbox, approval, hook, diagnostics, or review.  These are
    /// generic families, not application-specific feature switches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<ApplicationWorkbenchCapabilityDeclaration>,
    /// Permission profile refs resolved by `service.sandbox` and policy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permission_profiles: Vec<String>,
    /// Tool families made visible to tool planning after admission.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_families: Vec<String>,
    /// Required or optional service dependencies for the declared substrate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub service_dependencies: Vec<ApplicationWorkbenchServiceDependency>,
    /// Optional provider categories that may return structured unavailable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_provider_requirements: Vec<ApplicationWorkbenchOptionalProviderRequirement>,
    /// Plugin dependencies needed by the application package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugin_dependencies: Vec<ApplicationWorkbenchPluginDependency>,
    /// MCP server dependencies requested by the application package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_dependencies: Vec<ApplicationWorkbenchMcpDependency>,
    /// Skill bundles requested by the application package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skill_bundles: Vec<ApplicationWorkbenchSkillBundleDependency>,
    /// Event topics the application wants to subscribe to through app protocol.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_subscriptions: Vec<ApplicationWorkbenchEventSubscription>,
    /// App-owned UI/GenUI surfaces associated with the workbench declaration.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ui_surfaces: Vec<ApplicationWorkbenchUiSurfaceDeclaration>,
    /// Bounded declaration metadata.  Do not store raw prompts or secrets here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationWorkbenchManifestDeclaration {
    /// Return whether the declaration carries no actionable workbench metadata.
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
            && self.permission_profiles.is_empty()
            && self.tool_families.is_empty()
            && self.service_dependencies.is_empty()
            && self.optional_provider_requirements.is_empty()
            && self.plugin_dependencies.is_empty()
            && self.mcp_dependencies.is_empty()
            && self.skill_bundles.is_empty()
            && self.event_subscriptions.is_empty()
            && self.ui_surfaces.is_empty()
            && self.metadata.is_empty()
    }
}

/// One generic workbench capability family requested by an application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchCapabilityDeclaration {
    pub family: String,
    #[serde(default)]
    pub optional: bool,
    pub reason: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationWorkbenchCapabilityDeclaration {
    /// Build a required capability-family declaration.
    pub fn required(family: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            optional: false,
            reason: reason.into(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Service dependency declared by a workbench application package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchServiceDependency {
    pub service: KernelServiceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<CapabilityId>,
    #[serde(default)]
    pub optional: bool,
    pub reason: String,
}

impl ApplicationWorkbenchServiceDependency {
    /// Build a required service dependency.
    pub fn required(service: KernelServiceId, reason: impl Into<String>) -> Self {
        Self {
            service,
            capability: None,
            optional: false,
            reason: reason.into(),
        }
    }

    /// Attach the service capability that satisfies this dependency.
    pub fn capability(mut self, capability: CapabilityId) -> Self {
        self.capability = Some(capability);
        self
    }
}

/// Optional provider category required for an enhanced application experience.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchOptionalProviderRequirement {
    pub provider_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<CapabilityId>,
    pub reason: String,
}

/// Plugin dependency declared at the workbench layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchPluginDependency {
    pub plugin_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_req: Option<String>,
    #[serde(default)]
    pub optional: bool,
    pub reason: String,
}

/// MCP dependency declared at the workbench layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchMcpDependency {
    pub server_id: String,
    #[serde(default = "default_session_scope")]
    pub lifecycle_scope: String,
    #[serde(default)]
    pub optional: bool,
    pub reason: String,
}

/// Skill bundle dependency declared at the workbench layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchSkillBundleDependency {
    pub bundle_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_req: Option<String>,
    #[serde(default)]
    pub optional: bool,
    pub reason: String,
}

/// Event subscription requested by an application package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchEventSubscription {
    pub topic: String,
    #[serde(default)]
    pub optional: bool,
    pub reason: String,
}

/// Application-owned UI or GenUI surface metadata for workbench experiences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationWorkbenchUiSurfaceDeclaration {
    pub surface_id: String,
    pub schema: String,
    #[serde(default = "default_workspace_mode")]
    pub mode: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_bridge_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_subscriptions: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

fn default_session_scope() -> String {
    "session".into()
}

fn default_workspace_mode() -> String {
    "workspace".into()
}
