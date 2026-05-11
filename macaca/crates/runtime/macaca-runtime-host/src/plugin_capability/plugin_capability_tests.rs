use std::sync::Arc;

use macaca_proto::{
    CapabilityId, DeveloperId, KernelServiceId, PluginCapabilityCallCommand,
    PluginCapabilityDeactivateCommand, PluginCapabilityDescriptor, PluginCapabilityKind,
    PluginCapabilityQueryCommand, PluginCapabilityRegisterCommand, PluginCapabilitySlot, PluginId,
    PluginInstallRequest, PluginInstallSourceKind, PluginPackageLocation, PluginPermission,
    PluginProvidedCapability, PluginResource, PluginRuntimeDeclaration, PluginRuntimeKind,
    PluginServiceCategory, PluginSignature, PluginTargetCommand, PluginVersion, TraceContext,
    TraceSchemaRef,
};

use crate::plugin_capability::{
    discover_manifest_capabilities, PluginCapabilityRegistry, PluginCapabilityService,
};
use crate::plugin_control::PluginControlService;

fn descriptor(plugin_id: &str, capability_id: &str, slot: &str) -> PluginCapabilityDescriptor {
    let mut descriptor = PluginCapabilityDescriptor::new(
        CapabilityId::new(capability_id),
        PluginId::new(plugin_id),
        PluginCapabilityKind::CliCommand,
        slot,
        TraceSchemaRef::new("trace.plugin.capability.cli.v1"),
    );
    descriptor
        .slots
        .push(PluginCapabilitySlot::new("cli_command", slot, true));
    descriptor
}

fn manifest(plugin_id: &str, capability_id: &str) -> macaca_proto::PluginManifest {
    let mut manifest = macaca_proto::PluginManifest::new(
        PluginId::new(plugin_id),
        PluginVersion::new("1.0.0"),
        DeveloperId::new("developer.fixture"),
        PluginRuntimeDeclaration::new(PluginRuntimeKind::DescriptorOnly, "1"),
    );
    manifest
        .provides_capabilities
        .push(PluginProvidedCapability {
            id: CapabilityId::new(capability_id),
            service: Some(KernelServiceId::new("service.fixture")),
            category: PluginServiceCategory::Gateway,
            description: "Gateway fixture capability".into(),
            required_permissions: vec!["permission.gateway".into()],
            metadata: [("trace_schema".into(), "trace.fixture.gateway.v1".into())].into(),
        });
    manifest.permissions.push(PluginPermission {
        id: "permission.gateway".into(),
        reason: "Gateway fixture permission".into(),
        optional: false,
        metadata: Default::default(),
    });
    manifest.resources.push(PluginResource {
        id: "resource.gateway".into(),
        kind: "network".into(),
        reason: "Gateway fixture resource".into(),
        optional: false,
        metadata: Default::default(),
    });
    manifest.signature = Some(PluginSignature {
        algorithm: "fixture".into(),
        key_id: "fixture".into(),
        digest: "fixture".into(),
        verification_status: "verification_pending".into(),
    });
    manifest
}

#[test]
fn manifest_discovery_does_not_start_runtime_code() {
    let manifest = manifest("plugin.discovery", "capability.discovery");
    let descriptors = discover_manifest_capabilities(&manifest);

    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].plugin_id, PluginId::new("plugin.discovery"));
    assert_eq!(descriptors[0].kind, PluginCapabilityKind::Gateway);
    assert_eq!(
        descriptors[0].trace_schema.as_str(),
        "trace.fixture.gateway.v1"
    );
}

#[tokio::test]
async fn duplicate_exclusive_slot_is_rejected() {
    let registry = PluginCapabilityRegistry::default();
    registry
        .register(PluginCapabilityRegisterCommand {
            plugin_id: PluginId::new("plugin.first"),
            descriptors: vec![descriptor("plugin.first", "capability.first", "inspect")],
            trace: TraceContext::new("trace-first"),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    let err = registry
        .register(PluginCapabilityRegisterCommand {
            plugin_id: PluginId::new("plugin.second"),
            descriptors: vec![descriptor("plugin.second", "capability.second", "inspect")],
            trace: TraceContext::new("trace-second"),
            metadata: Default::default(),
        })
        .await
        .unwrap_err();
    assert!(err.to_string().contains("capability conflict"));
}

#[tokio::test]
async fn deactivation_removes_active_query_results() {
    let service = PluginCapabilityService::in_memory();
    service
        .register(PluginCapabilityRegisterCommand {
            plugin_id: PluginId::new("plugin.cleanup"),
            descriptors: vec![descriptor("plugin.cleanup", "capability.cleanup", "clean")],
            trace: TraceContext::new("trace-register"),
            metadata: Default::default(),
        })
        .await
        .unwrap();
    service
        .deactivate(PluginCapabilityDeactivateCommand {
            plugin_id: PluginId::new("plugin.cleanup"),
            trace: TraceContext::new("trace-deactivate"),
            reason: "test cleanup".into(),
        })
        .await
        .unwrap();

    let active = service
        .query(PluginCapabilityQueryCommand {
            kind: None,
            plugin_id: Some(PluginId::new("plugin.cleanup")),
            slot_namespace: None,
            slot_key: None,
            include_inactive: false,
            trace: TraceContext::new("trace-query-active"),
        })
        .await
        .unwrap();
    assert!(active.is_empty());
}

#[tokio::test]
async fn capability_call_returns_structured_unavailable_without_host() {
    let service = PluginCapabilityService::in_memory();
    service
        .register(PluginCapabilityRegisterCommand {
            plugin_id: PluginId::new("plugin.call"),
            descriptors: vec![descriptor("plugin.call", "capability.call", "call")],
            trace: TraceContext::new("trace-register"),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    let err = service
        .call(PluginCapabilityCallCommand {
            capability_id: CapabilityId::new("capability.call"),
            plugin_id: Some(PluginId::new("plugin.call")),
            input: serde_json::json!({ "value": true }),
            trace: TraceContext::new("trace-call"),
            metadata: Default::default(),
        })
        .await
        .unwrap_err();
    assert!(err.to_string().contains("execution host is unavailable"));
}

#[tokio::test]
async fn control_service_syncs_capability_cleanup_on_disable() {
    let capability_service = Arc::new(PluginCapabilityService::in_memory());
    let control = PluginControlService::builder()
        .capability_service(Arc::clone(&capability_service))
        .build();
    let manifest = manifest("plugin.control-sync", "capability.control-sync");
    control
        .install(PluginInstallRequest {
            location: PluginPackageLocation::new(
                PluginInstallSourceKind::UserLocal,
                "/tmp/plugin-control-sync",
            ),
            manifest: Some(manifest.clone()),
            enable_after_install: true,
            trace: TraceContext::new("trace-install"),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    let before = capability_service
        .query(PluginCapabilityQueryCommand {
            kind: None,
            plugin_id: Some(manifest.plugin_id.clone()),
            slot_namespace: None,
            slot_key: None,
            include_inactive: false,
            trace: TraceContext::new("trace-query-before"),
        })
        .await
        .unwrap();
    assert_eq!(before.len(), 1);

    control
        .disable(PluginTargetCommand::new(
            manifest.plugin_id.clone(),
            TraceContext::new("trace-disable"),
        ))
        .await
        .unwrap();

    let after = capability_service
        .query(PluginCapabilityQueryCommand {
            kind: None,
            plugin_id: Some(manifest.plugin_id),
            slot_namespace: None,
            slot_key: None,
            include_inactive: false,
            trace: TraceContext::new("trace-query-after"),
        })
        .await
        .unwrap();
    assert!(after.is_empty());
}
