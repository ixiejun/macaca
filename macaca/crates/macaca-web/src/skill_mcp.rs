//! Skill-backed MCP runtime integration.
//!
//! Standard AgentSkills provide instructions. Some skills also describe MCP
//! servers that provide the executable tools. This module bridges eligible,
//! visible skill snapshots into framework toolkit tools.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use axum::response::sse::Event;
use macaca_app::AppLoader;
use macaca_framework::tool::Toolkit;
use macaca_proto::ApplicationId;
use macaca_skill::{
    SkillMcpServerConfig, SkillPolicy, SkillRuntime, SkillRuntimeOptions, SkillSnapshot,
    SkillSnapshotEntry,
};
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Clone)]
struct SkillMcpServerLaunch {
    skill_name: String,
    server_id: String,
    command: String,
    args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillMcpStatus {
    pub skill: String,
    pub server_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub state: SkillMcpStatusState,
    pub exposed_tools: Vec<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillMcpStatusState {
    Ready,
    Failed,
    DependencyMissing,
}

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
    let definitions = crate::mcp_runtime::definitions_from_skill_snapshot(&snapshot);
    let context = crate::mcp_runtime::McpRuntimeContext::for_agent(app_id, session_id, agent_name);
    let _ = state
        .mcp_runtime
        .register_definitions(
            toolkit,
            definitions,
            &crate::mcp_runtime::McpToolPolicy::default(),
            &context,
            None,
        )
        .await;
}

pub(crate) async fn probe_skill_mcp_servers(snapshot: &SkillSnapshot) -> Vec<SkillMcpStatus> {
    let definitions = crate::mcp_runtime::definitions_from_skill_snapshot(snapshot);
    let statuses = crate::mcp_runtime::probe_definition_statuses(
        definitions,
        &crate::mcp_runtime::McpToolPolicy::default(),
    )
    .await;
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
                    crate::mcp_runtime::McpRuntimeStatusState::Ready => SkillMcpStatusState::Ready,
                    crate::mcp_runtime::McpRuntimeStatusState::DependencyMissing => {
                        SkillMcpStatusState::DependencyMissing
                    }
                    crate::mcp_runtime::McpRuntimeStatusState::Failed
                    | crate::mcp_runtime::McpRuntimeStatusState::Disabled => {
                        SkillMcpStatusState::Failed
                    }
                },
                exposed_tools: status.exposed_tools,
                failure_reason: status.failure_reason,
            }
        })
        .collect()
}

pub(crate) async fn load_or_build_skill_snapshot(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<&str>,
) -> Option<SkillSnapshot> {
    let snapshot_module = format!("skill_snapshot/{agent_name}");
    if let Some(session_id) = session_id {
        if let Ok(Some(value)) = state
            .sessions
            .framework_session_store
            .load(session_id, &snapshot_module)
            .await
        {
            if let Ok(snapshot) = serde_json::from_value::<SkillSnapshot>(value) {
                return Some(snapshot);
            }
        }
    }

    let app = {
        let registry = state.registry.read().await;
        registry.get_app(app_id).cloned()
    }?;
    let workspace_dir = {
        let workspaces = state.config.app_workspaces.read().await;
        workspaces.get(app_id).map(|ws| ws.root.clone())
    };
    let policy = resolve_agent_skill_policy(state, app_id, agent_name).await;
    let snapshot = SkillRuntime
        .build_snapshot(
            agent_name,
            SkillRuntimeOptions {
                workspace_dir,
                app_dir: Some(app.path),
                policy,
                ..Default::default()
            },
        )
        .await
        .ok()?;
    if let Some(session_id) = session_id {
        if let Ok(value) = serde_json::to_value(&snapshot) {
            let _ = state
                .sessions
                .framework_session_store
                .save(session_id, &snapshot_module, value)
                .await;
        }
    }
    Some(snapshot)
}

async fn resolve_agent_skill_policy(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
) -> SkillPolicy {
    let registry = state.registry.read().await;
    let Some(app) = registry.get_app(app_id) else {
        return SkillPolicy::default();
    };
    let Ok(agent_configs) = AppLoader::resolve_agent_configs(&app.manifest, &app.path) else {
        return SkillPolicy::default();
    };
    agent_configs
        .into_iter()
        .find(|agent| agent.name == agent_name)
        .and_then(|agent| agent.skills)
        .map(|skills| SkillPolicy {
            allow: skills.allow,
            deny: skills.deny,
        })
        .unwrap_or_default()
}

