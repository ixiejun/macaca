//! Generic industrial tool-family provider catalog.
//!
//! This module is the runtime-host Abstract Factory for the broad Tool
//! Capability Plane.  It does not implement file, shell, browser, payment, or
//! other domain behavior directly.  Instead it creates sanitized descriptor
//! rows that point to owning service, MCP, plugin, gateway, runtime-adapter, or
//! unavailable-provider boundaries.  Planning can therefore expose a complete
//! generic tool surface while invocation still flows through `service.tool`
//! decorators and the owning service route selected by descriptor data.

use std::collections::BTreeMap;

use macaca_proto::{
    AvailabilityExpression, CapabilityToolDescriptor, CapabilityToolOriginKind,
    IndustrialToolDescriptor, MacacaResult, ToolArtifactPolicy, ToolExecutorRouteKind,
    ToolFamilyRef, ToolLifecycleScope, ToolResultClass, ToolSideEffectClass, ToolTrustLevel,
    ToolsetRef, ENTITLEMENT_SERVICE_ID, MCP_SERVICE_ID, PAYMENT_SERVICE_ID,
    SCHEDULED_AGENT_TASK_SERVICE_ID, SCHEDULER_SERVICE_ID, TOOL_SERVICE_ID,
};

use crate::tool_service_availability::AvailabilitySignalSet;
use crate::tool_service_planning::{
    StaticToolDescriptorContributor, ToolPlanningService, ToolPlanningToolsetResolver,
};

const TASK_SERVICE_ID: &str = "service.task";
const TOOL_RUNTIME_ENVIRONMENT_SERVICE_ID: &str = "service.tool.runtime_environment";
const TOOL_MANAGED_GATEWAY_SERVICE_ID: &str = "service.tool.managed_gateway";
const TOOL_PLUGIN_SERVICE_ID: &str = "service.plugin";

/// Stable provider-neutral families required by the industrial Tools proposal.
///
/// The list is deliberately data, not control flow.  Adding a future family
/// should extend this table and associated descriptors rather than adding
/// provider-name branches in planning or invocation.
pub const REQUIRED_INDUSTRIAL_TOOL_FAMILIES: &[&str] = &[
    "file",
    "sandbox",
    "shell",
    "browser",
    "web",
    "memory",
    "knowledge",
    "code_intelligence",
    "git",
    "review",
    "task",
    "scheduler",
    "skill",
    "mcp",
    "media",
    "document",
    "communication",
    "enterprise_api",
    "code_execution",
    "computer_use",
    "payment_entitlement",
];

/// Inventory row used by governance notes, tests, and descriptor generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialToolFamilyProviderInventory {
    pub family: String,
    pub owner_service: String,
    pub provider_id: String,
    pub capability_id: String,
    pub tool_name: String,
    pub provider_path: String,
    pub sanitized_metadata: BTreeMap<String, String>,
}

/// Return the complete family inventory with only sanitized operational data.
pub fn industrial_tool_family_provider_inventory(
) -> MacacaResult<Vec<IndustrialToolFamilyProviderInventory>> {
    let specs = family_specs();
    tracing::info!(
        family_count = specs.len(),
        "industrial tool family provider inventory assembled"
    );
    specs
        .into_iter()
        .map(|spec| {
            // The inventory intentionally mirrors descriptor data without raw
            // provider payloads.  Governance tests can inspect this structure
            // without needing access to provider-specific configuration.
            let mut sanitized_metadata = BTreeMap::new();
            sanitized_metadata.insert("provider_path".into(), spec.provider_path.to_string());
            sanitized_metadata.insert("owner_service".into(), spec.owner_service.clone());
            sanitized_metadata.insert("availability_state".into(), spec.availability_state());
            sanitized_metadata.insert("extension_point".into(), spec.extension_point.to_string());
            Ok(IndustrialToolFamilyProviderInventory {
                family: spec.family.to_string(),
                owner_service: spec.owner_service,
                provider_id: spec.provider_id,
                capability_id: spec.capability_id,
                tool_name: spec.tool_name,
                provider_path: spec.provider_path.to_string(),
                sanitized_metadata,
            })
        })
        .collect()
}

