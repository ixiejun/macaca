//! Contract-first discovery for Plugin Capability Registry v1.
//!
//! Discovery converts manifests and built-in service descriptors into canonical
//! `PluginCapabilityDescriptor` values.  It never starts WASM, processes,
//! native code, remote proxies, or provider-specific clients.

use macaca_proto::{
    CapabilityId, PluginCapabilityDescriptor, PluginCapabilityKind, PluginCapabilitySlot,
    PluginCapabilityVisibility, PluginManifest, PluginProvidedCapability, PluginServiceCategory,
    ServiceDescriptor, TraceSchemaRef,
};

/// Discover provider-neutral capability descriptors from a plugin manifest.
pub fn discover_manifest_capabilities(
    manifest: &PluginManifest,
) -> Vec<PluginCapabilityDescriptor> {
    manifest
        .provides_capabilities
        .iter()
        .map(|capability| descriptor_from_provided(manifest, capability))
        .collect()
}

/// Canonicalize a built-in service descriptor as a plugin capability.
///
/// The caller supplies the plugin identity to avoid hardcoding provider or
/// deployment names in runtime-host.  The descriptor keeps behavior unchanged:
/// it only publishes discoverable metadata for existing service implementations.
pub fn built_in_service_capability(
    plugin_id: macaca_proto::PluginId,
    service: &ServiceDescriptor,
    category: PluginServiceCategory,
) -> PluginCapabilityDescriptor {
    let capability_id = service
        .capabilities
        .first()
        .map(|capability| capability.id.clone())
        .unwrap_or_else(|| CapabilityId::new(format!("capability.{}", service.id)));
    let mut descriptor = PluginCapabilityDescriptor::new(
        capability_id,
        plugin_id,
        capability_kind(&category),
        service.id.as_str(),
        service.trace_schema.clone(),
    );
    descriptor.service = Some(service.id.clone());
    descriptor.description = service
        .capabilities
        .first()
        .map(|capability| capability.description.clone())
        .unwrap_or_else(|| format!("Built-in {} capability", service.service_type));
    descriptor.visibility = PluginCapabilityVisibility::Workspace;
    descriptor.scopes = service.supported_scopes.clone();
    descriptor.slots.push(PluginCapabilitySlot::new(
        format!("{}_service", category),
        service.id.as_str(),
        true,
    ));
    descriptor
        .metadata
        .insert("adapter_kind".into(), "built_in".into());
    descriptor
}

fn descriptor_from_provided(
    manifest: &PluginManifest,
    capability: &PluginProvidedCapability,
) -> PluginCapabilityDescriptor {
    let mut descriptor = PluginCapabilityDescriptor::new(
        capability.id.clone(),
        manifest.plugin_id.clone(),
        capability_kind(&capability.category),
        capability.id.as_str(),
        TraceSchemaRef::new(
            capability
                .metadata
                .get("trace_schema")
                .cloned()
                .unwrap_or_else(|| "trace.plugin.capability.v1".into()),
        ),
    );
    descriptor.service = capability.service.clone();
    descriptor.description = capability.description.clone();
    descriptor.permission_hints = capability
        .required_permissions
        .iter()
        .map(|permission| macaca_proto::PluginCapabilityPermissionHint {
            permission: permission.clone(),
            required: true,
            reason: "Declared by plugin-provided capability".into(),
            metadata: Default::default(),
        })
        .collect();
    descriptor.metadata = capability.metadata.clone();
    descriptor.slots = default_slots(&descriptor);
    descriptor
}

fn capability_kind(category: &PluginServiceCategory) -> PluginCapabilityKind {
    match category {
        PluginServiceCategory::Gateway => PluginCapabilityKind::Gateway,
        PluginServiceCategory::Driver => PluginCapabilityKind::Driver,
        PluginServiceCategory::Memory => PluginCapabilityKind::Memory,
        PluginServiceCategory::Context => PluginCapabilityKind::Context,
        PluginServiceCategory::Skill => PluginCapabilityKind::Skill,
        PluginServiceCategory::Mcp => PluginCapabilityKind::Mcp,
        PluginServiceCategory::Payment => PluginCapabilityKind::Custom("payment".into()),
        PluginServiceCategory::Compliance => PluginCapabilityKind::Custom("compliance".into()),
        PluginServiceCategory::Custom(value) => PluginCapabilityKind::Custom(value.clone()),
    }
}

fn default_slots(descriptor: &PluginCapabilityDescriptor) -> Vec<PluginCapabilitySlot> {
    let namespace = match &descriptor.kind {
        PluginCapabilityKind::Tool => "tool_name",
        PluginCapabilityKind::Gateway => "gateway_route",
        PluginCapabilityKind::HttpRoute => "http_route",
        PluginCapabilityKind::CliCommand => "cli_command",
        PluginCapabilityKind::Memory => "memory_provider",
        PluginCapabilityKind::Context => "context_provider",
        PluginCapabilityKind::LlmProvider => "llm_provider",
        PluginCapabilityKind::Custom(value) => value.as_str(),
        _ => return Vec::new(),
    };
    vec![PluginCapabilitySlot::new(
        namespace,
        descriptor.name.clone(),
        true,
    )]
}
