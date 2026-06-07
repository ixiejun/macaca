//! Session event emission for skill-backed MCP lifecycle (audit/trace surface).

use std::sync::Arc;

use crate::runtime_event_bridge::emit_runtime_event;
use crate::state::AppState;

use super::types::SkillMcpServerLaunch;

/// Emit a structured runtime event when skill-backed MCP servers transition.
pub(super) async fn emit_skill_mcp_event(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    agent_name: &str,
    event_type: &str,
    launch: &SkillMcpServerLaunch,
    extra: serde_json::Value,
) {

    tracing::info!(
        agent = %agent_name,
        skill = %launch.skill_name,
        server = %launch.server_id,
        event = %event_type,
        "skill-backed MCP event"
    );
    let Some(session_id) = session_id else {
        return;
    };
    let mut payload = serde_json::json!({
        "agent": agent_name,
        "skill": launch.skill_name,
        "server_id": launch.server_id,
        "command": launch.command,
        "args": launch.args,
    });
    if let (Some(target), Some(extra)) = (payload.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    emit_runtime_event(
        state,
        session_id,
        event_type,
        agent_name,
        Some(agent_name),
        payload,
    )
    .await;

}
