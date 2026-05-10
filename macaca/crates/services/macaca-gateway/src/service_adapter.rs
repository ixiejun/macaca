//! System service descriptor adapter for the gateway layer.
//!
//! Gateways connect external messaging surfaces to Agent OS.  The descriptor is
//! provider-neutral so concrete transports can be plugins or optional modules
//! without changing kernel contracts.

use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceSchemaRef,
};

/// Build the provider-neutral descriptor for the gateway system service.
pub fn gateway_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new("service.gateway"),
        ServiceType::new("gateway"),
        TraceSchemaRef::new("trace.system_service.gateway.v1"),
    );
    descriptor.capabilities = vec![ServiceCapability::new(
        CapabilityId::new("capability.gateway.message"),
        "Receives and sends external messages through a traced gateway boundary.",
    )];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec!["gateway.message".into()];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_descriptor_exports_contract_shape() {
        let descriptor = gateway_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "gateway");
        assert_eq!(descriptor.capabilities.len(), 1);
        assert!(descriptor
            .required_permissions
            .contains(&"gateway.message".into()));
    }
}
