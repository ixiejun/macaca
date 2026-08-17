//! Contract tests for provider-neutral foundation config providers.

use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

use crate::{
    ConfigService, ConfigSourceKind, LayeredConfigProvider, MockConfigProvider,
    ReferenceMapConfigSource, UnavailableConfigProvider,
};

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn mock_provider_returns_only_opaque_references_and_hashed_snapshot_keys() {
    let provider = MockConfigProvider::default();
    provider
        .insert_reference("ui.theme", "artifact:theme-default")
        .unwrap();
    let reply = provider
        .call(command(
            "config.get",
            serde_json::json!({
                "key":{"namespace":"app","key":"ui.theme"},
                "selector":{"profile":"default"}
            }),
        ))
        .await
        .unwrap();
    assert_eq!(reply.output["value_ref"], "artifact:theme-default");
    assert!(!reply.output.to_string().contains("raw-secret"));
    assert_eq!(
        reply.metadata.get("config.audit_event").map(String::as_str),
        Some("config_pack_service_call_succeeded")
    );
    let snapshot = provider.snapshot();
    assert!(snapshot
        .source_hashes
        .keys()
        .all(|key| !key.contains("ui.theme")));
    provider.shutdown().await.unwrap();
    assert!(provider.snapshot().source_hashes.is_empty());
}

#[tokio::test]
async fn unavailable_provider_fails_closed_with_trace_evidence() {
    let provider = UnavailableConfigProvider::new("source unavailable");
    let reply = provider
        .call(command("config.get", serde_json::json!({})))
        .await
        .unwrap();
    assert_eq!(reply.status, "unavailable");
    assert_eq!(reply.trace.trace_id.as_str(), "trace-config.get");
    assert_eq!(
        reply.metadata.get("config.audit_event").map(String::as_str),
        Some("config_pack_unavailable")
    );
    assert_eq!(provider.snapshot().provider_class, "unavailable");
}

#[tokio::test]
async fn every_declared_config_command_is_trace_addressable_for_replay() {
    let provider = MockConfigProvider::default();
    provider
        .insert_reference("setting", "artifact:replay-reference")
        .unwrap();
    for operation in macaca_proto::FOUNDATION_CONFIG_COMMANDS {
        let payload = match *operation {
            "config.get" | "config.resolve_effective" | "config.explain_provenance" => {
                serde_json::json!({"key":{"key":"setting"}})
            }
            _ => serde_json::json!({}),
        };
        let reply = provider.call(command(operation, payload)).await.unwrap();
        assert_eq!(
            reply.metadata.get("replay.config_command"),
            Some(&operation.to_string())
        );
        assert_eq!(reply.trace.trace_id.as_str(), format!("trace-{operation}"));
    }
    let snapshot = provider.snapshot();
    assert_eq!(snapshot.provider_class, "mock");
    assert!(!snapshot.source_hashes.is_empty());
    assert_eq!(snapshot.layer_order, vec!["mock"]);
    assert_eq!(snapshot.validation_status, "valid");
    assert!(snapshot.replay_ref.starts_with("replay:foundation-config:"));
}

#[tokio::test]
async fn layered_adapters_apply_declared_precedence_and_clear_on_shutdown() {
    // The source list deliberately contains no app-specific identity. Runtime composition
    // supplies generic source IDs and determines precedence without exposing native handles.
    let package =
        ReferenceMapConfigSource::new(ConfigSourceKind::PackageDescriptor, "pkg-v1").unwrap();
    let workspace =
        ReferenceMapConfigSource::new(ConfigSourceKind::Workspace, "workspace-v1").unwrap();
    let environment =
        ReferenceMapConfigSource::new(ConfigSourceKind::Environment, "env-v1").unwrap();
    let tenant = ReferenceMapConfigSource::new(ConfigSourceKind::Tenant, "tenant-v1").unwrap();
    let remote = ReferenceMapConfigSource::new(ConfigSourceKind::Remote, "remote-v1").unwrap();
    package
        .insert_reference("setting", "artifact:package")
        .unwrap();
    workspace
        .insert_reference("setting", "artifact:workspace")
        .unwrap();
    tenant
        .insert_reference("setting", "secret:tenant-ref")
        .unwrap();
    let provider =
        LayeredConfigProvider::new(vec![package, workspace, environment, tenant, remote]).unwrap();

    let reply = provider
        .call(command(
            "config.get",
            serde_json::json!({"key":{"key":"setting"}}),
        ))
        .await
        .unwrap();
    assert_eq!(reply.output["value_ref"], "secret:tenant-ref");
    assert!(reply.output["source_hash"].is_string());
    assert_eq!(provider.snapshot().source_hashes.len(), 5);
    provider.shutdown().await.unwrap();
    let cleared = provider
        .call(command(
            "config.get",
            serde_json::json!({"key":{"key":"setting"}}),
        ))
        .await
        .unwrap();
    assert_eq!(cleared.output["status"], "not_found");
    assert_eq!(provider.health(), macaca_proto::ServiceHealth::Healthy);
}

#[tokio::test]
async fn providers_report_capabilities_and_release_watch_lifecycle_state() {
    let provider = MockConfigProvider::default();
    let watched = provider
        .call(command("config.watch", serde_json::json!({})))
        .await
        .unwrap();
    let checkpoint = watched.output["checkpoint"].as_str().unwrap();
    assert!(provider.provider_capabilities().supports_watch);
    provider.cancel_watch(checkpoint).await.unwrap();
    provider.shutdown().await.unwrap();
}
