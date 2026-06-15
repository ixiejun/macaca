//! **Strategy** chain that resolves MCP stdio launch plans from skill snapshots.
//!
//! Explicit `mcp_servers` blocks in skill metadata take precedence; bundled mapping registry
//! entries provide deterministic launch plans for skills that declare install specs only.

use std::collections::HashSet;

use macaca_host_composition::mcp_runtime::apply_concurrency_isolation;
use macaca_host_composition::mcp_runtime::skill_mcp_mapping_registry::default_skill_mcp_mapping_registry;
use macaca_host_composition::runtime_host::{
    SkillMcpServerConfig, SkillSnapshot, SkillSnapshotEntry,
};

use super::types::SkillMcpServerLaunch;

/// Collect deduplicated stdio launch plans for every visible skill row.
pub(crate) fn resolve_skill_mcp_servers(snapshot: &SkillSnapshot) -> Vec<SkillMcpServerLaunch> {
    let mut launches = Vec::new();
    let mut seen = HashSet::new();
    for skill in &snapshot.skills {
        for server in &skill.mcp_servers {
            if let Some(launch) = launch_from_explicit_server(skill, server) {
                if seen.insert(format!("{}:{}", launch.skill_name, launch.server_id)) {
                    launches.push(launch);
                }
            }
        }
        if let Some(launch) = launch_from_skill_mapping_registry(skill) {
            if seen.insert(format!("{}:{}", launch.skill_name, launch.server_id)) {
                launches.push(launch);
            }
        }
    }
    launches
}

fn launch_from_explicit_server(
    skill: &SkillSnapshotEntry,
    server: &SkillMcpServerConfig,
) -> Option<SkillMcpServerLaunch> {
    if !server.transport.eq_ignore_ascii_case("stdio") {
        return None;
    }
    Some(SkillMcpServerLaunch {
        skill_name: skill.name.clone(),
        server_id: server.id.clone(),
        command: server.command.clone(),
        args: server.args.clone(),
    })
}

fn launch_from_skill_mapping_registry(skill: &SkillSnapshotEntry) -> Option<SkillMcpServerLaunch> {
    let registry = default_skill_mcp_mapping_registry();
    let entry = registry.resolve_for_skill(skill)?;
    if !entry.server.transport.eq_ignore_ascii_case("stdio") {
        return None;
    }
    let args = match entry.concurrency_isolation.as_ref() {
        Some(iso) => apply_concurrency_isolation(&iso.policy(), entry.server.args.clone()),
        None => entry.server.args.clone(),
    };
    Some(SkillMcpServerLaunch {
        skill_name: skill.name.clone(),
        server_id: entry.id.clone(),
        command: entry.server.command.clone(),
        args,
    })
}

/// Reverse lookup: map a runtime server id back to the originating skill launch plan.
pub(super) fn launch_from_runtime_server_id(
    snapshot: &SkillSnapshot,
    server_id: &str,
) -> Option<SkillMcpServerLaunch> {
    resolve_skill_mcp_servers(snapshot)
        .into_iter()
        .find(|launch| server_id == format!("skill:{}:{}", launch.skill_name, launch.server_id))
}