fn resolve_skill_mcp_servers(snapshot: &SkillSnapshot) -> Vec<SkillMcpServerLaunch> {
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
        if let Some(launch) = launch_from_compat_registry(skill) {
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

fn launch_from_compat_registry(skill: &SkillSnapshotEntry) -> Option<SkillMcpServerLaunch> {
    let has_playwright_package = skill.install.iter().any(|install| {
        install.package.as_deref() == Some("@playwright/mcp")
            || install.bins.iter().any(|bin| bin == "playwright-mcp")
    });
    if has_playwright_package {
        return Some(SkillMcpServerLaunch {
            skill_name: skill.name.clone(),
            server_id: "playwright".to_string(),
            command: "playwright-mcp".to_string(),
            args: vec!["--headless".to_string(), "--isolated".to_string()],
        });
    }
    None
}

fn launch_from_runtime_server_id(
    snapshot: &SkillSnapshot,
    server_id: &str,
) -> Option<SkillMcpServerLaunch> {
    resolve_skill_mcp_servers(snapshot)
        .into_iter()
        .find(|launch| {
            server_id == format!("skill:{}:{}", launch.skill_name, launch.server_id)
                || server_id == format!("skill:{}:playwright", launch.skill_name)
        })
}

#[cfg(test)]
fn command_exists(command: &str) -> bool {
    if command.contains(std::path::MAIN_SEPARATOR) {
        return PathBuf::from(command).is_file();
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(command).is_file())
}

async fn emit_skill_mcp_event(
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
    state
        .persist
        .event_log
        .append(session_id, event_type, agent_name, payload.clone())
        .await;

    let sse_tx = {
        let active_sessions = state.sessions.active_sessions.read().await;
        active_sessions
            .get(session_id)
            .map(|session| Arc::clone(&session.sse_tx))
    };
    if let Some(sse_tx) = sse_tx {
        let sender = sse_tx.read().await;
        let _ = sender
            .send(Ok(Event::default()
                .event(event_type)
                .data(payload.to_string())))
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_skill::{SkillInstallSpec, SkillMcpServerConfig, SkillSnapshot};

    fn snapshot_with_entry(entry: SkillSnapshotEntry) -> SkillSnapshot {
        SkillSnapshot {
            agent: "agent".into(),
            prompt: String::new(),
            skills: vec![entry],
            filtered: Vec::new(),
            truncated: false,
            compact: false,
            version: 1,
        }
    }

    fn playwright_entry() -> SkillSnapshotEntry {
        SkillSnapshotEntry {
            name: "playwright-mcp".into(),
            description: "Browser".into(),
            location: PathBuf::from("/tmp/SKILL.md"),
            base_dir: PathBuf::from("/tmp"),
            source: "test".into(),
            source_scope: macaca_skill::SkillSourceScope::MacacaCentral,
            primary_env: None,
            required_env: Vec::new(),
            install: vec![SkillInstallSpec {
                kind: "npm".into(),
                package: Some("@playwright/mcp".into()),
                bins: vec!["playwright-mcp".into()],
                ..Default::default()
            }],
            mcp_servers: Vec::new(),
        }
    }

    #[test]
    fn compat_registry_resolves_playwright_mcp_package() {
        let snapshot = snapshot_with_entry(playwright_entry());
        let launches = resolve_skill_mcp_servers(&snapshot);
        assert_eq!(launches.len(), 1);
        assert_eq!(launches[0].command, "playwright-mcp");
        assert_eq!(launches[0].args, vec!["--headless", "--isolated"]);
    }

    #[tokio::test]
    async fn probe_reports_dependency_missing_without_registering_tools() {
        let mut entry = playwright_entry();
        entry.mcp_servers = vec![SkillMcpServerConfig {
            id: "missing".into(),
            command: "definitely-missing-macaca-mcp-command".into(),
            args: Vec::new(),
            transport: "stdio".into(),
            tool_prefix: None,
        }];
        entry.install = Vec::new();

        let statuses = probe_skill_mcp_servers(&snapshot_with_entry(entry)).await;
        assert_eq!(statuses.len(), 1);
        assert!(matches!(
            statuses[0].state,
            SkillMcpStatusState::DependencyMissing
        ));
        assert!(statuses[0].exposed_tools.is_empty());
        assert_eq!(
            statuses[0].failure_reason.as_deref(),
            Some("missing dependency: definitely-missing-macaca-mcp-command")
        );
    }

    #[tokio::test]
    async fn probe_playwright_mcp_lists_browser_tools_when_installed() {
        if !command_exists("playwright-mcp") {
            eprintln!("playwright-mcp not installed; skipping local integration probe");
            return;
        }

        let statuses = probe_skill_mcp_servers(&snapshot_with_entry(playwright_entry())).await;
        assert_eq!(statuses.len(), 1);
        assert!(matches!(statuses[0].state, SkillMcpStatusState::Ready));
        assert!(statuses[0]
            .exposed_tools
            .iter()
            .any(|tool| tool == "browser_navigate"));
    }
}
