//! Developer-facing AbilityKit for Macaca Application Platform.
//!
//! AbilityKit is a small Facade + Builder layer over provider-neutral proto
//! DTOs.  It does not construct application runtimes, kernels, service
//! runtimes, or provider implementations.  Its only job is to help application
//! authors describe abilities in a deterministic, auditable way.

use macaca_proto::{
    AbilityActivation, AbilityCapabilityDeclaration, AbilityImplementationKind,
    AbilityPermissionDeclaration, AbilityServiceRequirement, AbilityUiSurfaceDeclaration,
    ApplicationAbilityDescriptor, ApplicationAbilityKind, CapabilityId, KernelServiceId,
};

/// Facade entry point for ability descriptor construction.
#[derive(Debug, Default, Clone, Copy)]
pub struct AbilityKit;

impl AbilityKit {
    /// Create a builder for an Agent ability.
    pub fn agent(id: impl Into<String>) -> AbilityDescriptorBuilder {
        AbilityDescriptorBuilder::new(
            id,
            ApplicationAbilityKind::Agent,
            AbilityImplementationKind::Declarative,
        )
    }

    /// Create a builder for a UI ability.
    pub fn ui(id: impl Into<String>) -> AbilityDescriptorBuilder {
        AbilityDescriptorBuilder::new(
            id,
            ApplicationAbilityKind::Ui,
            AbilityImplementationKind::Declarative,
        )
    }

    /// Create a builder for a headless ability.
    pub fn headless(id: impl Into<String>) -> AbilityDescriptorBuilder {
        AbilityDescriptorBuilder::new(
            id,
            ApplicationAbilityKind::Headless,
            AbilityImplementationKind::Declarative,
        )
    }

    /// Create a builder for a WASM-backed ability.
    pub fn wasm(id: impl Into<String>, kind: ApplicationAbilityKind) -> AbilityDescriptorBuilder {
        AbilityDescriptorBuilder::new(id, kind, AbilityImplementationKind::WasmComponent)
    }
}

/// Fluent builder that assembles one ability descriptor.
pub struct AbilityDescriptorBuilder {
    descriptor: ApplicationAbilityDescriptor,
}

impl AbilityDescriptorBuilder {
    pub fn new(
        id: impl Into<String>,
        kind: ApplicationAbilityKind,
        implementation: AbilityImplementationKind,
    ) -> Self {
        Self {
            descriptor: ApplicationAbilityDescriptor::new(id, kind, implementation),
        }
    }

    pub fn activation(mut self, mode: impl Into<String>, entry: impl Into<String>) -> Self {
        self.descriptor = self
            .descriptor
            .activation(AbilityActivation::new(mode).entry(entry));
        self
    }

    pub fn permission(mut self, name: impl Into<String>, reason: impl Into<String>) -> Self {
        self.descriptor = self
            .descriptor
            .permission(AbilityPermissionDeclaration::required(name, reason));
        self
    }

    pub fn optional_permission(
        mut self,
        name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        self.descriptor = self
            .descriptor
            .permission(AbilityPermissionDeclaration::optional(name, reason));
        self
    }

    pub fn service(mut self, service: KernelServiceId, reason: impl Into<String>) -> Self {
        self.descriptor = self
            .descriptor
            .service(AbilityServiceRequirement::required(service, reason));
        self
    }

    pub fn capability(mut self, id: CapabilityId, description: impl Into<String>) -> Self {
        self.descriptor = self
            .descriptor
            .capability(AbilityCapabilityDeclaration::new(id, description));
        self
    }

    pub fn ui_surface(mut self, surface_id: impl Into<String>, schema: impl Into<String>) -> Self {
        self.descriptor = self
            .descriptor
            .ui_surface(AbilityUiSurfaceDeclaration::new(surface_id, schema));
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.descriptor = self.descriptor.metadata(key, value);
        self
    }

    /// Return the provider-neutral ability descriptor.
    pub fn build(self) -> ApplicationAbilityDescriptor {
        self.descriptor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_kit_builds_agent_ability() {
        let ability = AbilityKit::agent("ability.fixture.agent")
            .activation("session", "agent.fixture")
            .permission("trace.emit", "Agent emits trace events")
            .service(
                KernelServiceId::new("service.task"),
                "Agent creates task goals",
            )
            .capability(
                CapabilityId::new("capability.fixture.agent"),
                "Fixture agent capability",
            )
            .build();

        assert_eq!(ability.kind, ApplicationAbilityKind::Agent);
        assert_eq!(ability.permissions.len(), 1);
        assert_eq!(ability.services.len(), 1);
    }
}
