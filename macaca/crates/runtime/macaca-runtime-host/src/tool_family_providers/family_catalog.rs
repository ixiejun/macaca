//! Data catalog for built-in industrial tool families (Table Data pattern).
//!
//! `family_specs()` is the single enumeration point for the generic tool surface.
//! Planning, inventory assembly, and descriptor builders all consume this table so
//! provider-specific behavior stays in owning services rather than control-flow here.

use macaca_proto::{
    ToolArtifactPolicy, ToolExecutorRouteKind, ToolResultClass, ToolSideEffectClass,
    ToolTrustLevel, ENTITLEMENT_SERVICE_ID, MCP_SERVICE_ID, SCHEDULER_SERVICE_ID, TOOL_SERVICE_ID,
};

use super::constants::{
    TASK_SERVICE_ID, TOOL_MANAGED_GATEWAY_SERVICE_ID, TOOL_PLUGIN_SERVICE_ID,
    TOOL_RUNTIME_ENVIRONMENT_SERVICE_ID,
};

/// One row in the built-in industrial tool-family catalog.
///
/// Each row describes how `service.tool` should advertise and route a family without
/// embedding raw provider configuration or application-specific payloads.
#[derive(Debug, Clone)]
pub(crate) struct FamilySpec {
    /// Stable family identifier exposed to tool planning requests.
    pub family: &'static str,
    /// Generic service boundary that owns concrete invocation behavior.
    pub owner_service: String,
    /// Provider-neutral descriptor identity used for audits and diagnostics.
    pub provider_id: String,
    /// Capability identity used by service.tool routing and policy layers.
    pub capability_id: String,
    /// Default tool name advertised for the family-level descriptor.
    pub tool_name: String,
    /// Sanitized extension class, never a raw provider implementation detail.
    pub provider_path: &'static str,
    /// Typed execution route used by service.tool invocation dispatch.
    pub route_kind: ToolExecutorRouteKind,
    /// Registry seam where a real provider may be installed by platform code.
    pub extension_point: &'static str,
    /// Declares how service.tool should classify successful outputs.
    pub result_class: ToolResultClass,
    /// Declares whether planning must treat invocation as mutating/external.
    pub side_effect_class: ToolSideEffectClass,
    /// Shared artifact policy keeps oversized outputs out of inline audit data.
    pub artifact_policy: ToolArtifactPolicy,
    /// Optional families begin unavailable until a concrete provider is healthy.
    pub trust_level: ToolTrustLevel,
    /// Marks families that require an external platform/plugin registration.
    pub unsupported_platform: bool,
}

impl FamilySpec {
    /// Returns a stable availability label for governance inventory and sanitized metadata.
    pub(crate) fn availability_state(&self) -> String {
        if self.unsupported_platform {
            "unsupported_until_provider_registered".into()
        } else {
            "service_health_required".into()
        }
    }
}

/// Return the complete built-in family catalog.
///
/// Keeping the catalog as a single data table lets inventory, contributor, and descriptor
/// builders share one source of truth without provider-name control flow.
pub(crate) fn family_specs() -> Vec<FamilySpec> {
    vec![
        spec(
            "file",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::BinaryArtifact,
            ToolSideEffectClass::Write,
        ),
        spec(
            "sandbox",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::ReadOnly,
        ),
        spec(
            "shell",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::Process,
        ),
        spec(
            "approval",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::ApprovalRequest,
            ToolSideEffectClass::External,
        ),
        spec(
            "hook",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::External,
        ),
        spec(
            "config",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::Write,
        ),
        spec(
            "plugin_marketplace",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::External,
        ),
        spec(
            "browser",
            "gateway",
            ToolExecutorRouteKind::ManagedGateway,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::External,
        ),
        spec(
            "web",
            "gateway",
            ToolExecutorRouteKind::ManagedGateway,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::Network,
        ),
        spec(
            "memory",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::ReadOnly,
        ),
        spec(
            "knowledge",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::ReadOnly,
        ),
        spec(
            "code_intelligence",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::ReadOnly,
        ),
        spec(
            "git",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::Write,
        ),
        spec(
            "review",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::Write,
        ),
        spec(
            "task",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::Write,
        ),
        spec(
            "scheduler",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::BackgroundHandle,
            ToolSideEffectClass::Write,
        ),
        spec(
            "skill",
            "owning_service",
            ToolExecutorRouteKind::Skill,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::External,
        ),
        spec(
            "mcp",
            "mcp",
            ToolExecutorRouteKind::Mcp,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::External,
        ),
        spec(
            "media",
            "gateway",
            ToolExecutorRouteKind::ManagedGateway,
            ToolResultClass::Multimodal,
            ToolSideEffectClass::External,
        ),
        spec(
            "document",
            "runtime_adapter",
            ToolExecutorRouteKind::RuntimeEnvironment,
            ToolResultClass::BinaryArtifact,
            ToolSideEffectClass::Write,
        ),
        spec(
            "communication",
            "gateway",
            ToolExecutorRouteKind::ManagedGateway,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::External,
        ),
        spec(
            "enterprise_api",
            "gateway",
            ToolExecutorRouteKind::ManagedGateway,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::External,
        ),
        spec(
            "code_execution",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::Process,
        ),
        FamilySpec {
            unsupported_platform: true,
            ..spec(
                "computer_use",
                "plugin",
                ToolExecutorRouteKind::Plugin,
                ToolResultClass::StructuredJson,
                ToolSideEffectClass::External,
            )
        },
        spec(
            "payment_entitlement",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::External,
        ),
        spec(
            "diagnostics",
            "owning_service",
            ToolExecutorRouteKind::OwningServiceCommand,
            ToolResultClass::StructuredJson,
            ToolSideEffectClass::ReadOnly,
        ),
        FamilySpec {
            unsupported_platform: true,
            ..spec(
                "realtime",
                "owning_service",
                ToolExecutorRouteKind::OwningServiceCommand,
                ToolResultClass::StructuredJson,
                ToolSideEffectClass::External,
            )
        },
        FamilySpec {
            unsupported_platform: true,
            ..spec(
                "remote_environment",
                "owning_service",
                ToolExecutorRouteKind::OwningServiceCommand,
                ToolResultClass::StructuredJson,
                ToolSideEffectClass::External,
            )
        },
    ]
}

