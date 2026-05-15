use std::sync::Arc;

use macaca_kernel::MockSystemService;
use macaca_proto::{
    KernelServiceId, ServiceCommand, ServiceCommandName, ServiceDescriptor, ServiceHealth,
    ServiceLifecycleState, ServiceType, TraceContext, TraceSchemaRef,
};
use macaca_runtime_host::{
    DenyAllServiceRuntimePolicy, InMemoryServiceRuntimeEventSink, ServiceProviderInstance,
    ServiceRuntime, ServiceRuntimeConfig, ServiceRuntimeError, StaticServiceProviderFactory,
};

fn descriptor(service_id: &str) -> ServiceDescriptor {
    ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new("test.service"),
        TraceSchemaRef::new("trace.test.service.v1"),
    )
}

fn runtime_with_events() -> (ServiceRuntime, Arc<InMemoryServiceRuntimeEventSink>) {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    (runtime, events)
}

fn factory(service_id: &str) -> StaticServiceProviderFactory {
    let descriptor = descriptor(service_id);
    StaticServiceProviderFactory::new(ServiceProviderInstance::new(
        descriptor.clone(),
        Arc::new(MockSystemService::new(descriptor)),
    ))
}

fn failing_factory(service_id: &str) -> StaticServiceProviderFactory {
    let descriptor = descriptor(service_id);
    StaticServiceProviderFactory::new(ServiceProviderInstance::new(
        descriptor.clone(),
        Arc::new(MockSystemService::failing(descriptor)),
    ))
}

fn traced_command(trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new("invoke"),
        serde_json::json!({"input": true}),
        TraceContext::new(trace_id),
    )
}

#[tokio::test]
async fn service_runtime_registers_starts_calls_stops_and_cleans_mock_service() {
    let (runtime, events) = runtime_with_events();
    let service_id = runtime
        .register_provider(&factory("service.runtime.mock"), Default::default())
        .await
        .unwrap();

    runtime
        .start(&service_id, TraceContext::new("trace-start"))
        .await
        .unwrap();
    let reply = runtime
        .call(
            &service_id,
            macaca_proto::ServiceBusSource::new("test.source"),
            traced_command("trace-call"),
        )
        .await
        .unwrap();
    runtime
        .stop(&service_id, TraceContext::new("trace-stop"))
        .await
        .unwrap();
    runtime
        .cleanup(&service_id, TraceContext::new("trace-cleanup"))
        .await
        .unwrap();

    assert!(reply.success);
    assert_eq!(reply.output, Some(serde_json::json!({"input": true})));
    let snapshot = runtime.snapshot().unwrap();
    assert_eq!(snapshot.services.len(), 1);
    assert_eq!(
        snapshot.services[0].lifecycle_state,
        ServiceLifecycleState::CleanedUp
    );
    let emitted = events.events().unwrap();
    assert!(emitted
        .iter()
        .any(|event| event.operation == "service_runtime.register.completed"));
    assert!(emitted
        .iter()
        .any(|event| event.operation == "service_runtime.call.completed"));
}

#[tokio::test]
async fn service_runtime_rejects_missing_trace_before_dispatch() {
    let (runtime, events) = runtime_with_events();
    let service_id = runtime
        .register_provider(&factory("service.runtime.trace"), Default::default())
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-start"))
        .await
        .unwrap();

    let err = runtime
        .call(
            &service_id,
            macaca_proto::ServiceBusSource::new("test.source"),
            ServiceCommand::without_trace(ServiceCommandName::new("invoke"), serde_json::json!({})),
        )
        .await
        .unwrap_err();

    assert_eq!(err, ServiceRuntimeError::MissingTraceContext);
    assert!(events
        .events()
        .unwrap()
        .iter()
        .any(|event| event.operation == "service_runtime.call.rejected"));
}

#[tokio::test]
async fn service_runtime_rejects_policy_denial_before_dispatch() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        policy: Arc::new(DenyAllServiceRuntimePolicy::new("blocked by test policy")),
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let service_id = runtime
        .register_provider(&factory("service.runtime.policy"), Default::default())
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-start"))
        .await
        .unwrap();

    let err = runtime
        .call(
            &service_id,
            macaca_proto::ServiceBusSource::new("test.source"),
            traced_command("trace-policy"),
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        ServiceRuntimeError::PolicyDenied("blocked by test policy".into())
    );
    assert!(events
        .events()
        .unwrap()
        .iter()
        .any(|event| event.operation == "service_runtime.call.rejected"));
}

#[tokio::test]
async fn service_runtime_records_all_admission_decorators_before_dispatch() {
    let (runtime, events) = runtime_with_events();
    let service_id = runtime
        .register_provider(&factory("service.runtime.admission"), Default::default())
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-start"))
        .await
        .unwrap();

    runtime
        .call(
            &service_id,
            macaca_proto::ServiceBusSource::new("test.source"),
            traced_command("trace-admission"),
        )
        .await
        .unwrap();

    let dispatched = events
        .events()
        .unwrap()
        .into_iter()
        .find(|event| event.operation == "service_runtime.call.dispatched")
        .expect("service runtime should emit dispatch event after admission decorators");
    let decorators = dispatched
        .payload
        .get("admission_decorators")
        .and_then(|value| value.as_array())
        .expect("dispatch event should list admission decorators");
    let decorator_names: Vec<&str> = decorators.iter().filter_map(|v| v.as_str()).collect();

    assert_eq!(
        decorator_names,
        vec![
            "trace_required",
            "policy",
            "resource_placeholder",
            "entitlement_placeholder",
            "metering_placeholder",
            "audit_placeholder",
        ]
    );
}

#[tokio::test]
async fn service_runtime_rejects_duplicate_and_unknown_services() {
    let (runtime, _events) = runtime_with_events();
    let first = factory("service.runtime.identity");
    let second = factory("service.runtime.identity");
    let service_id = runtime
        .register_provider(&first, Default::default())
        .await
        .unwrap();

    let duplicate = runtime
        .register_provider(&second, Default::default())
        .await
        .unwrap_err();
    let unknown = runtime
        .call(
            &KernelServiceId::new("service.runtime.missing"),
            macaca_proto::ServiceBusSource::new("test.source"),
            traced_command("trace-unknown"),
        )
        .await
        .unwrap_err();

    assert_eq!(
        duplicate,
        ServiceRuntimeError::DuplicateService(service_id.to_string())
    );
    assert_eq!(
        unknown,
        ServiceRuntimeError::UnknownService("service.runtime.missing".into())
    );
}

#[tokio::test]
async fn service_runtime_records_provider_failure_as_failed_state() {
    let (runtime, events) = runtime_with_events();
    let service_id = runtime
        .register_provider(
            &failing_factory("service.runtime.failure"),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-start"))
        .await
        .unwrap();

    let err = runtime
        .call(
            &service_id,
            macaca_proto::ServiceBusSource::new("test.source"),
            traced_command("trace-failure"),
        )
        .await
        .unwrap_err();

    assert!(matches!(err, ServiceRuntimeError::Service(_)));
    let snapshot = runtime.snapshot().unwrap();
    assert!(matches!(
        snapshot.services[0].lifecycle_state,
        ServiceLifecycleState::Failed { .. }
    ));
    assert!(matches!(
        snapshot.services[0].health,
        ServiceHealth::Unavailable { .. }
    ));
    assert!(events
        .events()
        .unwrap()
        .iter()
        .any(|event| event.operation == "service_runtime.call.failed"));
}
