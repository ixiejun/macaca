//! Audit and replay contracts for foundation-time service calls.
//!
//! The test uses the shared ServiceRouter Observer so time calls inherit the
//! same redaction and replay behavior as every other service. It deliberately
//! verifies only hashes enter audit events, never parse input or timer handles.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{KernelServiceId, ServiceBusSource, ServiceCommandName, TraceContext};
use macaca_time::FrozenTimeProvider;

use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServicePolicyEngine, ServiceProviderInstance, ServiceRouteRequest, ServiceRouter,
    ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory, TimeSystemServiceProvider,
};

#[tokio::test]
async fn time_router_replay_redacts_parse_input_and_reconstructs_clock_decision() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(TimeSystemServiceProvider::new(Arc::new(
        FrozenTimeProvider::new(42),
    )));
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-time-audit-start"))
        .await
        .unwrap();

    let sink = Arc::new(InMemoryServiceCallAuditSink::new());
    let policy: Arc<dyn ServicePolicyEngine> = Arc::new(InMemoryServicePolicyEngine::new());
    let router = ServiceRouter::new(
        Arc::clone(&runtime),
        ServiceBusSource::new("test.foundation.time"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        policy,
    )
    .with_audit_sink(sink.clone());
    let trace_id = "trace-time-audit-parse";
    let response = router.route(ServiceRouteRequest {
        app_id: Some("app:generic".into()), tenant_id: None, session_id: Some("session:time".into()),
        service_id: KernelServiceId::new("service.foundation.time"),
        operation: ServiceCommandName::new("time.parse"),
        payload: serde_json::json!({"input_ref":"2024-01-01T00:00:00Z","credential":"private-key-marker"}),
        metadata: BTreeMap::new(), trace: TraceContext::new(trace_id),
    }).await.unwrap();
    assert_eq!(response.output["epoch_millis"], 1_704_067_200_000_i64);

    let replay = router.replay_audit_by_trace_id(trace_id).unwrap();
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_requested"));
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_dispatched"));
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_succeeded"));
    let audit_text = format!("{replay:?}");
    assert!(!audit_text.contains("2024-01-01T00:00:00Z"));
    assert!(!audit_text.contains("private-key-marker"));
    assert!(replay.iter().all(
        |event| event.input_hash.is_none() || event.input_hash.as_deref().unwrap().len() <= 32
    ));
}
