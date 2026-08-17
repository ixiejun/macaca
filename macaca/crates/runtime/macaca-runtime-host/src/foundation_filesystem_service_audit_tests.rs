//! Audit and policy-boundary proofs for the foundation filesystem service bridge.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_foundation_filesystem::{
    FilesystemResourceLedger, FilesystemService, MockFilesystemProvider,
};
use macaca_kernel::SystemService;
use macaca_proto::{
    FilesystemResourceLimits, KernelServiceId, ServiceBusSource, ServiceCommand,
    ServiceCommandName, TraceContext,
};

use crate::foundation_filesystem_service_provider::FoundationFilesystemSystemServiceProvider;
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
        .start(&id, TraceContext::new("trace-filesystem-start"))
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn filesystem_router_replay_redacts_paths_and_content_references() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(
        FoundationFilesystemSystemServiceProvider::new(Arc::new(MockFilesystemProvider::default())),
    );
    let id = registered_provider(&runtime, provider).await;
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.foundation.filesystem"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        Arc::new(InMemoryServicePolicyEngine::new()),
    )
    .with_audit_sink(Arc::new(InMemoryServiceCallAuditSink::new()));
    let trace_id = "trace-filesystem-audit";
    router
        .route(ServiceRouteRequest {
            app_id: Some("app:generic".into()),
            tenant_id: None,
            session_id: None,
            service_id: id,
            operation: ServiceCommandName::new("filesystem.write_file"),
            payload: serde_json::json!({"path":{"relative_path":"private-plan.txt"},"content":{"content_ref":"artifact:private-content"},"host_path":"/private/host-path-marker","file_bytes":"raw-file-bytes-marker","secret":"raw-secret-marker","credential":"raw-credential-marker","manifest":"raw-manifest-marker","package_bytes":"raw-package-marker","private_key":"raw-private-key-marker","provider_payload":"raw-provider-marker","unbounded_output":"raw-unbounded-output-marker"}),
            metadata: BTreeMap::new(),
            trace: TraceContext::new(trace_id),
        })
        .await
        .unwrap();
    let replay = router.replay_audit_by_trace_id(trace_id).unwrap();
    let text = format!("{replay:?}");
    for forbidden in [
        "private-plan.txt",
        "artifact:private-content",
        "/private/host-path-marker",
        "raw-file-bytes-marker",
        "raw-secret-marker",
        "raw-credential-marker",
        "raw-manifest-marker",
        "raw-package-marker",
        "raw-private-key-marker",
        "raw-provider-marker",
        "raw-unbounded-output-marker",
    ] {
        assert!(!text.contains(forbidden), "audit exposed {forbidden}");
    }
    let success = replay
        .iter()
        .find(|event| event.stage == "service_call_succeeded")
        .unwrap();
    assert_eq!(
        success.replay_metadata.get("replay.filesystem_command"),
        Some(&"filesystem.write_file".into())
    );
    assert!(replay
        .iter()
        .any(|event| event.stage == "filesystem_pack_service_call_succeeded"));
}

#[tokio::test]
async fn filesystem_policy_denial_happens_before_mock_invocation() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let mock = Arc::new(MockFilesystemProvider::default());
    let service: Arc<dyn FilesystemService> = mock.clone();
    let provider: Arc<dyn SystemService> =
        Arc::new(FoundationFilesystemSystemServiceProvider::new(service));
    let id = registered_provider(&runtime, provider).await;
    let policy = Arc::new(InMemoryServicePolicyEngine::new());
    policy.set_baseline(ServicePolicyLayer {
        deny_services: ["service.foundation.filesystem".into()].into(),
        ..Default::default()
    });
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.foundation.filesystem"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        policy,
    );
    let error = router
        .route(ServiceRouteRequest {
            app_id: None,
            tenant_id: None,
            session_id: None,
            service_id: id,
            operation: ServiceCommandName::new("filesystem.write_file"),
            payload: serde_json::json!({"path":{"relative_path":"document.txt"},"content":{"content_ref":"artifact:test"}}),
            metadata: BTreeMap::new(),
            trace: TraceContext::new("trace-filesystem-denied"),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, ServiceRuntimeError::PolicyDenied(_)));
    assert_eq!(mock.snapshot().open_handle_count, 0);
    assert_eq!(mock.snapshot().root_hashes.len(), 0);
}

