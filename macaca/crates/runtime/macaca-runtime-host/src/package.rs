//! Runtime-host package compatibility hooks.
//!
//! Runtime-host owns MCP lifecycle and compatibility glue.  This module
//! exposes package-shaped requirement metadata without forcing MCP/plugin
//! runtime migration in Phase 04.

use macaca_proto::{
    CapabilityId, DeveloperId, KernelServiceId, PackageCapability, PackageDescriptor, PackageId,
    PackageManifest, PackageRuntime, PackageRuntimeKind, PackageServiceRequirement, PackageType,
};

/// Declarative package requirement for a runtime-host managed MCP surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHostPackageRequirement {
    pub package_id: String,
    pub service_id: String,
    pub capability_id: String,
}

impl RuntimeHostPackageRequirement {
    /// Create a package requirement with explicit ids.
    pub fn new(
        package_id: impl Into<String>,
        service_id: impl Into<String>,
        capability_id: impl Into<String>,
    ) -> Self {
        Self {
            package_id: package_id.into(),
            service_id: service_id.into(),
            capability_id: capability_id.into(),
        }
    }
}

/// Build a package descriptor for an MCP runtime-host integration point.
pub fn runtime_host_mcp_package_descriptor(
    requirement: &RuntimeHostPackageRequirement,
) -> PackageDescriptor {
    let mut manifest = PackageManifest::new(
        PackageId::new(&requirement.package_id),
        PackageType::Mcp,
        "1.0.0",
        DeveloperId::new("local.runtime-host"),
        PackageRuntime::new(PackageRuntimeKind::RemoteService, "1"),
    );
    manifest.required_services.push(PackageServiceRequirement {
        service: KernelServiceId::new(&requirement.service_id),
        capability: Some(CapabilityId::new(&requirement.capability_id)),
        reason: "Runtime-host MCP package requires a registered MCP service".into(),
    });
    manifest.provides.push(PackageCapability {
        id: CapabilityId::new(&requirement.capability_id),
        description: "Runtime-host MCP package capability".into(),
    });
    PackageDescriptor::new(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_host_requirement_maps_to_package_descriptor() {
        let requirement = RuntimeHostPackageRequirement::new(
            "mcp.browser",
            "service.mcp.browser",
            "capability.mcp.browser",
        );

        let descriptor = runtime_host_mcp_package_descriptor(&requirement);

        assert_eq!(descriptor.manifest.package_type, PackageType::Mcp);
        assert_eq!(descriptor.manifest.required_services.len(), 1);
    }
}
