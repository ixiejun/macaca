//! MCP dependency probe and toolkit registration (Adapter to framework + shell).

use std::sync::Arc;

use macaca_sdk::framework::tool::Toolkit;
use macaca_proto::ApplicationId;
use macaca_sdk::runtime_host::{probe_definition_statuses, McpRuntimeStatusState, McpToolPolicy};
use macaca_sdk::skill::SkillSnapshot;

use crate::state::AppState;

use super::server_resolution::launch_from_runtime_server_id;
use super::snapshot::load_or_build_skill_snapshot;
use super::types::{SkillMcpStatus, SkillMcpStatusState};

/// Register MCP tools backed by visible AgentSkills for one traced agent.
pub(crate) async fn register_skill_backed_mcp_tools(
    toolkit: &mut Toolkit,
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<&str>,
) {

    let Some(snapshot) = load_or_build_skill_snapshot(state, app_id, agent_name, session_id).await
    else {
        return;
    };
    let definitions = macaca_sdk::runtime_host::McpServerFactory::with_bundled_mapping_registry()
        .from_skill_snapshot(&snapshot);
    crate::mcp_shell_adapter::register_skill_mcp_definitions(
        state,
        toolkit,
        definitions,
        app_id,
        agent_name,
        session_id,
    )
    .await;

}

/// Probe skill-declared MCP servers without registering tools (status-only path).
pub(crate) async fn probe_skill_mcp_servers(snapshot: &SkillSnapshot) -> Vec<SkillMcpStatus> {

    let definitions = macaca_sdk::runtime_host::McpServerFactory::with_bundled_mapping_registry()
        .from_skill_snapshot(snapshot);
    let statuses = probe_definition_statuses(definitions, &McpToolPolicy::default()).await;
    statuses
        .into_iter()
        .map(|status| {
            let launch = launch_from_runtime_server_id(snapshot, &status.server_id);
            SkillMcpStatus {
                skill: launch
                    .as_ref()
                    .map(|launch| launch.skill_name.clone())
                    .unwrap_or_default(),
                server_id: status.server_id,
                command: launch
                    .as_ref()
                    .map(|launch| launch.command.clone())
                    .unwrap_or_default(),
                args: launch.map(|launch| launch.args).unwrap_or_default(),
                state: match status.state {
                    McpRuntimeStatusState::Ready => SkillMcpStatusState::Ready,
                    McpRuntimeStatusState::DependencyMissing => {
                        SkillMcpStatusState::DependencyMissing
                    }
                    McpRuntimeStatusState::Failed | McpRuntimeStatusState::Disabled => {
                        SkillMcpStatusState::Failed
                    }
                },
                exposed_tools: status.exposed_tools,
                failure_reason: status.failure_reason,
            }
        })
        .collect()

}
