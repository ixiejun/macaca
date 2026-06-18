//! Payment Service descriptor construction.
//!
//! The descriptor is a declarative capability record. Keeping it outside the
//! provider implementation makes the service contract auditable without reading
//! command dispatch or adapter state-management code.

use macaca_proto::{
    CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor, ServiceHealth,
    ServiceScope, ServiceType, TraceSchemaRef, PAYMENT_SERVICE_ID,
};

/// Build the provider-neutral descriptor for Payment Service.
pub fn payment_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(PAYMENT_SERVICE_ID),
        ServiceType::new("payment"),
        TraceSchemaRef::new("trace.system_service.payment.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.payment.quote"),
            "Negotiates provider-neutral A2A payment quotes.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.payment.settle"),
            "Settles approved local simulated payment intents.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.payment.snapshot"),
            "Reports sanitized Payment service snapshots.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec![
        "payment.quote".into(),
        "payment.intent.settle".into(),
        "payment.snapshot".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}
