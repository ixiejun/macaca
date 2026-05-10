//! System service descriptor adapter for the kernel trace boundary.
//!
//! Trace delivery is represented as a service surface so later phases can route
//! trace and audit events through a service bus.  The adapter only exports
//! metadata; it does not replace the existing trace/event persistence paths.

use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceSchemaRef,
};

/// Build the provider-neutral descriptor for the trace system service.
pub fn trace_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new("service.trace"),
        ServiceType::new("trace"),
        TraceSchemaRef::new("trace.system_service.trace.v1"),
    );
    descriptor.capabilities = vec![ServiceCapability::new(
        CapabilityId::new("capability.trace.emit"),
        "Emits trace and audit events through a presentation-neutral boundary.",
    )];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Session("*".into())];
    descriptor.required_permissions = vec!["trace.emit".into()];
    descriptor.cleanup_policy = CleanupPolicy::None;
    descriptor
}
