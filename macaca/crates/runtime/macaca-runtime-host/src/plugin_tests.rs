use super::*;
use macaca_proto::{
    CapabilityId, DeveloperId, KernelServiceId, PluginId, PluginPermission,
    PluginProvidedCapability, PluginProvidedService, PluginResource, PluginRuntimeDeclaration,
    PluginServiceCategory, PluginSignature, PluginVersion, ServiceDescriptor, ServiceType,
    TraceSchemaRef,
};

fn manifest(runtime_kind: PluginRuntimeKind) -> PluginManifest {
    let mut service = ServiceDescriptor::new(
        KernelServiceId::new("service.plugin.gateway"),
        ServiceType::new("gateway"),
        TraceSchemaRef::new("trace.plugin.gateway"),
    );
    service
        .required_permissions
        .push("permission.gateway".into());
    let mut manifest = PluginManifest::new(
        PluginId::new("plugin.gateway"),
        PluginVersion::new("1.0.0"),
        DeveloperId::new("developer.local"),
        PluginRuntimeDeclaration::new(runtime_kind, "1"),
    );
    manifest.provides_services.push(PluginProvidedService::new(
        service,
        PluginServiceCategory::Gateway,
    ));
    manifest
        .provides_capabilities
        .push(PluginProvidedCapability {
            id: CapabilityId::new("capability.gateway"),
            service: Some(KernelServiceId::new("service.plugin.gateway")),
            category: PluginServiceCategory::Gateway,
            description: "Gateway descriptor".into(),
            required_permissions: vec!["permission.gateway".into()],
            metadata: Default::default(),
        });
    manifest.permissions.push(PluginPermission {
        id: "permission.gateway".into(),
        reason: "Gateway permission".into(),
        optional: false,
        metadata: Default::default(),
    });
    manifest.resources.push(PluginResource {
        id: "resource.gateway".into(),
        kind: "network".into(),
        reason: "Gateway resource".into(),
        optional: false,
        metadata: Default::default(),
    });
    manifest.signature = Some(PluginSignature {
        algorithm: "fixture".into(),
        key_id: "fixture-key".into(),
        digest: "fixture-digest".into(),
        verification_status: "verification_pending".into(),
    });
    manifest
}

fn built_in_manifest_shell() -> PluginManifest {
    let mut manifest = PluginManifest::new(
        PluginId::new("plugin.builtin"),
        PluginVersion::new("1.0.0"),
        DeveloperId::new("developer.local"),
        PluginRuntimeDeclaration::new(PluginRuntimeKind::BuiltInAdapter, "1"),
    );
    manifest.permissions.push(PluginPermission {
        id: "permission.service".into(),
        reason: "Built-in descriptor permission".into(),
        optional: false,
        metadata: Default::default(),
    });
    manifest.signature = Some(PluginSignature {
        algorithm: "fixture".into(),
        key_id: "fixture-key".into(),
        digest: "fixture-digest".into(),
        verification_status: "verification_pending".into(),
    });
    manifest
}

fn built_in_descriptor(category: PluginServiceCategory) -> BuiltInPluginDescriptor {
    BuiltInPluginDescriptor::new(
        KernelServiceId::new(format!("service.{category}")),
        category.clone(),
        CapabilityId::new(format!("capability.{category}")),
        "Provider-neutral built-in capability",
        "os-service",
        TraceSchemaRef::new(format!("trace.{category}")),
    )
    .with_required_permission("permission.service")
}

#[test]
fn validator_rejects_missing_permission() {
    let mut manifest = manifest(PluginRuntimeKind::DescriptorOnly);
    manifest.permissions.clear();
    let err = PluginManifestValidator::new()
        .validate(&manifest)
        .unwrap_err();
    assert!(matches!(err, PluginError::MissingPermission(_)));
}

#[test]
fn validator_rejects_unsupported_runtime() {
    let manifest = manifest(PluginRuntimeKind::Wasm);
    let err = PluginManifestValidator::new()
        .validate(&manifest)
        .unwrap_err();
    assert!(matches!(err, PluginError::UnsupportedRuntime(_)));
}

#[test]
fn validator_rejects_missing_resource_declaration() {
    let mut manifest = manifest(PluginRuntimeKind::DescriptorOnly);
    manifest.resources.clear();
    let err = PluginManifestValidator::new()
        .validate(&manifest)
        .unwrap_err();
    assert!(matches!(err, PluginError::MissingResource(_)));
}

