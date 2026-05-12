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

    /// Create a builder for a scheduled ability.
    ///
    /// Scheduled abilities are still declarative contracts at the SDK layer.
    /// The SDK records the ability kind and leaves timer interpretation to the
    /// Application Framework admission/runtime path so application packages do
    /// not depend on any concrete scheduler implementation.
    pub fn scheduled(id: impl Into<String>) -> AbilityDescriptorBuilder {
        AbilityDescriptorBuilder::new(
            id,
            ApplicationAbilityKind::Scheduled,
            AbilityImplementationKind::Declarative,
        )
    }

    /// Create a builder for a gateway ability.
    ///
    /// Gateway abilities describe an application-owned ingress contract.  The
    /// descriptor is intentionally provider-neutral: shells and gateway hosts
    /// can route by declared services/capabilities without the SDK knowing the
    /// gateway implementation type.
    pub fn gateway(id: impl Into<String>) -> AbilityDescriptorBuilder {
        AbilityDescriptorBuilder::new(
            id,
            ApplicationAbilityKind::Gateway,
            AbilityImplementationKind::Declarative,
        )
    }

    /// Create a builder for an extension ability.
    ///
    /// Extension abilities let an application add integration points without
    /// hardcoding a specific Plugin, MCP, Driver, Skill, or provider backend in
    /// the SDK.  Concrete wiring is resolved later by service/capability
    /// registries under policy and trace governance.
    pub fn extension(id: impl Into<String>) -> AbilityDescriptorBuilder {
        AbilityDescriptorBuilder::new(
            id,
            ApplicationAbilityKind::Extension,
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

    #[test]
    fn ability_kit_builds_all_first_class_ability_kinds() {
        let scheduled = AbilityKit::scheduled("ability.fixture.scheduled").build();
        let gateway = AbilityKit::gateway("ability.fixture.gateway").build();
        let extension = AbilityKit::extension("ability.fixture.extension").build();

        assert_eq!(scheduled.kind, ApplicationAbilityKind::Scheduled);
        assert_eq!(gateway.kind, ApplicationAbilityKind::Gateway);
        assert_eq!(extension.kind, ApplicationAbilityKind::Extension);
        assert_eq!(
            scheduled.implementation,
            AbilityImplementationKind::Declarative
        );
        assert_eq!(
            gateway.implementation,
            AbilityImplementationKind::Declarative
        );
        assert_eq!(
            extension.implementation,
            AbilityImplementationKind::Declarative
        );
    }
}
