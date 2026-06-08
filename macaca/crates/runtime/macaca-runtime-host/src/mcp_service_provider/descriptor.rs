//! Provider-neutral MCP system service descriptor factory.
//!
//! **Pattern:** Factory — builds the static capability manifest consumed by the
//! service registry without coupling to any application runtime.

use macaca_proto::{
    CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor, ServiceHealth,
    ServiceScope, ServiceType, TraceSchemaRef, MCP_SERVICE_ID,
};

/// Build the provider-neutral descriptor for the MCP system service.
pub fn mcp_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(MCP_SERVICE_ID),
        ServiceType::new("mcp"),
        TraceSchemaRef::new("trace.system_service.mcp.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.register"),
            "Registers provider-neutral MCP server definitions.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.probe"),
            "Probes MCP dependency and lifecycle status without leaking secrets.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.tool.catalog"),
            "Reports sanitized MCP tool metadata.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.tool.invoke"),
            "Invokes MCP tools through descriptor-routed service dispatch.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.cleanup"),
            "Cleans MCP resources through explicit lifecycle scope.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.mcp.operator_lifecycle"),
            "Manages MCP reload, OAuth, resources, diagnostics, and exposure refresh.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![
        ServiceScope::Global,
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.required_permissions = vec![
        "mcp.register".into(),
        "mcp.probe".into(),
        "mcp.tool.catalog".into(),
        "mcp.tool.invoke".into(),
        "mcp.server.reload".into(),
        "mcp.oauth.login".into(),
        "mcp.resource.read".into(),
        "mcp.diagnostics.snapshot".into(),
        "mcp.cleanup".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::Always;
    descriptor
}