/// Build the descriptor contributor consumed by `ToolPlanningService`.
///
/// The contributor is a Null Object / Adapter bridge: when a concrete optional
/// provider is not installed, its descriptor remains visible to diagnostics but
/// is hidden by availability evaluation instead of pretending that invocation
/// would succeed.
pub fn industrial_tool_family_provider_contributor() -> MacacaResult<StaticToolDescriptorContributor>
{
    let descriptors = family_specs()
        .into_iter()
        .map(spec_to_descriptor)
        .collect::<MacacaResult<Vec<_>>>()?;
    tracing::info!(
        family_count = descriptors.len(),
        "industrial tool family provider contributor assembled"
    );
    Ok(StaticToolDescriptorContributor::new(
        "industrial-tool-family-providers",
        macaca_proto::ServiceHealth::Healthy,
        descriptors,
    ))
}

/// Toolset resolver used by application-neutral plans and integration proof.
pub fn industrial_tool_family_toolsets() -> MacacaResult<ToolPlanningToolsetResolver> {
    Ok(ToolPlanningToolsetResolver::default()
        .with_toolset(
            "industrial.full_stack",
            REQUIRED_INDUSTRIAL_TOOL_FAMILIES.iter().copied(),
        )
        .with_toolset(
            "industrial.proof",
            ["web", "file", "shell", "memory", "document", "scheduler"],
        )
        .with_toolset(
            "industrial.research",
            ["web", "browser", "memory", "knowledge", "document"],
        )
        .with_toolset(
            "industrial.automation",
            ["task", "scheduler", "skill", "mcp", "computer_use"],
        ))
}

/// Build the production industrial planner used by runtime-host bootstraps.
///
/// This is the approved Abstract Factory composition point for the generic
/// industrial tool surface. The planner receives the real family contributor,
/// generic toolsets, and service availability signals for in-process services
/// known to the runtime host. Optional external providers remain structured
/// hidden diagnostics until a plugin, MCP server, gateway, or environment
/// provider contributes a healthy route.
pub fn industrial_tool_planning_service() -> MacacaResult<ToolPlanningService> {
    let mut availability = AvailabilitySignalSet::default();
    for service_id in [
        TOOL_SERVICE_ID,
        TOOL_RUNTIME_ENVIRONMENT_SERVICE_ID,
        TOOL_MANAGED_GATEWAY_SERVICE_ID,
        macaca_memory::MEMORY_SERVICE_ID,
        macaca_context::CONTEXT_SERVICE_ID,
        macaca_proto::workbench::file::SERVICE_ID,
        macaca_proto::workbench::code_intelligence::SERVICE_ID,
        macaca_proto::workbench::git::SERVICE_ID,
        macaca_proto::workbench::review::SERVICE_ID,
        macaca_proto::workbench::sandbox::SERVICE_ID,
        macaca_proto::workbench::process::SERVICE_ID,
        TASK_SERVICE_ID,
        SCHEDULER_SERVICE_ID,
        SCHEDULED_AGENT_TASK_SERVICE_ID,
        macaca_skill::SKILL_SERVICE_ID,
        MCP_SERVICE_ID,
        ENTITLEMENT_SERVICE_ID,
        PAYMENT_SERVICE_ID,
    ] {
        availability = availability.with_service(service_id);
    }
    Ok(ToolPlanningService::builder()
        .with_contributor(industrial_tool_family_provider_contributor()?)
        .with_availability(availability)
        .with_toolsets(industrial_tool_family_toolsets()?)
        .build())
}

