//! Abstract Factory composition for industrial tool planning (contributor + toolsets + service).
//!
//! This module is the approved runtime-host bootstrap point for the generic industrial tool
//! surface.  Optional external providers remain structured hidden diagnostics until a plugin,
//! MCP server, gateway, or environment provider contributes a healthy route.

use macaca_proto::{
    MacacaResult, ENTITLEMENT_SERVICE_ID, MCP_SERVICE_ID, PAYMENT_SERVICE_ID,
    SCHEDULED_AGENT_TASK_SERVICE_ID, SCHEDULER_SERVICE_ID, TOOL_SERVICE_ID,
};

use crate::tool_service_availability::AvailabilitySignalSet;
use crate::tool_service_planning::{
    StaticToolDescriptorContributor, ToolPlanningService, ToolPlanningToolsetResolver,
};

use super::constants::{
    REQUIRED_INDUSTRIAL_TOOL_FAMILIES, TASK_SERVICE_ID, TOOL_MANAGED_GATEWAY_SERVICE_ID,
    TOOL_RUNTIME_ENVIRONMENT_SERVICE_ID,
};
use super::descriptor_builder::spec_to_descriptor;
use super::family_catalog::family_specs;

/// Build the descriptor contributor consumed by `ToolPlanningService`.
///
/// The contributor is a Null Object / Adapter bridge: when a concrete optional provider is not
/// installed, its descriptor remains visible to diagnostics but is hidden by availability
/// evaluation instead of pretending that invocation would succeed.
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
        )
        .with_toolset(
            "industrial.workbench",
            [
                "file",
                "sandbox",
                "shell",
                "approval",
                "hook",
                "config",
                "plugin_marketplace",
                "mcp",
                "skill",
                "code_intelligence",
                "git",
                "review",
                "diagnostics",
                "realtime",
                "remote_environment",
            ],
        ))
}

/// Build the production industrial planner used by runtime-host bootstraps.
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
        macaca_proto::workbench::approval::SERVICE_ID,
        macaca_proto::workbench::hook::SERVICE_ID,
        macaca_proto::workbench::config::SERVICE_ID,
        macaca_proto::workbench::plugin_marketplace::SERVICE_ID,
        macaca_proto::workbench::diagnostics::SERVICE_ID,
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
    tracing::info!("industrial tool planning service factory assembling contributor and toolsets");
    Ok(ToolPlanningService::builder()
        .with_contributor(industrial_tool_family_provider_contributor()?)
        .with_availability(availability)
        .with_toolsets(industrial_tool_family_toolsets()?)
        .build())
}
