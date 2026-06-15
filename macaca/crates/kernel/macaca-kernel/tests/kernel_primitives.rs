use macaca_kernel::{
    CapabilityRegistry, DefaultAllowPolicyEngine, DefaultKernelFacade, KernelFacade,
    KernelTraceEvent, PolicyEngine, ResourceManager, StaticDenyPolicyEngine, SystemServiceRegistry,
    TraceEventBus,
};
use macaca_proto::{
    CapabilityDescriptor, CapabilityId, KernelPrimitiveError, KernelServiceId, PolicyDecision,
    PolicyRequest, ResourceScope, ServiceScope, TraceContext,
};

fn assert_kernel_facade<T: KernelFacade>(_facade: &T) {}

#[test]
fn default_facade_registers_and_queries_capability() {
    let facade = DefaultKernelFacade::new();
    assert_kernel_facade(&facade);

    let descriptor = CapabilityDescriptor::new(
        CapabilityId::new("capability.plan"),
        KernelServiceId::new("service.task"),
        ServiceScope::Global,
        "Plan capability exposed through a provider-neutral descriptor.",
    );

    facade.register_capability(descriptor.clone()).unwrap();

    let found = facade
        .get_capability(&CapabilityId::new("capability.plan"))
        .unwrap();
    assert_eq!(found, Some(descriptor));
}

#[test]
fn service_registry_stores_provider_neutral_scope() {
    let facade = DefaultKernelFacade::new();
    let service_id = KernelServiceId::new("service.memory");

    facade
        .register_service(
            service_id.clone(),
            ServiceScope::Application("app-1".into()),
        )
        .unwrap();

    let scope = facade.get_service_scope(&service_id).unwrap();
    assert_eq!(scope, Some(ServiceScope::Application("app-1".into())));
}

#[test]
fn default_policy_allows_for_additive_startup() {
    let policy = DefaultAllowPolicyEngine::new();
    let request = PolicyRequest::new("app:test", "capability.call");

    let decision = policy.evaluate(&request).unwrap();

    assert!(matches!(decision, PolicyDecision::Allow { .. }));
    assert!(decision.is_allowed());
}

#[test]
fn deny_policy_returns_structured_denial() {
    let policy = StaticDenyPolicyEngine::new("approval required");
    let request = PolicyRequest::new("app:test", "capability.call");

    let decision = policy.evaluate(&request).unwrap();

    assert_eq!(
        decision,
        PolicyDecision::Deny {
            reason: "approval required".into()
        }
    );
    assert!(!decision.is_allowed());
}

#[test]
fn resource_manager_rejects_duplicate_scope() {
    let facade = DefaultKernelFacade::new();
    let scope = ResourceScope::Workspace("shared".into());

    facade.register_resource(scope.clone()).unwrap();
    let err = facade.register_resource(scope.clone()).unwrap_err();

    assert_eq!(err, KernelPrimitiveError::ResourceAlreadyRegistered(scope));
}

#[test]
fn trace_event_bus_keeps_events_presentation_neutral() {
    let facade = DefaultKernelFacade::new();
    let event = KernelTraceEvent {
        context: TraceContext::new("trace-primitive"),
        event_type: "kernel.primitive.test".into(),
        payload: serde_json::json!({ "ok": true }),
    };

    facade.emit_trace(event.clone()).unwrap();

    let events = facade.trace_events().unwrap();
    assert_eq!(events, vec![event]);
}
