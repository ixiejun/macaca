use std::collections::BTreeMap;

use crate::{
    CapabilityId, KernelServiceId, PluginCapabilityDescriptor, PluginCapabilityKind,
    PluginCapabilityPermissionHint, PluginCapabilityResourceHint, PluginCapabilitySchema,
    PluginCapabilitySlot, PluginCapabilityVisibility, PluginId, ServiceScope, TraceSchemaRef,
};

#[test]
fn plugin_capability_descriptor_round_trips_through_serde() {
    let mut descriptor = PluginCapabilityDescriptor::new(
        CapabilityId::new("capability.plugin.tool.echo"),
        PluginId::new("plugin.echo"),
        PluginCapabilityKind::Tool,
        "tool.echo",
        TraceSchemaRef::new("trace.plugin.capability.tool.v1"),
    );
    descriptor.service = Some(KernelServiceId::new("service.plugin.echo"));
    descriptor.description = "Echo tool descriptor".into();
    descriptor.visibility = PluginCapabilityVisibility::Public;
    descriptor.scopes = vec![
        ServiceScope::Global,
        ServiceScope::Application("app".into()),
    ];
    descriptor.input_schema = Some(PluginCapabilitySchema::referenced(
        "echo.input",
        "schema.echo.input.v1",
    ));
    descriptor.output_schema = Some(PluginCapabilitySchema::referenced(
        "echo.output",
        "schema.echo.output.v1",
    ));
    descriptor
        .permission_hints
        .push(PluginCapabilityPermissionHint {
            permission: "permission.tool.echo".into(),
            required: true,
            reason: "Tool execution requires permission admission".into(),
            metadata: BTreeMap::new(),
        });
    descriptor
        .resource_hints
        .push(PluginCapabilityResourceHint {
            resource: "resource.tool.echo".into(),
            kind: "cpu".into(),
            required: true,
            reason: "Tool execution consumes runtime resources".into(),
            metadata: BTreeMap::new(),
        });
    descriptor
        .slots
        .push(PluginCapabilitySlot::new("tool_name", "echo", true));
    descriptor
        .metadata
        .insert("contract".into(), "fixture".into());

    let encoded = serde_json::to_string(&descriptor).unwrap();
    let decoded: PluginCapabilityDescriptor = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.capability_id, descriptor.capability_id);
    assert_eq!(decoded.plugin_id, descriptor.plugin_id);
    assert_eq!(decoded.kind, PluginCapabilityKind::Tool);
    assert_eq!(decoded.service, descriptor.service);
    assert_eq!(decoded.visibility, PluginCapabilityVisibility::Public);
    assert_eq!(decoded.permission_hints.len(), 1);
    assert_eq!(decoded.resource_hints.len(), 1);
    assert_eq!(
        decoded.trace_schema.as_str(),
        "trace.plugin.capability.tool.v1"
    );
    assert_eq!(decoded.slots[0].namespace, "tool_name");
    assert_eq!(
        decoded.metadata.get("contract").map(String::as_str),
        Some("fixture")
    );
}
