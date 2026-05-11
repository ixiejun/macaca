//! Provider-neutral Application Ability descriptors for Macaca Application Platform.
//!
//! An ability is a component inside an application.  The model is intentionally
//! data-only so SDKs, package tools, runtime hosts, shells, and future remote
//! services can exchange the same declaration without importing application
//! runtime internals or provider implementations.  Runtime execution is owned by
//! Application Framework/host layers; this module only describes contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CapabilityId, KernelServiceId};

/// Stable ability categories supported by the first Application Platform slice.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationAbilityKind {
    Agent,
    Ui,
    Headless,
    Scheduled,
    Gateway,
    Extension,
}

/// Declares how an ability is implemented without binding to a concrete host.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbilityImplementationKind {
    Declarative,
    WasmComponent,
    BuiltIn,
    Remote,
    Hybrid,
}

/// Defines how an ability can be activated by the OS or a shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityActivation {
    pub mode: String,
    pub entry: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl AbilityActivation {
    /// Create an activation declaration.  The `mode` is string-backed so future
    /// activation protocols can be added without changing the wire format.
    pub fn new(mode: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            entry: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Attach a stable entry reference such as an agent, surface, or export.
    pub fn entry(mut self, entry: impl Into<String>) -> Self {
        self.entry = Some(entry.into());
        self
    }

    /// Attach safe routing metadata.  Callers must not store secrets here.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Lifecycle policy declared by an ability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityLifecyclePolicy {
    pub autostart: bool,
    pub restart: String,
    pub metadata: BTreeMap<String, String>,
}

impl Default for AbilityLifecyclePolicy {
    fn default() -> Self {
        Self {
            autostart: false,
            restart: "never".into(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Permission required by an ability.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AbilityPermissionDeclaration {
    pub name: String,
    pub reason: String,
    pub optional: bool,
}

impl AbilityPermissionDeclaration {
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

/// Service dependency required by an ability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityServiceRequirement {
    pub service: KernelServiceId,
    pub capability: Option<CapabilityId>,
    pub reason: String,
    pub optional: bool,
}

impl AbilityServiceRequirement {
    pub fn required(service: KernelServiceId, reason: impl Into<String>) -> Self {
        Self {
            service,
            capability: None,
            reason: reason.into(),
            optional: false,
        }
    }

    pub fn capability(mut self, capability: CapabilityId) -> Self {
        self.capability = Some(capability);
        self
    }

    pub fn optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }
}

/// Capability provided or consumed by an ability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityCapabilityDeclaration {
    pub id: CapabilityId,
    pub description: String,
    pub metadata: BTreeMap<String, String>,
}

impl AbilityCapabilityDeclaration {
    pub fn new(id: CapabilityId, description: impl Into<String>) -> Self {
        Self {
            id,
            description: description.into(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// UI surface declared by an ability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityUiSurfaceDeclaration {
    pub surface_id: String,
    pub schema: String,
    pub event_schema: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl AbilityUiSurfaceDeclaration {
    pub fn new(surface_id: impl Into<String>, schema: impl Into<String>) -> Self {
        Self {
            surface_id: surface_id.into(),
            schema: schema.into(),
            event_schema: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn event_schema(mut self, schema: impl Into<String>) -> Self {
        self.event_schema = Some(schema.into());
        self
    }
}

/// Composite ability descriptor owned by an application manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationAbilityDescriptor {
    pub id: String,
    pub kind: ApplicationAbilityKind,
    pub implementation: AbilityImplementationKind,
    pub activation: Vec<AbilityActivation>,
    pub lifecycle: AbilityLifecyclePolicy,
    pub permissions: Vec<AbilityPermissionDeclaration>,
    pub services: Vec<AbilityServiceRequirement>,
    pub capabilities: Vec<AbilityCapabilityDeclaration>,
    pub ui_surfaces: Vec<AbilityUiSurfaceDeclaration>,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationAbilityDescriptor {
    /// Create a descriptor with empty declarations.  Builders and adapters add
    /// optional declarations explicitly so the default state stays auditable.
    pub fn new(
        id: impl Into<String>,
        kind: ApplicationAbilityKind,
        implementation: AbilityImplementationKind,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            implementation,
            activation: Vec::new(),
            lifecycle: AbilityLifecyclePolicy::default(),
            permissions: Vec::new(),
            services: Vec::new(),
            capabilities: Vec::new(),
            ui_surfaces: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn activation(mut self, activation: AbilityActivation) -> Self {
        self.activation.push(activation);
        self
    }

    pub fn permission(mut self, permission: AbilityPermissionDeclaration) -> Self {
        self.permissions.push(permission);
        self
    }

    pub fn service(mut self, service: AbilityServiceRequirement) -> Self {
        self.services.push(service);
        self
    }

    pub fn capability(mut self, capability: AbilityCapabilityDeclaration) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn ui_surface(mut self, surface: AbilityUiSurfaceDeclaration) -> Self {
        self.ui_surfaces.push(surface);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_descriptor_roundtrips_all_minimum_kinds() {
        let kinds = [
            ApplicationAbilityKind::Agent,
            ApplicationAbilityKind::Ui,
            ApplicationAbilityKind::Headless,
            ApplicationAbilityKind::Scheduled,
            ApplicationAbilityKind::Gateway,
            ApplicationAbilityKind::Extension,
        ];

        for kind in kinds {
            let descriptor = ApplicationAbilityDescriptor::new(
                format!("ability.{kind:?}"),
                kind,
                AbilityImplementationKind::Declarative,
            );
            let encoded = serde_json::to_string(&descriptor).unwrap();
            let decoded: ApplicationAbilityDescriptor = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, descriptor);
        }
    }
}
