//! System service descriptor adapter for the task layer.
//!
//! The task service descriptor represents planning, todo, dependency, review,
//! and worker-loop capabilities as service metadata.  It does not move current
//! task execution or review logic into a new service bus.

use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceSchemaRef, TASK_SERVICE_ID,
};

/// Build the provider-neutral descriptor for the task system service.
pub fn task_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(TASK_SERVICE_ID),
        ServiceType::new("task"),
        TraceSchemaRef::new("trace.system_service.task.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            CapabilityId::new("capability.task.plan"),
            "Creates and manages autonomous task plans.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.task.review"),
            "Reviews task completion before coordinator resume.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.required_permissions = vec!["task.manage".into()];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_descriptor_exports_contract_shape() {
        let descriptor = task_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "task");
        assert_eq!(descriptor.capabilities.len(), 2);
        assert!(descriptor
            .required_permissions
            .contains(&"task.manage".into()));
    }
}
