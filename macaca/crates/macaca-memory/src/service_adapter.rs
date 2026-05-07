//! System service descriptor adapter for the memory layer.
//!
//! Memory and context engines are replaceable strategy-backed services.  This
//! descriptor skeleton makes that boundary visible without changing current
//! memory runtime, vector backend, or governance behavior.

use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceSchemaRef,
};

/// Build the provider-neutral descriptor for the memory system service.
pub fn memory_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new("service.memory"),
        ServiceType::new("memory"),
        TraceSchemaRef::new("trace.system_service.memory.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            CapabilityId::new("capability.memory.recall"),
            "Recalls contextual memory through a service boundary.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.memory.write"),
            "Writes contextual memory through a service boundary.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.required_permissions = vec!["memory.access".into()];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_descriptor_exports_contract_shape() {
        let descriptor = memory_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "memory");
        assert_eq!(descriptor.capabilities.len(), 2);
        assert!(descriptor
            .required_permissions
            .contains(&"memory.access".into()));
    }
}
