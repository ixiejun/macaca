//! System service descriptor adapter for the driver layer.
//!
//! Drivers are replaceable service/plugin capabilities.  This descriptor keeps
//! the kernel aware of the driver service surface without naming any concrete
//! driver implementation or changing runtime driver execution.

use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceSchemaRef,
};

/// Build the provider-neutral descriptor for the driver system service.
pub fn driver_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new("service.driver"),
        ServiceType::new("driver"),
        TraceSchemaRef::new("trace.system_service.driver.v1"),
    );
    descriptor.capabilities = vec![ServiceCapability::new(
        CapabilityId::new("capability.driver.execute"),
        "Executes external software operations through a traced driver boundary.",
    )];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.required_permissions = vec!["driver.execute".into()];
    descriptor.cleanup_policy = CleanupPolicy::Always;
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_descriptor_exports_contract_shape() {
        let descriptor = driver_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "driver");
        assert_eq!(descriptor.capabilities.len(), 1);
        assert!(descriptor
            .required_permissions
            .contains(&"driver.execute".into()));
    }
}
