//! Audit, replay, policy, and resource proofs for the KV service bridge.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_foundation_key_value_state::{
    KeyValueResourceLedger, KeyValueStateService, MockKeyValueStateProvider,
};
use macaca_kernel::SystemService;
use macaca_proto::{
    KernelServiceId, KeyValueResourceLimits, ServiceBusSource, ServiceCommand, ServiceCommandName,
    ServiceError, TraceContext,
};

use crate::foundation_key_value_state_service_provider::FoundationKeyValueStateSystemServiceProvider;
use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServicePolicyLayer, ServiceProviderInstance, ServiceRouteRequest, ServiceRouter,
    ServiceRuntime, ServiceRuntimeConfig, ServiceRuntimeError, StaticServiceProviderFactory,
};

async fn registered_provider(
    runtime: &ServiceRuntime,
    provider: Arc<dyn SystemService>,
) -> KernelServiceId {
    let descriptor = provider.descriptor();
    let id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&id, TraceContext::new("trace-key-value-start"))
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn key_value_router_replay_redacts_values_and_records_provider_stage() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> =
        Arc::new(FoundationKeyValueStateSystemServiceProvider::new(Arc::new(
            MockKeyValueStateProvider::default(),
        )));
    let id = registered_provider(&runtime, provider).await;
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.foundation.key-value-state"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        Arc::new(InMemoryServicePolicyEngine::new()),
    )
    .with_audit_sink(Arc::new(InMemoryServiceCallAuditSink::new()));
    let trace_id = "trace-key-value-audit";
    router.route(ServiceRouteRequest {
        app_id: Some("app:generic".into()), tenant_id: None, session_id: None,
        service_id: id, operation: ServiceCommandName::new("kv.put"),
        payload: serde_json::json!({"namespace":"preferences","key":"private-key","value":"raw-value","secret":"raw-secret","prompt":"raw-prompt","manifest":"raw-manifest","package_bytes":"raw-package","credential":"raw-credential","private_key":"raw-private-key","provider_payload":"raw-provider","unbounded_keys":"raw-unbounded-list"}),
        metadata: BTreeMap::new(), trace: TraceContext::new(trace_id),
    }).await.unwrap();
    let replay = router.replay_audit_by_trace_id(trace_id).unwrap();
    let text = format!("{replay:?}");
    for forbidden in [
        "private-key",
        "raw-value",
        "raw-secret",
        "raw-prompt",
        "raw-manifest",
        "raw-package",
        "raw-credential",
        "raw-private-key",
        "raw-provider",
        "raw-unbounded-list",
    ] {
        assert!(!text.contains(forbidden), "audit exposed {forbidden}");
    }
    assert!(replay
        .iter()
        .any(|event| event.stage == "key_value_state_pack_service_call_succeeded"));
    assert!(replay.iter().any(
        |event| event.replay_metadata.get("replay.key_value_state_command")
            == Some(&"kv.put".into())
    ));
}

#[tokio::test]
async fn key_value_policy_denial_and_resource_quota_precede_mock_side_effects() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let mock = Arc::new(MockKeyValueStateProvider::default());
    let provider: Arc<dyn SystemService> = Arc::new(
        FoundationKeyValueStateSystemServiceProvider::new(mock.clone()),
    );
    let id = registered_provider(&runtime, provider).await;
    let policy = Arc::new(InMemoryServicePolicyEngine::new());
    policy.set_baseline(ServicePolicyLayer {
        deny_services: ["service.foundation.key.value.state".into()].into(),
        ..Default::default()
    });
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.foundation.key-value-policy"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        policy,
    );
    let denied = router
        .route(ServiceRouteRequest {
            app_id: None,
            tenant_id: None,
            session_id: None,
            service_id: id,
            operation: ServiceCommandName::new("kv.put"),
            payload: serde_json::json!({}),
            metadata: BTreeMap::new(),
            trace: TraceContext::new("trace-key-value-denied"),
        })
        .await
        .unwrap_err();
    assert!(matches!(denied, ServiceRuntimeError::PolicyDenied(_)));
    assert_eq!(mock.snapshot().active_watch_count, 0);

    let quota_mock = Arc::new(MockKeyValueStateProvider::default());
    let quota_provider = FoundationKeyValueStateSystemServiceProvider::with_resource_ledger(
        quota_mock.clone(),
        KeyValueResourceLedger::new(KeyValueResourceLimits {
            max_byte_units: 0,
            max_entry_units: 0,
            max_batch_operations: 0,
            max_watch_slots: 0,
            max_snapshot_units: 0,
            max_mutation_operations: 0,
            max_request_units: 0,
        }),
    );
    let result = quota_provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("kv.watch_namespace"),
            serde_json::json!({}),
            TraceContext::new("trace-key-value-quota"),
        ))
        .await;
    assert!(result.is_err());
    assert_eq!(quota_mock.snapshot().active_watch_count, 0);

    let unsupported = MockKeyValueStateProvider::default();
    let result = unsupported
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("kv.provider_native_command"),
            serde_json::json!({}),
            TraceContext::new("trace-key-value-unsupported"),
        ))
        .await;
    assert!(matches!(result, Err(ServiceError::UnsupportedCommand(_))));
    assert_eq!(unsupported.snapshot().active_watch_count, 0);
}

#[tokio::test]
async fn every_key_value_command_has_trace_addressable_sanitized_replay_evidence() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> =
        Arc::new(FoundationKeyValueStateSystemServiceProvider::new(Arc::new(
            MockKeyValueStateProvider::default(),
        )));
    let service_id = registered_provider(&runtime, provider).await;
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.foundation.key-value-state.replay"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        Arc::new(InMemoryServicePolicyEngine::new()),
    )
    .with_audit_sink(Arc::new(InMemoryServiceCallAuditSink::new()));
    for operation in macaca_proto::FOUNDATION_KEY_VALUE_STATE_COMMANDS {
        let trace_id = format!("trace-key-value-replay-{operation}");
        router
            .route(ServiceRouteRequest {
                app_id: Some("app:generic".into()),
                tenant_id: None,
                session_id: None,
                service_id: service_id.clone(),
                operation: ServiceCommandName::new(*operation),
                payload: serde_json::json!({"namespace":"private-namespace","key":"private-key","value":"private-value"}),
                metadata: BTreeMap::new(),
                trace: TraceContext::new(&trace_id),
            })
            .await
            .unwrap();
        let replay = router.replay_audit_by_trace_id(&trace_id).unwrap();
        let success = replay
            .iter()
            .find(|event| event.stage == "service_call_succeeded")
            .unwrap();
        assert_eq!(
            success
                .replay_metadata
                .get("replay.key_value_state_command"),
            Some(&operation.to_string())
        );
        assert!(!format!("{replay:?}").contains("private-value"));
    }
}
