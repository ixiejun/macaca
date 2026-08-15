//! Audit redaction proof for the provider-neutral foundation config service.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_foundation_config::MockConfigProvider;
use macaca_kernel::SystemService;
use macaca_proto::{KernelServiceId, ServiceBusSource, ServiceCommandName, TraceContext};

use crate::foundation_config_service_provider::FoundationConfigSystemServiceProvider;
use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServiceProviderInstance, ServiceRouteRequest, ServiceRouter, ServiceRuntime,
    ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn config_router_replay_redacts_payload_and_retains_only_trace_metadata() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let mock = Arc::new(MockConfigProvider::default());
    mock.insert_reference("ui.theme", "artifact:theme-default")
        .unwrap();
    let provider: Arc<dyn SystemService> =
        Arc::new(FoundationConfigSystemServiceProvider::new(mock));
    let descriptor = provider.descriptor();
    let id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&id, TraceContext::new("trace-config-audit-start"))
        .await
        .unwrap();
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.foundation.config"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        Arc::new(InMemoryServicePolicyEngine::new()),
    )
    .with_audit_sink(Arc::new(InMemoryServiceCallAuditSink::new()));
    let trace_id = "trace-config-audit";
    router.route(ServiceRouteRequest { app_id: Some("app:generic".into()), tenant_id: None, session_id: None, service_id: KernelServiceId::new("service.foundation.config"), operation: ServiceCommandName::new("config.get"), payload: serde_json::json!({"key":{"namespace":"app","key":"ui.theme"},"secret":"raw-secret-marker","credential":"private-key-marker"}), metadata: BTreeMap::new(), trace: TraceContext::new(trace_id) }).await.unwrap();
    let replay = router.replay_audit_by_trace_id(trace_id).unwrap();
    let text = format!("{replay:?}");
    assert!(!text.contains("raw-secret-marker"));
    assert!(!text.contains("private-key-marker"));
    assert!(!text.contains("theme-default"));
}
