//! Skill snapshot → MCP server definition resolution.
//!
//! Walks visible skill install specs and applies the declarative compatibility registry.

use std::collections::{BTreeMap, HashSet};

use macaca_framework::mcp::{McpSessionMode, McpTransportConfig};
use macaca_skill::{SkillMcpServerConfig, SkillSnapshot, SkillSnapshotEntry};

use crate::skill_mcp_mapping_registry::{default_skill_mcp_mapping_registry, SkillMcpMappingRegistry};

use super::policy::apply_concurrency_isolation;
use super::types::{McpDefinitionSource, McpLifecycleScope, McpServerDefinition};

/// Resolve MCP definitions declared by a visible skill snapshot, consulting
/// the process-default compatibility registry.
#[deprecated(note = "Use `McpServerFactory::from_skill_snapshot` instead.")]
pub fn definitions_from_skill_snapshot(snapshot: &SkillSnapshot) -> Vec<McpServerDefinition> {
    definitions_from_skill_snapshot_with_registry(snapshot, default_skill_mcp_mapping_registry())
}

/// Resolve MCP definitions with an explicit compatibility registry (for
/// tests and hosts that supply their own override layer).
pub fn definitions_from_skill_snapshot_with_registry(
    snapshot: &SkillSnapshot,
    registry: &SkillMcpMappingRegistry,
) -> Vec<McpServerDefinition> {
    let mut definitions = Vec::new();
    let mut seen = HashSet::new();
    for skill in &snapshot.skills {
        for server in &skill.mcp_servers {
            if let Some(definition) = definition_from_skill_server(skill, server, registry) {
                if seen.insert(definition.id.clone()) {
                    definitions.push(definition);
                }
            }
        }
        if let Some(compat_entry) = registry.resolve_for_skill(skill) {
            let id = format!("skill:{}:{}", skill.name, compat_entry.id);
            if let Some(definition) = compat_entry.to_definition(id) {
                if seen.insert(definition.id.clone()) {
                    definitions.push(definition);
                }
            }
        }
    }
    definitions
}

pub(crate) fn definition_from_skill_server(
    skill: &SkillSnapshotEntry,
    server: &SkillMcpServerConfig,
    registry: &SkillMcpMappingRegistry,
) -> Option<McpServerDefinition> {
    if !server.transport.eq_ignore_ascii_case("stdio") {
        return None;
    }
    let id = format!("skill:{}:{}", skill.name, server.id);
    let policy = registry.policy_for_command(&server.command);
    let args = policy
        .as_ref()
        .map(|p| apply_concurrency_isolation(p, server.args.clone()))
        .unwrap_or_else(|| server.args.clone());
    Some(McpServerDefinition {
        id,
        transport: McpTransportConfig::Stdio {
            command: server.command.clone(),
            args,
            env: BTreeMap::new(),
            cwd: Some(skill.base_dir.clone()),
        },
        lifecycle: McpLifecycleScope::AgentSession,
        session_mode: McpSessionMode::Stateful,
        tool_prefix: server.tool_prefix.clone(),
        required_bins: vec![server.command.clone()],
        enabled: true,
        source: McpDefinitionSource::Skill,
        concurrency_isolation: policy,
    })
}

fn flatten_timeout_result<T>(
    result: Result<Result<T, macaca_framework::mcp::McpError>, tokio::time::error::Elapsed>,
) -> Result<T, String> {
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err("connect_timeout".to_string()),
    }
}
