//! MCP shell adapter — service-first MCP toolkit registration.
//!
//! Skill MCP registration still requires the in-process `McpRuntimeFacade` during P3.
//! This adapter isolates that legacy seam so route and toolkit code does not read
//! `AppState::mcp_runtime` directly.

use std::sync::Arc;

use macaca_framework::tool::Toolkit;
use macaca_proto::ApplicationId;
use macaca_runtime_host::{McpRuntimeContext, McpServerDefinition, McpToolPolicy};
use tracing::info;

use crate::state::AppState;

/// Register skill-derived MCP definitions into the framework toolkit.
///
/// Primary path remains the runtime facade until MCP service exposes an equivalent
/// register command for dynamic skill snapshots. The adapter logs trace context for audit.
pub async fn register_skill_mcp_definitions(
    state: &Arc<AppState>,
    toolkit: &mut Toolkit,
    definitions: Vec<McpServerDefinition>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<&str>,
) {
    if definitions.is_empty() {
        return;
    }

    let context = McpRuntimeContext::for_agent(app_id, session_id, agent_name);
    info!(
        app_id = %app_id,
        agent = agent_name,
        session_id = session_id.unwrap_or("skill-mcp-sessionless"),
        definition_count = definitions.len(),
        "mcp shell adapter registering skill MCP definitions via legacy runtime facade"
    );

    let _ = state
        .composition
        .mcp_runtime
        .register_definitions(
            toolkit,
            definitions,
            &McpToolPolicy::default(),
            &context,
            None,
        )
        .await;
}
