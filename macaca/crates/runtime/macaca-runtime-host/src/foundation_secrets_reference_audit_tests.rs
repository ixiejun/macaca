//! Sanitized replay proofs for the secrets-reference service bridge.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_foundation_secrets_reference::MockSecretsReferenceProvider;
use macaca_kernel::SystemService;
use macaca_proto::{ServiceBusSource, ServiceCommandName, TraceContext};

use crate::foundation_secrets_reference_service_provider::FoundationSecretsReferenceSystemServiceProvider;
use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServiceProviderInstance, ServiceRouteRequest, ServiceRouter, ServiceRuntime,
    ServiceRuntimeConfig, StaticServiceProviderFactory,
};

#[tokio::test]
async fn secrets_reference_commands_are_traceable_and_redacted() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> =
        Arc::new(FoundationSecretsReferenceSystemServiceProvider::new(
            Arc::new(MockSecretsReferenceProvider::default()),
        ));
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-secrets-start"))
        .await
        .unwrap();
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.secrets-reference"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        Arc::new(InMemoryServicePolicyEngine::new()),
    )
    .with_audit_sink(Arc::new(InMemoryServiceCallAuditSink::new()));
    for operation in macaca_proto::FOUNDATION_SECRETS_REFERENCE_COMMANDS {
        let trace_id = format!("trace-secrets-{operation}");
        router.route(ServiceRouteRequest {
            app_id: Some("app:generic".into()), tenant_id: None, session_id: None,
            service_id: service_id.clone(), operation: ServiceCommandName::new(*operation),
            payload: serde_json::json!({
                "reference":{"reference_id":"secret-ref","provider_class":"mock","version_hint":"current"},
                "purpose":{"purpose":"test-purpose","service_id":"service.test","expires_at_epoch_millis":null},
                "policy":{"allowed_service_ids":["service.test"],"requires_approval":false,"max_lease_ttl_seconds":3600},
                "locator":{"provider_class":"mock","redacted_locator_hash":"locator-hash"},
                "lease":{"lease_id":"lease:test","reference_id":"secret-ref","expires_at_epoch_millis":999999999i64},
                "raw_secret":"redact-me","provider_locator":"redact-me"
            }),
            metadata: BTreeMap::new(), trace: TraceContext::new(&trace_id)
        }).await.unwrap();
        let replay = router.replay_audit_by_trace_id(&trace_id).unwrap();
        assert!(!format!("{replay:?}").contains("redact-me"));
        assert!(replay.iter().any(|event| event
            .replay_metadata
            .get("replay.secrets_reference_command")
            == Some(&operation.to_string())));
    }
}