#[derive(Debug, Clone)]
struct FamilySpec {
    /// Stable family identifier exposed to tool planning requests.
    family: &'static str,
    /// Generic service boundary that owns concrete invocation behavior.
    owner_service: String,
    /// Provider-neutral descriptor identity used for audits and diagnostics.
    provider_id: String,
    /// Capability identity used by service.tool routing and policy layers.
    capability_id: String,
    /// Default tool name advertised for the family-level descriptor.
    tool_name: String,
    /// Sanitized extension class, never a raw provider implementation detail.
    provider_path: &'static str,
    /// Typed execution route used by service.tool invocation dispatch.
    route_kind: ToolExecutorRouteKind,
    /// Registry seam where a real provider may be installed by platform code.
    extension_point: &'static str,
    /// Declares how service.tool should classify successful outputs.
    result_class: ToolResultClass,
    /// Declares whether planning must treat invocation as mutating/external.
    side_effect_class: ToolSideEffectClass,
    /// Shared artifact policy keeps oversized outputs out of inline audit data.
    artifact_policy: ToolArtifactPolicy,
    /// Optional families begin unavailable until a concrete provider is healthy.
    trust_level: ToolTrustLevel,
    /// Marks families that require an external platform/plugin registration.
    unsupported_platform: bool,
}

impl FamilySpec {
    fn availability_state(&self) -> String {
        if self.unsupported_platform {
            "unsupported_until_provider_registered".into()
        } else {
            "service_health_required".into()
        }
    }
}

fn family_specs() -> Vec<FamilySpec> {
    // This table is the only place where the built-in family catalog is
    // enumerated.  Keeping it as data lets the planner, inventory, and tests
    // consume the same source of truth without provider-name control flow.
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
    ]
}

fn spec(
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

fn owner_service_for(family: &str, route_kind: &ToolExecutorRouteKind) -> String {
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
            "knowledge" => macaca_context::CONTEXT_SERVICE_ID.into(),
            "code_intelligence" => macaca_proto::workbench::code_intelligence::SERVICE_ID.into(),
            "git" => macaca_proto::workbench::git::SERVICE_ID.into(),
            "review" => macaca_proto::workbench::review::SERVICE_ID.into(),
            "task" => TASK_SERVICE_ID.into(),
            "scheduler" => SCHEDULER_SERVICE_ID.into(),
            "payment_entitlement" => ENTITLEMENT_SERVICE_ID.into(),
            _ => TOOL_SERVICE_ID.into(),
        },
    }
}

fn spec_to_descriptor(spec: FamilySpec) -> MacacaResult<IndustrialToolDescriptor> {
    tracing::debug!(
        family = spec.family,
        owner_service = %spec.owner_service,
        provider_path = spec.provider_path,
        "building industrial tool family descriptor"
    );
    let mut base = CapabilityToolDescriptor::new(
        spec.owner_service.clone(),
        spec.provider_id.clone(),
        spec.capability_id.clone(),
        spec.tool_name.clone(),
        format!("Application-neutral {} family tool provider", spec.family),
        serde_json::json!({
            "type": "object",
            "additionalProperties": true,
        }),
        origin_kind_for_route(&spec.route_kind),
    )?;
    base.metadata
        .insert("tool.family".into(), spec.family.into());
    base.metadata.insert(
        "tool.stable_id".into(),
        format!("tool.family.{}.default", spec.family),
    );
    base.metadata.insert(
        "tool.toolsets".into(),
        toolsets_for_family(spec.family).join(","),
    );
    base.metadata
        .insert("provider_path".into(), spec.provider_path.into());
    base.metadata
        .insert("owner_service".into(), spec.owner_service.clone());

    let mut descriptor = IndustrialToolDescriptor::new(
        format!("tool.family.{}.default", spec.family),
        spec.tool_name.clone(),
        format!("{} family provider", spec.family),
        ToolFamilyRef::new(spec.family)?,
        base,
    )?;
    descriptor.executor_route = descriptor
        .executor_route
        .with_route_kind(spec.route_kind.clone(), command_name_for(&spec));
    descriptor.toolsets = toolsets_for_family(spec.family)
        .into_iter()
        .map(ToolsetRef::new)
        .collect::<MacacaResult<Vec<_>>>()?;
    descriptor.lifecycle_scope = ToolLifecycleScope::Session;
    descriptor.side_effect_class = spec.side_effect_class.clone();
    descriptor.result_class = spec.result_class.clone();
    descriptor.artifact_policy = spec.artifact_policy.clone();
    descriptor.trust_level = spec.trust_level.clone();
    descriptor.availability = availability_for(&spec);
    descriptor
        .sanitized_metadata
        .insert("provider_path".into(), spec.provider_path.into());
    descriptor
        .sanitized_metadata
        .insert("owner_service".into(), spec.owner_service.clone());
    descriptor
        .sanitized_metadata
        .insert("availability_state".into(), spec.availability_state());
    descriptor
        .sanitized_metadata
        .insert("route_kind".into(), format!("{:?}", spec.route_kind));
    Ok(descriptor)
}

