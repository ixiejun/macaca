//! System service descriptor adapter for the skill layer.
//!
//! Skills are installable and policy-controlled capabilities.  This descriptor
//! skeleton exposes the skill service surface while leaving current skill
//! registry and runtime behavior untouched.

use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceSchemaRef,
};

/// Build the provider-neutral descriptor for the skill system service.
pub fn skill_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new("service.skill"),
        ServiceType::new("skill"),
        TraceSchemaRef::new("trace.system_service.skill.v1"),
    );
    descriptor.capabilities = vec![ServiceCapability::new(
        CapabilityId::new("capability.skill.invoke"),
        "Invokes installed skills through a traced and policy-aware boundary.",
    )];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec!["skill.invoke".into()];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_descriptor_exports_contract_shape() {
        let descriptor = skill_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "skill");
        assert_eq!(descriptor.capabilities.len(), 1);
        assert!(descriptor
            .required_permissions
            .contains(&"skill.invoke".into()));
    }
}
