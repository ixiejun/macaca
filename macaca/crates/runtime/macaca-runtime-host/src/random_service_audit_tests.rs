//! Redaction and replay checks for the provider-neutral random service.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{KernelServiceId, ServiceBusSource, ServiceCommandName, TraceContext};
use macaca_random::HostRandomProvider;

use crate::random_service_provider::RandomSystemServiceProvider;
use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServicePolicyEngine, ServiceProviderInstance, ServiceRouteRequest, ServiceRouter,
    ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn random_router_replay_redacts_generated_values_and_sensitive_input() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(RandomSystemServiceProvider::new(Arc::new(
        HostRandomProvider,
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
        .start(&service_id, TraceContext::new("trace-random-audit-start"))
        .await
        .unwrap();
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.foundation.random"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        Arc::new(InMemoryServicePolicyEngine::new()) as Arc<dyn ServicePolicyEngine>,
    )
    .with_audit_sink(Arc::new(InMemoryServiceCallAuditSink::new()));
    let trace_id = "trace-random-audit";
    let response = router.route(ServiceRouteRequest {
        app_id: Some("app:generic".into()), tenant_id: None, session_id: Some("session:random".into()),
        service_id: KernelServiceId::new("service.foundation.random"), operation: ServiceCommandName::new("random.bytes"),
        payload: serde_json::json!({"length":16,"seed":"raw-seed-marker","credential":"private-key-marker"}),
        metadata: BTreeMap::new(), trace: TraceContext::new(trace_id),
    }).await.unwrap();
    let generated = response.output["data"].as_str().unwrap().to_string();
    let replay = router.replay_audit_by_trace_id(trace_id).unwrap();
    let text = format!("{replay:?}");
    assert!(!text.contains(&generated));
    assert!(!text.contains("raw-seed-marker"));
    assert!(!text.contains("private-key-marker"));
    let success = replay
        .iter()
        .find(|event| event.stage == "service_call_succeeded")
        .unwrap();
    assert_eq!(
        success.replay_metadata.get("replay.random_command"),
        Some(&"random.bytes".into())
    );
    assert_eq!(
        success.replay_metadata.get("replay.random_length"),
        Some(&"16".into())
    );
}
