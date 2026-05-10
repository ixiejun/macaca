use super::*;
use crate::{ServiceLifecycleState, ServiceType, TraceSchemaRef};

fn fixture_manifest(category: PluginServiceCategory) -> PluginManifest {
    let service_type = ServiceType::new(category.to_string());
    let service = ServiceDescriptor::new(
        KernelServiceId::new(format!("service.{}", category)),
        service_type,
        TraceSchemaRef::new(format!("trace.plugin.{}", category)),
    );
    let mut manifest = PluginManifest::new(
        PluginId::new(format!("plugin.{}", category)),
        PluginVersion::new("1.0.0"),
        DeveloperId::new("developer.local"),
        PluginRuntimeDeclaration::new(PluginRuntimeKind::DescriptorOnly, "1"),
    );
    manifest.provides_services.push(
        PluginProvidedService::new(service, category.clone())
            .with_required_permission(format!("permission.{}", category)),
    );
    manifest
        .provides_capabilities
        .push(PluginProvidedCapability {
            id: CapabilityId::new(format!("capability.{}", category)),
            service: None,
            category,
            description: "Fixture plugin capability".into(),
            required_permissions: vec!["permission.fixture".into()],
            metadata: BTreeMap::new(),
        });
    manifest.permissions.push(PluginPermission {
        id: "permission.fixture".into(),
        reason: "Fixture permission required by tests".into(),
        optional: false,
        metadata: BTreeMap::new(),
    });
    manifest.resources.push(PluginResource {
        id: "resource.fixture".into(),
        kind: "workspace".into(),
        reason: "Fixture resource required by tests".into(),
        optional: false,
        metadata: BTreeMap::new(),
    });
    manifest.signature = Some(PluginSignature {
        algorithm: "fixture".into(),
        key_id: "fixture-key".into(),
        digest: "fixture-digest".into(),
        verification_status: "verification_pending".into(),
    });
    manifest
}

impl PluginProvidedService {
    fn with_required_permission(mut self, permission: impl Into<String>) -> Self {
        self.required_permissions.push(permission.into());
        self
    }
}

#[test]
fn gateway_driver_memory_and_skill_manifests_round_trip() {
    for category in [
        PluginServiceCategory::Gateway,
        PluginServiceCategory::Driver,
        PluginServiceCategory::Memory,
        PluginServiceCategory::Mcp,
        PluginServiceCategory::Skill,
    ] {
        let manifest = fixture_manifest(category);
        let encoded = serde_json::to_string(&manifest).unwrap();
        let decoded: PluginManifest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.plugin_id, manifest.plugin_id);
        assert_eq!(decoded.provides_services.len(), 1);
        assert_eq!(
            decoded.provides_services[0].service.lifecycle_state,
            ServiceLifecycleState::Registered
        );
    }
}

#[test]
fn unsupported_runtime_kind_remains_structured() {
    let manifest = PluginManifest::new(
        PluginId::new("plugin.future"),
        PluginVersion::new("1.0.0"),
        DeveloperId::new("developer.local"),
        PluginRuntimeDeclaration::new(PluginRuntimeKind::Wasm, "1"),
    );
    let decoded: PluginManifest =
        serde_json::from_str(&serde_json::to_string(&manifest).unwrap()).unwrap();
    assert_eq!(decoded.runtime.kind, PluginRuntimeKind::Wasm);
    assert!(!decoded.runtime.kind.is_supported_v0());
}

#[test]
fn lifecycle_event_preserves_traceable_scope() {
    let manifest = fixture_manifest(PluginServiceCategory::Compliance);
    let event = PluginLifecycleEvent::new(
        &manifest,
        Some(PluginLifecycleState::Installed),
        PluginLifecycleState::Registered,
        "register",
        "ok",
        Some(TraceContext::new("plugin-trace")),
    );
    assert_eq!(event.plugin_id, manifest.plugin_id);
    assert_eq!(event.service_ids.len(), 1);
    assert_eq!(event.status, "ok");
    assert!(event.trace.is_some());
}