#[tokio::test]
async fn facade_installs_registers_starts_stops_and_uninstalls_descriptor_plugin() {
    let manifest = manifest(PluginRuntimeKind::DescriptorOnly);
    let registry = Arc::new(PluginRegistry::new());
    let facade = PluginRuntimeFacade::new(Arc::clone(&registry));

    let events = facade
        .install_and_register(manifest.clone(), Some(TraceContext::new("plugin-trace")))
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].next_state, PluginLifecycleState::Installed);
    assert_eq!(events[1].next_state, PluginLifecycleState::Registered);
    assert_eq!(registry.snapshot().await.service_owners.len(), 1);

    let started = facade.start(&manifest, None).await.unwrap();
    assert_eq!(started.previous_state, Some(PluginLifecycleState::Starting));
    assert_eq!(started.next_state, PluginLifecycleState::Running);

    let stopped = facade.stop(&manifest, None).await.unwrap();
    assert_eq!(stopped.previous_state, Some(PluginLifecycleState::Stopping));
    assert_eq!(stopped.next_state, PluginLifecycleState::Stopped);

    let uninstalled = facade.uninstall(&manifest, None).await.unwrap();
    assert_eq!(
        uninstalled.previous_state,
        Some(PluginLifecycleState::Stopped)
    );
    assert_eq!(uninstalled.next_state, PluginLifecycleState::Uninstalled);
    assert!(registry.snapshot().await.service_owners.is_empty());
}

#[test]
fn failure_event_is_structured_and_auditable() {
    let manifest = manifest(PluginRuntimeKind::DescriptorOnly);
    let event = plugin_failure_event(
        &manifest,
        Some(PluginLifecycleState::Starting),
        "start",
        &PluginError::Unavailable("host unavailable".into()),
        Some(TraceContext::new("plugin-trace")),
    );
    assert_eq!(event.next_state, PluginLifecycleState::Failed);
    assert_eq!(event.status, "error");
    assert!(event.error_code.is_some());
    assert!(event.trace.is_some());
}

#[test]
fn built_in_adapter_descriptors_are_provider_neutral_and_queryable() {
    let manifest = BuiltInPluginDescriptorAdapter::new(built_in_manifest_shell())
        .maybe_add_descriptor(Some(built_in_descriptor(PluginServiceCategory::Gateway)))
        .maybe_add_descriptor(Some(built_in_descriptor(PluginServiceCategory::Driver)))
        .maybe_add_descriptor(Some(built_in_descriptor(PluginServiceCategory::Memory)))
        .maybe_add_descriptor(Some(built_in_descriptor(PluginServiceCategory::Context)))
        .maybe_add_descriptor(Some(built_in_descriptor(PluginServiceCategory::Skill)))
        .maybe_add_descriptor(Some(built_in_descriptor(PluginServiceCategory::Mcp)))
        .build();

    assert_eq!(manifest.runtime.kind, PluginRuntimeKind::BuiltInAdapter);
    assert_eq!(manifest.provides_services.len(), 6);
    assert_eq!(manifest.provides_capabilities.len(), 6);
    assert!(manifest.provides_services.iter().all(|service| {
        service.metadata.get("adapter_kind").map(String::as_str) == Some("built_in")
    }));
}

#[test]
fn built_in_adapter_absent_service_is_optional() {
    let manifest = BuiltInPluginDescriptorAdapter::new(built_in_manifest_shell())
        .maybe_add_descriptor(None)
        .build();

    assert!(manifest.provides_services.is_empty());
    assert!(manifest.provides_capabilities.is_empty());
    assert!(manifest.resources.is_empty());
}

#[tokio::test]
async fn failed_plugin_cleanup_leaves_no_active_descriptors() {
    let manifest = manifest(PluginRuntimeKind::DescriptorOnly);
    let registry = Arc::new(PluginRegistry::new());
    let facade = PluginRuntimeFacade::new(Arc::clone(&registry));
    facade
        .install_and_register(manifest.clone(), None)
        .await
        .unwrap();

    registry.fail(&manifest.plugin_id, "fixture failure").await;
    registry.uninstall(&manifest.plugin_id).await.unwrap();

    let failure = plugin_failure_event(
        &manifest,
        Some(PluginLifecycleState::Registered),
        "cleanup",
        &PluginError::Unavailable("fixture failure".into()),
        None,
    );
    assert_eq!(failure.next_state, PluginLifecycleState::Failed);
    assert!(registry.snapshot().await.service_owners.is_empty());
}