fn origin_kind_for_route(route_kind: &ToolExecutorRouteKind) -> CapabilityToolOriginKind {
    match route_kind {
        ToolExecutorRouteKind::Driver => CapabilityToolOriginKind::Driver,
        ToolExecutorRouteKind::Skill => CapabilityToolOriginKind::Skill,
        ToolExecutorRouteKind::Mcp => CapabilityToolOriginKind::Mcp,
        // Route kinds outside the historical Driver/Skill/MCP origin surface
        // use MCP-compatible descriptor origin only for legacy serialization.
        // The typed executor route is authoritative for dispatch.
        ToolExecutorRouteKind::OwningServiceCommand
        | ToolExecutorRouteKind::RuntimeEnvironment
        | ToolExecutorRouteKind::ManagedGateway
        | ToolExecutorRouteKind::Plugin
        | ToolExecutorRouteKind::Unavailable => CapabilityToolOriginKind::Mcp,
    }
}

fn command_name_for(spec: &FamilySpec) -> Option<String> {
    match spec.route_kind {
        ToolExecutorRouteKind::RuntimeEnvironment => Some("tool.runtime_environment.invoke".into()),
        ToolExecutorRouteKind::ManagedGateway => Some("tool.managed_gateway.invoke".into()),
        ToolExecutorRouteKind::Plugin => Some("tool.plugin.invoke".into()),
        ToolExecutorRouteKind::OwningServiceCommand => match spec.family {
            "file" => Some(macaca_proto::workbench::file::TOOL_INVOKE_COMMAND.into()),
            "code_intelligence" => {
                Some(macaca_proto::workbench::code_intelligence::SEARCH_COMMAND.into())
            }
            "git" => Some(macaca_proto::workbench::git::STATUS_COMMAND.into()),
            "review" => Some(macaca_proto::workbench::review::START_COMMAND.into()),
            "sandbox" => Some(macaca_proto::workbench::sandbox::TOOL_INVOKE_COMMAND.into()),
            "shell" | "code_execution" => {
                Some(macaca_proto::workbench::process::TOOL_INVOKE_COMMAND.into())
            }
            _ => Some(format!("{}.tool.invoke", spec.family)),
        },
        _ => None,
    }
}

fn availability_for(spec: &FamilySpec) -> Vec<AvailabilityExpression> {
    // Unsupported-platform expressions are a Null Object diagnostic: the
    // family remains visible in hidden catalog output, while service.tool will
    // not route invocations until an owning provider registers a healthy route.
    if spec.unsupported_platform {
        vec![AvailabilityExpression::Platform {
            os: "provider_registered_platform".into(),
        }]
    } else {
        vec![AvailabilityExpression::ServiceHealth {
            service_id: spec.owner_service.clone().into(),
        }]
    }
}

fn toolsets_for_family(family: &str) -> Vec<&'static str> {
    let mut toolsets = vec!["industrial.full_stack"];
    match family {
        "web" | "file" | "sandbox" | "shell" | "memory" | "document" | "scheduler" | "git"
        | "review" => {
            toolsets.push("industrial.proof");
        }
        _ => {}
    }
    match family {
        "web" | "browser" | "memory" | "knowledge" | "document" | "code_intelligence" => {
            toolsets.push("industrial.research");
        }
        _ => {}
    }
    match family {
        "task" | "scheduler" | "skill" | "mcp" | "computer_use" | "review" => {
            toolsets.push("industrial.automation");
        }
        _ => {}
    }
    toolsets
}
