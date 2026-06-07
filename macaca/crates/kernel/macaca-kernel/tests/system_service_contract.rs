use macaca_kernel::{
    trace_service_descriptor, DefaultKernelFacade, DefaultServiceLifecycleController,
    InMemoryTraceEventBus, MockSystemService, ServiceCallExecutor, ServiceLifecycleController,
    SystemService, SystemServiceRegistry,
};
use macaca_proto::{
    CapabilityId, KernelServiceId, ServiceCommand, ServiceCommandName, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceLifecycleState, ServiceScope, ServiceType, TraceContext,
    TraceSchemaRef,
};

fn descriptor(id: &str) -> ServiceDescriptor {
    ServiceDescriptor::new(
        KernelServiceId::new(id),
        ServiceType::new("contract-test"),
        TraceSchemaRef::new("trace.system_service.contract_test.v1"),
    )
}

#[tokio::test]
async fn mock_service_registers_starts_calls_stops_and_cleans_up() {
    let facade = DefaultKernelFacade::new();
    let service = MockSystemService::new(descriptor("service.contract.mock"));
    let service_descriptor = service.descriptor();

    facade
        .register_service(service_descriptor.id.clone(), ServiceScope::Global)
        .unwrap();
    service.start().await.unwrap();
    assert_eq!(service.health().await.unwrap(), ServiceHealth::Healthy);

    let executor = ServiceCallExecutor::new(InMemoryTraceEventBus::new());
    let command = ServiceCommand::with_trace(
        ServiceCommandName::new("echo"),
        serde_json::json!({ "value": 42 }),
        TraceContext::new("trace-contract-ok"),
    );
    let result = executor.execute(&service, command).await.unwrap();

    assert_eq!(result.output["value"], 42);
    service.stop().await.unwrap();
    service.cleanup().await.unwrap();
}

#[tokio::test]
async fn missing_trace_context_is_rejected_before_dispatch() {
    let service = MockSystemService::failing(descriptor("service.contract.no_trace"));
    let executor = ServiceCallExecutor::new(InMemoryTraceEventBus::new());
    let command = ServiceCommand::without_trace(
        ServiceCommandName::new("must_not_dispatch"),
        serde_json::json!({ "dispatch": false }),
    );

    let err = executor.execute(&service, command).await.unwrap_err();

    assert_eq!(err, ServiceError::MissingTraceContext);
    assert!(executor.trace_bus().events().unwrap().is_empty());
}

#[tokio::test]
async fn failed_service_call_returns_structured_error_and_trace() {
    let service = MockSystemService::failing(descriptor("service.contract.fail"));
    let executor = ServiceCallExecutor::new(InMemoryTraceEventBus::new());
    let command = ServiceCommand::with_trace(
        ServiceCommandName::new("fail"),
        serde_json::json!({ "value": false }),
        TraceContext::new("trace-contract-fail"),
    );

    let err = executor.execute(&service, command).await.unwrap_err();
    let events = executor.trace_bus().events().unwrap();

    assert!(matches!(err, ServiceError::AdapterFailure(_)));
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "system_service.call.accepted");
    assert_eq!(events[1].event_type, "system_service.call.failed");
}

#[tokio::test]
async fn valid_trace_context_emits_accepted_and_completed_events() {
    let service = MockSystemService::new(descriptor("service.contract.trace"));
    let executor = ServiceCallExecutor::new(InMemoryTraceEventBus::new());
    let command = ServiceCommand::with_trace(
        ServiceCommandName::new("echo"),
        serde_json::json!({ "ok": true }),
        TraceContext::new("trace-contract-complete"),
    );

    executor.execute(&service, command).await.unwrap();
    let events = executor.trace_bus().events().unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "system_service.call.accepted");
    assert_eq!(events[1].event_type, "system_service.call.completed");
}

#[test]
fn lifecycle_controller_rejects_invalid_transition() {
    let mut lifecycle = DefaultServiceLifecycleController::new();

    let err = lifecycle
        .transition_to(ServiceLifecycleState::Running)
        .unwrap_err();

    assert_eq!(
        err,
        ServiceError::InvalidLifecycleTransition {
            from: ServiceLifecycleState::Installed,
            to: ServiceLifecycleState::Running,
        }
    );
}

/// Kernel-owned system service descriptors must remain provider-neutral.
///
/// Provider-specific descriptors (task/driver/skill/gateway) are validated in their
/// respective service crates; the microkernel must not depend on those crates.
#[test]
fn kernel_trace_service_descriptor_exports_provider_neutral_contract() {
    let descriptor = trace_service_descriptor();

    assert!(!descriptor.id.is_empty());
    assert!(!descriptor.service_type.as_str().is_empty());
    assert!(!descriptor.trace_schema.as_str().is_empty());
    assert!(!descriptor.capabilities.is_empty());
    assert!(!descriptor.supported_scopes.is_empty());
    assert_eq!(descriptor.health, ServiceHealth::Healthy);
    assert!(descriptor.capabilities.iter().any(|cap| {
        cap.id == CapabilityId::new("capability.trace.emit")
    }));
}
