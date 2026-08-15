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
    // These values model sensitive data from independent sources. The router may retain
    // bounded command evidence, but must never serialize source payloads into audit/replay.
    router.route(ServiceRouteRequest { app_id: Some("app:generic".into()), tenant_id: None, session_id: None, service_id: KernelServiceId::new("service.foundation.config"), operation: ServiceCommandName::new("config.get"), payload: serde_json::json!({"key":{"namespace":"app","key":"ui.theme"},"secret":"raw-secret-marker","credential":"private-key-marker","environment":{"TOKEN":"raw-environment-marker"},"prompt":"raw-prompt-marker","manifest":"raw-manifest-marker","package_bytes":"raw-package-marker","provider_payload":"raw-provider-marker","private_key":"raw-private-key-marker","unbounded_value":"raw-config-value-marker"}), metadata: BTreeMap::new(), trace: TraceContext::new(trace_id) }).await.unwrap();
    let replay = router.replay_audit_by_trace_id(trace_id).unwrap();
    let text = format!("{replay:?}");
    assert!(!text.contains("raw-secret-marker"));
    assert!(!text.contains("private-key-marker"));
    assert!(!text.contains("theme-default"));
    for forbidden in [
        "raw-environment-marker",
        "raw-prompt-marker",
        "raw-manifest-marker",
        "raw-package-marker",
        "raw-provider-marker",
        "raw-private-key-marker",
        "raw-config-value-marker",
    ] {
        assert!(!text.contains(forbidden), "audit exposed {forbidden}");
    }
    let success = replay
        .iter()
        .find(|event| event.stage == "service_call_succeeded")
        .unwrap();
    assert_eq!(
        success.replay_metadata.get("replay.config_command"),
        Some(&"config.get".into())
    );
}
