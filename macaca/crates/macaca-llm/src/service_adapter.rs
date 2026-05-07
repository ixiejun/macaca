//! System service descriptor adapter for the LLM layer.
//!
//! This adapter is a descriptor skeleton only.  It lets Route C service
//! discovery see the LLM capability surface without changing how existing LLM
//! providers are selected or called.

use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceSchemaRef,
};

/// Build the provider-neutral descriptor for the LLM system service.
pub fn llm_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new("service.llm"),
        ServiceType::new("llm"),
        TraceSchemaRef::new("trace.system_service.llm.v1"),
    );
    descriptor.capabilities = vec![ServiceCapability::new(
        CapabilityId::new("capability.llm.chat"),
        "Runs model-backed conversational completion through the LLM service boundary.",
    )];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec!["llm.call".into()];
    descriptor.cleanup_policy = CleanupPolicy::None;
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_descriptor_exports_contract_shape() {
        let descriptor = llm_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "llm");
        assert_eq!(descriptor.capabilities.len(), 1);
        assert!(descriptor.required_permissions.contains(&"llm.call".into()));
    }
}