/// Build a default catalog row for a family with standard trust and extension metadata.
pub(crate) fn spec(
    family: &'static str,
    provider_path: &'static str,
    route_kind: ToolExecutorRouteKind,
    result_class: ToolResultClass,
    side_effect_class: ToolSideEffectClass,
) -> FamilySpec {
    let owner_service = owner_service_for(family, &route_kind);
    FamilySpec {
        family,
        owner_service: owner_service.clone(),
        provider_id: format!("provider.tool.family.{family}"),
        capability_id: format!("capability.tool.family.{family}"),
        tool_name: format!("{family}_tool"),
        provider_path,
        route_kind,
        extension_point: "service_or_extension_registry",
        result_class,
        side_effect_class,
        artifact_policy: ToolArtifactPolicy::PersistOversized,
        trust_level: ToolTrustLevel::Unavailable,
        unsupported_platform: false,
    }
}

/// Resolve the owning service identifier from family name and route kind.
///
/// This is the Strategy mapping layer: route kind selects the resolution table while
/// family name picks the concrete service boundary for OwningServiceCommand routes.
pub(crate) fn owner_service_for(family: &str, route_kind: &ToolExecutorRouteKind) -> String {
    match route_kind {
        ToolExecutorRouteKind::Driver => macaca_driver::DRIVER_SERVICE_ID.into(),
        ToolExecutorRouteKind::Skill => macaca_skill::SKILL_SERVICE_ID.into(),
        ToolExecutorRouteKind::Mcp => MCP_SERVICE_ID.into(),
        ToolExecutorRouteKind::RuntimeEnvironment => TOOL_RUNTIME_ENVIRONMENT_SERVICE_ID.into(),
        ToolExecutorRouteKind::ManagedGateway => TOOL_MANAGED_GATEWAY_SERVICE_ID.into(),
        ToolExecutorRouteKind::Plugin => TOOL_PLUGIN_SERVICE_ID.into(),
        ToolExecutorRouteKind::Unavailable => format!("{TOOL_SERVICE_ID}.unavailable"),
        ToolExecutorRouteKind::OwningServiceCommand => match family {
            "memory" => macaca_memory::MEMORY_SERVICE_ID.into(),
            "file" => macaca_proto::workbench::file::SERVICE_ID.into(),
            "sandbox" => macaca_proto::workbench::sandbox::SERVICE_ID.into(),
            "shell" | "code_execution" => macaca_proto::workbench::process::SERVICE_ID.into(),
            "approval" => macaca_proto::workbench::approval::SERVICE_ID.into(),
            "hook" => macaca_proto::workbench::hook::SERVICE_ID.into(),
            "config" => macaca_proto::workbench::config::SERVICE_ID.into(),
            "plugin_marketplace" => macaca_proto::workbench::plugin_marketplace::SERVICE_ID.into(),
            "knowledge" => macaca_context::CONTEXT_SERVICE_ID.into(),
            "code_intelligence" => macaca_proto::workbench::code_intelligence::SERVICE_ID.into(),
            "git" => macaca_proto::workbench::git::SERVICE_ID.into(),
            "review" => macaca_proto::workbench::review::SERVICE_ID.into(),
            "diagnostics" => macaca_proto::workbench::diagnostics::SERVICE_ID.into(),
            "realtime" => macaca_proto::workbench::realtime::SERVICE_ID.into(),
            "remote_environment" => macaca_proto::workbench::remote_environment::SERVICE_ID.into(),
            "task" => TASK_SERVICE_ID.into(),
            "scheduler" => SCHEDULER_SERVICE_ID.into(),
            "payment_entitlement" => ENTITLEMENT_SERVICE_ID.into(),
            _ => TOOL_SERVICE_ID.into(),
        },
    }
}