#[tokio::test]
async fn filesystem_resource_quota_rejects_before_mock_side_effect_and_releases_capacity() {
    let mock = Arc::new(MockFilesystemProvider::default());
    let provider = FoundationFilesystemSystemServiceProvider::with_resource_ledger(
        mock.clone(),
        FilesystemResourceLedger::new(FilesystemResourceLimits {
            max_byte_units: 0,
            max_entry_units: 0,
            max_recursive_operations: 0,
            max_watch_slots: 0,
            max_snapshot_units: 0,
            max_mutation_operations: 0,
            max_request_units: 0,
        }),
    );
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("filesystem.write_file"),
            serde_json::json!({"path":{"relative_path":"document.txt"},"content":{"content_ref":"artifact:test"}}),
            TraceContext::new("trace-filesystem-resource-denied"),
        ))
        .await;
    assert!(result.is_err());
    assert!(mock.snapshot().root_hashes.is_empty());
}

#[tokio::test]
async fn every_filesystem_command_has_trace_addressable_sanitized_replay_evidence() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(
        FoundationFilesystemSystemServiceProvider::new(Arc::new(MockFilesystemProvider::default())),
    );
    let service_id = registered_provider(&runtime, provider).await;
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.foundation.filesystem.replay"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        Arc::new(InMemoryServicePolicyEngine::new()),
    )
    .with_audit_sink(Arc::new(InMemoryServiceCallAuditSink::new()));
    for operation in macaca_proto::FOUNDATION_FILESYSTEM_COMMANDS {
        let trace_id = format!("trace-filesystem-replay-{operation}");
        router
            .route(ServiceRouteRequest {
                app_id: Some("app:generic".into()),
                tenant_id: None,
                session_id: None,
                service_id: service_id.clone(),
                operation: ServiceCommandName::new(*operation),
                payload: replay_payload(operation),
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
            success.replay_metadata.get("replay.filesystem_command"),
            Some(&operation.to_string())
        );
        let serialized = format!("{replay:?}");
        assert!(!serialized.contains("private-document.txt"));
        assert!(!serialized.contains("artifact:private-content"));
    }
}

#[tokio::test]
async fn filesystem_bridge_cancels_watches_and_shutdown_clears_lifecycle_state() {
    let mock = Arc::new(MockFilesystemProvider::default());
    let provider = FoundationFilesystemSystemServiceProvider::new(mock.clone());
    provider.start().await.unwrap();
    let watch = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("filesystem.watch_path"),
            serde_json::json!({"path":{"relative_path":"document.txt"}}),
            TraceContext::new("trace-filesystem-watch-lifecycle"),
        ))
        .await
        .unwrap();
    let checkpoint = watch.output["watch_checkpoint"].as_str().unwrap();
    assert_eq!(provider.snapshot().active_watch_count, 1);
    provider.cancel_watch(checkpoint).await.unwrap();
    assert_eq!(provider.snapshot().active_watch_count, 0);
    provider.stop().await.unwrap();
    assert_eq!(provider.snapshot().open_handle_count, 0);
    assert!(matches!(
        provider.health().await.unwrap(),
        macaca_proto::ServiceHealth::Healthy
    ));
}

fn replay_payload(operation: &str) -> serde_json::Value {
    match operation {
        "filesystem.open_handle"
        | "filesystem.read_file"
        | "filesystem.stat_path"
        | "filesystem.list_directory"
        | "filesystem.create_directory"
        | "filesystem.delete_path"
        | "filesystem.watch_path" => {
            serde_json::json!({"path":{"relative_path":"private-document.txt"}})
        }
        "filesystem.write_file" | "filesystem.append_file" => {
            serde_json::json!({"path":{"relative_path":"private-document.txt"},"content":{"content_ref":"artifact:private-content"}})
        }
        _ => serde_json::json!({}),
    }
}
