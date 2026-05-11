use macaca_proto::{
    PluginAbiVersion, PluginHealth, PluginHostDescriptor, PluginHostEntryPoint,
    PluginHostOperation, PluginHostStatus, PluginId, PluginRuntimeKind, PluginVersion,
    TraceContext,
};

use super::*;

fn descriptor(runtime_kind: PluginRuntimeKind) -> PluginHostDescriptor {
    let mut descriptor = PluginHostDescriptor::new(
        PluginId::new("plugin.host.fixture"),
        PluginVersion::new("1.0.0"),
        runtime_kind,
        PluginAbiVersion::default(),
    );
    descriptor.entry_point = Some(PluginHostEntryPoint::new("fixture", "entry"));
    descriptor
}

#[tokio::test]
async fn descriptor_host_accepts_metadata_only_start() {
    let supervisor = PluginHostLifecycleSupervisor::new();
    let result = supervisor
        .execute_operation(
            descriptor(PluginRuntimeKind::DescriptorOnly),
            PluginHostOperation::Start,
            serde_json::json!({}),
            TraceContext::new("host-trace"),
        )
        .await
        .unwrap();

    assert_eq!(result.status, PluginHostStatus::Accepted);
    assert_eq!(result.error_code, None);
}

#[tokio::test]
async fn wasm_process_and_remote_hosts_are_unavailable_safe() {
    for kind in [
        PluginRuntimeKind::Wasm,
        PluginRuntimeKind::Process,
        PluginRuntimeKind::RemoteProxy,
    ] {
        let supervisor = PluginHostLifecycleSupervisor::new();
        let result = supervisor
            .execute_operation(
                descriptor(kind),
                PluginHostOperation::Start,
                serde_json::json!({}),
                TraceContext::new("host-trace"),
            )
            .await
            .unwrap();

        assert_eq!(result.status, PluginHostStatus::Unavailable);
        assert_eq!(
            result.error_code.as_deref(),
            Some("plugin_host_unavailable")
        );
    }
}

#[tokio::test]
async fn remote_proxy_health_probe_is_auditable_and_unavailable() {
    let supervisor = PluginHostLifecycleSupervisor::new();
    let health = supervisor
        .health(
            descriptor(PluginRuntimeKind::RemoteProxy),
            TraceContext::new("host-health-trace"),
        )
        .await
        .unwrap();

    assert_eq!(health.runtime_kind, PluginRuntimeKind::RemoteProxy);
    assert!(matches!(health.health, PluginHealth::Unavailable { .. }));
    assert_eq!(health.trace.trace_id, "host-health-trace");
}
