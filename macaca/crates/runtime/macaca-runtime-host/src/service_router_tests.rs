use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_kernel::MockSystemService;
use macaca_proto::{
    KernelServiceId, ServiceBusSource, ServiceCommandName, ServiceDescriptor, ServiceHealth,
    ServiceLifecycleState, ServiceType, TraceContext, TraceSchemaRef,
};
use serde_json::json;

use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServicePolicyEngine, ServiceProviderInstance, ServiceRouteRequest, ServiceRouter,
    ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn service_router_emits_replayable_audit_chain() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = register_mock_service(&runtime, "service.audit.chain").await;
    let contracts = Arc::new(InMemoryServiceContractRegistry::new());
    let policy_engine = Arc::new(InMemoryServicePolicyEngine::new());
    let policy_engine_trait: Arc<dyn ServicePolicyEngine> = policy_engine;
    let audit_sink = Arc::new(InMemoryServiceCallAuditSink::new());
    let router = ServiceRouter::new(
        Arc::clone(&runtime),
        ServiceBusSource::new("test.service.router"),
        contracts,
        policy_engine_trait,
    )
    .with_audit_sink(audit_sink.clone());

    let response = router
        .route(ServiceRouteRequest {
            app_id: Some("app.audit".into()),
            tenant_id: None,
            session_id: Some("session.audit".into()),
            service_id: service_id.clone(),
            operation: ServiceCommandName::new("invoke"),
            payload: json!({"probe": true}),
            metadata: BTreeMap::new(),
            trace: TraceContext::new("trace.audit.chain"),
        })
        .await
        .unwrap();

    assert_eq!(response.status, "ok");
    let replay = audit_sink.replay_by_trace_id("trace.audit.chain").unwrap();
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_requested"));
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_dispatched"));
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_succeeded"));

    let router_replay = router
        .replay_audit_by_trace_id("trace.audit.chain")
        .unwrap();
    assert_eq!(router_replay.len(), replay.len());
    let session_replay = router.replay_audit_by_session_id("session.audit").unwrap();
    assert!(!session_replay.is_empty());
}

async fn register_mock_service(runtime: &ServiceRuntime, service_id: &str) -> KernelServiceId {
    let descriptor = ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new("test.service"),
        TraceSchemaRef::new("trace.test.service.v1"),
    );
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                descriptor.clone(),
                Arc::new(MockSystemService::new(descriptor)),
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-start-router-test"))
        .await
        .unwrap();
    let snapshot = runtime.snapshot().unwrap();
    assert_eq!(
        snapshot.services[0].lifecycle_state,
        ServiceLifecycleState::Running
    );
    assert_eq!(snapshot.services[0].health, ServiceHealth::Healthy);
    service_id
}
