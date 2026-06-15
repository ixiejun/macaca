//! Skill snapshot load/build with session cache (Cache-Aside pattern).

use std::sync::Arc;

use macaca_host_composition::runtime_host::{
    SkillServiceScope, SkillSnapshot, SkillSnapshotServiceCommand,
};
use macaca_proto::{ApplicationId, TraceContext};

use crate::runtime_event_bridge::{emit_runtime_event, emit_skill_snapshot_event};
use crate::state::AppState;

use super::governance_telemetry::{
    record_governed_skill_snapshot_activation, resolve_agent_skill_policy,
};

/// Load a cached per-agent snapshot or build one via Skill Service.
pub(crate) async fn load_or_build_skill_snapshot(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<&str>,
) -> Option<SkillSnapshot> {
    const TRACE_ID: &str = "web-skill-mcp-snapshot";
    let snapshot_module = format!("skill_snapshot/{agent_name}");
    if let Some(session_id) = session_id {
        if let Some(snapshot) =
            crate::framework_state_memento::load_session_namespace::<SkillSnapshot>(
                state.sessions.framework_session_store.as_ref(),
                &app_id.0.to_string(),
                session_id,
                &snapshot_module,
            )
            .await
        {
            emit_skill_snapshot_event(
                state,
                session_id,
                agent_name,
                "skill_snapshot_cache_hit",
                &snapshot,
                TRACE_ID,
            )
            .await;
            record_governed_skill_snapshot_activation(
                state, app_id, agent_name, session_id, &snapshot, TRACE_ID,
            )
            .await;
            return Some(snapshot);
        }
    }

    let app = {
        let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
        registry.get_app(app_id).cloned()
    }?;
    let workspace_dir = {
        let workspaces = state.config.app_workspaces.read().await;
        workspaces.get(app_id).map(|ws| ws.root.clone())
    };
    let policy = resolve_agent_skill_policy(state, app_id, agent_name).await;
    let app_dir = app.path.clone();
    if let Some(session_id) = session_id {
        emit_runtime_event(
            state,
            session_id,
            "skill_snapshot_build_started",
            agent_name,
            Some(agent_name),
            serde_json::json!({
                "agent": agent_name,
                "trace_id": TRACE_ID,
                "workspace_projected": workspace_dir.is_some(),
            }),
        )
        .await;
    }
    let snapshot = match state
        .skill_client
        .snapshot(SkillSnapshotServiceCommand {
            trace: TraceContext::new(TRACE_ID),
            scope: SkillServiceScope::agent(
                *app_id,
                session_id.unwrap_or("no-session"),
                agent_name,
            )
            .ok()?,
            agent_name: agent_name.to_string(),
            workspace_dir,
            app_dir: Some(app_dir),
            include_instructions: true,
            exposure_policy: policy,
            policy: Default::default(),
        })
        .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tracing::warn!(
                error = %error,
                agent = %agent_name,
                "Skill Service snapshot failed; no shell-local fallback is allowed"
            );
            if let Some(session_id) = session_id {
                emit_runtime_event(
                    state,
                    session_id,
                    "skill_snapshot_failed",
                    agent_name,
                    Some(agent_name),
                    serde_json::json!({
                        "agent": agent_name,
                        "trace_id": TRACE_ID,
                        "error": error.to_string(),
                    }),
                )
                .await;
            }
            return None;
        }
    };
    if let Some(session_id) = session_id {
        emit_skill_snapshot_event(
            state,
            session_id,
            agent_name,
            "skill_snapshot_ready",
            &snapshot,
            TRACE_ID,
        )
        .await;
        record_governed_skill_snapshot_activation(
            state, app_id, agent_name, session_id, &snapshot, TRACE_ID,
        )
        .await;
    }
    if let Some(session_id) = session_id {
        crate::framework_state_memento::save_session_namespace(
            state.sessions.framework_session_store.as_ref(),
            &app_id.0.to_string(),
            session_id,
            &snapshot_module,
            &snapshot,
        )
        .await;
        emit_skill_snapshot_event(
            state,
            session_id,
            agent_name,
            "skill_snapshot_cached",
            &snapshot,
            TRACE_ID,
        )
        .await;
    }
    Some(snapshot)
}
