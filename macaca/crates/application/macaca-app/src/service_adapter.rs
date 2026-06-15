//! System service descriptor adapter for the Application Framework.
//!
//! The Application Service descriptor is the discovery contract for applications.
//! It advertises application lifecycle, session envelope, host dispatch, and
//! GenUI surface capabilities without exposing `AppRuntime`, `AppRegistry`,
//! `Kernel`, Web state, or package-store implementation details.

use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceSchemaRef, APPLICATION_SERVICE_ID,
};

/// Build the provider-neutral descriptor for the Application system service.
pub fn application_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(APPLICATION_SERVICE_ID),
        ServiceType::new("application"),
        TraceSchemaRef::new("trace.system_service.application.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            CapabilityId::new("capability.application.discovery"),
            "Discovers applications through a traced service boundary.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.application.lifecycle"),
            "Starts, stops, removes, and snapshots applications through audited lifecycle commands.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.application.session"),
            "Creates session lifecycle envelopes without owning chat execution.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.application.host"),
            "Dispatches ApplicationHost ABI commands through trace-required service calls.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.application.genui"),
            "Reports application-owned GenUI surface metadata without binding to Web renderer internals.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.application.metadata"),
            "Projects sanitized application metadata views without exposing raw manifests or provider payloads.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![
        ServiceScope::Global,
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.required_permissions = vec![
        "application.discover".into(),
        "application.lifecycle".into(),
        "application.session".into(),
        "application.host".into(),
        "application.genui".into(),
        "application.metadata".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_descriptor_exports_contract_shape() {
        let descriptor = application_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "application");
        assert_eq!(descriptor.capabilities.len(), 6);
        assert!(descriptor
            .capabilities
            .iter()
            .any(|capability| capability.id.as_str() == "capability.application.metadata"));
        assert!(descriptor
            .required_permissions
            .contains(&"application.lifecycle".into()));
        assert!(descriptor
            .required_permissions
            .contains(&"application.metadata".into()));
    }
}
