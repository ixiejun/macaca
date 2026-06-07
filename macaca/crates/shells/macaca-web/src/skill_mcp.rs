//! Skill-backed MCP runtime integration.
//!
//! Standard AgentSkills provide instructions. Some skills also describe MCP
//! servers that provide the executable tools. This module bridges eligible,
//! visible skill snapshots into framework toolkit tools.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use macaca_app::AppLoader;
use macaca_framework::tool::Toolkit;
use macaca_proto::{ApplicationId, TraceContext};
use macaca_runtime_host::skill_mcp_mapping_registry::default_skill_mcp_mapping_registry;
use macaca_runtime_host::{
    apply_concurrency_isolation, probe_definition_statuses, McpRuntimeContext,
    McpRuntimeStatusState, McpToolPolicy,
};
use macaca_skill::{
    SkillGovernanceRecord, SkillGovernanceRecordUsageCommand, SkillGovernanceSnapshotCommand,
    SkillLifecycleState, SkillMcpServerConfig, SkillPolicy, SkillRuntimeFacade, SkillServiceScope,
    SkillSnapshot, SkillSnapshotEntry, SkillSnapshotRequest, SkillSnapshotServiceCommand,
    SkillUsageEventKind, SkillUsageObservation,
};
use serde::Serialize;

use crate::runtime_event_bridge::{emit_runtime_event, emit_skill_snapshot_event};
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
    let definitions = macaca_runtime_host::McpServerFactory::with_bundled_mapping_registry()
        .from_skill_snapshot(&snapshot);
    let context = McpRuntimeContext::for_agent(app_id, session_id, agent_name);
    let _ = state
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

pub(crate) async fn probe_skill_mcp_servers(snapshot: &SkillSnapshot) -> Vec<SkillMcpStatus> {
    let definitions = macaca_runtime_host::McpServerFactory::with_bundled_mapping_registry()
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

pub(crate) async fn load_or_build_skill_snapshot(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<&str>,
) -> Option<SkillSnapshot> {
    const TRACE_ID: &str = "web-skill-mcp-snapshot";
    let snapshot_module = format!("skill_snapshot/{agent_name}");
    if let Some(session_id) = session_id {
        if let Ok(Some(value)) = state
            .sessions
            .framework_session_store
            .load(session_id, &snapshot_module)
            .await
        {
            if let Ok(snapshot) = serde_json::from_value::<SkillSnapshot>(value) {
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
    let app_dir = app.path.clone();
    let request = SkillSnapshotRequest::builder(agent_name)
        .workspace_dir(workspace_dir.clone())
        .app_dir(Some(app_dir.clone()))
        .policy(policy.clone())
        .build();
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
                "Skill Service snapshot failed; using deprecated SkillRuntimeFacade fallback"
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
            #[allow(deprecated)]
            SkillRuntimeFacade::new()
                .build_snapshot(request)
                .await
                .ok()?
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
        if let Ok(value) = serde_json::to_value(&snapshot) {
            let _ = state
                .sessions
                .framework_session_store
                .save(session_id, &snapshot_module, value)
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
    }
    Some(snapshot)
}

async fn record_governed_skill_snapshot_activation(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: &str,
    snapshot: &SkillSnapshot,
    trace_id: &str,
) {
    let scope = match SkillServiceScope::agent(*app_id, session_id, agent_name) {
        Ok(scope) => scope,
        Err(error) => {
            tracing::warn!(
                error = %error,
                app_id = %app_id,
                agent = %agent_name,
                session_id,
                "skipping governed Skill activation telemetry because scope is invalid"
            );
            return;
        }
    };
    let governance = match state
        .skill_client
        .governance_snapshot(SkillGovernanceSnapshotCommand {
            trace: TraceContext::new(trace_id),
            scope: scope.clone(),
            include_archived: false,
            lifecycle_filters: vec![SkillLifecycleState::Active],
        })
        .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tracing::warn!(
                error = %error,
                app_id = %app_id,
                agent = %agent_name,
                session_id,
                "governed Skill activation telemetry snapshot lookup failed"
            );
            return;
        }
    };

    for command in build_governed_skill_activation_usage_commands(
        snapshot,
        &governance.records,
        *app_id,
        session_id,
        agent_name,
        trace_id,
    ) {
        let skill_id = command.observation.skill_id.clone();
        if let Err(error) = state.skill_client.record_governance_usage(command).await {
            tracing::warn!(
                error = %error,
                app_id = %app_id,
                agent = %agent_name,
                session_id,
                skill_id,
                "governed Skill activation telemetry recording failed"
            );
        }
    }
}

fn build_governed_skill_activation_usage_commands(
    snapshot: &SkillSnapshot,
    governance_records: &[SkillGovernanceRecord],
    app_id: ApplicationId,
    session_id: &str,
    agent_name: &str,
    trace_id: &str,
) -> Vec<SkillGovernanceRecordUsageCommand> {
    let visible_skill_names = snapshot
        .skills
        .iter()
        .map(|skill| skill.name.as_str())
        .collect::<HashSet<_>>();
    let Ok(scope) = SkillServiceScope::agent(app_id, session_id, agent_name) else {
        return Vec::new();
    };

    governance_records
        .iter()
        .filter(|record| record.lifecycle == SkillLifecycleState::Active)
        .filter(|record| visible_skill_names.contains(record.provenance.name.as_str()))
        .map(|record| {
            let mut metadata = BTreeMap::new();
            metadata.insert("activation_surface".into(), "agent_skill_snapshot".into());
            metadata.insert("agent_name".into(), agent_name.into());
            metadata.insert("session_id".into(), session_id.into());
            metadata.insert("snapshot_version".into(), snapshot.version.to_string());
            SkillGovernanceRecordUsageCommand {
                trace: TraceContext::new(trace_id),
                scope: scope.clone(),
                observation: SkillUsageObservation {
                    skill_id: record.provenance.skill_id.clone(),
                    name: record.provenance.name.clone(),
                    source: record.provenance.source.clone(),
                    source_scope: record.provenance.source_scope.clone(),
                    event: SkillUsageEventKind::Activated,
                    author_kind: record.provenance.author_kind.clone(),
                    created_by: record.provenance.created_by.clone(),
                    pinned: Some(record.pinned),
                    evidence_id: Some(format!(
                        "eventlog://sessions/{session_id}/skill_snapshot/{agent_name}"
                    )),
                    metadata,
                },
            }
        })
        .collect()
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

fn launch_from_runtime_server_id(
    snapshot: &SkillSnapshot,
    server_id: &str,
) -> Option<SkillMcpServerLaunch> {
    resolve_skill_mcp_servers(snapshot)
        .into_iter()
        .find(|launch| server_id == format!("skill:{}:{}", launch.skill_name, launch.server_id))
}

#[cfg(test)]
fn command_exists(command: &str) -> bool {
    if command.contains(std::path::MAIN_SEPARATOR) {
        return std::path::PathBuf::from(command).is_file();
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
            source_location: std::path::PathBuf::from("/tmp/SKILL.md"),
            source_base_dir: std::path::PathBuf::from("/tmp"),
            location: std::path::PathBuf::from("/tmp/SKILL.md"),
            base_dir: std::path::PathBuf::from("/tmp"),
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
    fn activation_usage_commands_only_cover_active_governed_snapshot_skills() {
        let app_id = ApplicationId(uuid::Uuid::new_v4());
        let active_name = "skill-exp-active";
        let snapshot = SkillSnapshot {
            agent: "coordinator".into(),
            prompt: String::new(),
            skills: vec![
                SkillSnapshotEntry {
                    name: active_name.into(),
                    description: "active governed skill".into(),
                    source_location: std::path::PathBuf::from("/tmp/active/SKILL.md"),
                    source_base_dir: std::path::PathBuf::from("/tmp/active"),
                    location: std::path::PathBuf::from("/tmp/available/active/SKILL.md"),
                    base_dir: std::path::PathBuf::from("/tmp/available/active"),
                    source: "application".into(),
                    source_scope: macaca_skill::SkillSourceScope::Application,
                    primary_env: None,
                    required_env: Vec::new(),
                    install: Vec::new(),
                    mcp_servers: Vec::new(),
                },
                SkillSnapshotEntry {
                    name: "ungoverned".into(),
                    description: "plain app skill".into(),
                    source_location: std::path::PathBuf::from("/tmp/plain/SKILL.md"),
                    source_base_dir: std::path::PathBuf::from("/tmp/plain"),
                    location: std::path::PathBuf::from("/tmp/available/plain/SKILL.md"),
                    base_dir: std::path::PathBuf::from("/tmp/available/plain"),
                    source: "application".into(),
                    source_scope: macaca_skill::SkillSourceScope::Application,
                    primary_env: None,
                    required_env: Vec::new(),
                    install: Vec::new(),
                    mcp_servers: Vec::new(),
                },
            ],
            filtered: Vec::new(),
            truncated: false,
            compact: false,
            version: 1,
        };
        let active_record = macaca_skill::SkillGovernanceRecord {
            provenance: macaca_skill::SkillGovernanceProvenance::new(
                "skill://agent/skill-exp-active",
                active_name,
                "skill.evolution",
                "proposal",
                macaca_skill::SkillAuthorKind::Agent,
            ),
            lifecycle: macaca_skill::SkillLifecycleState::Active,
            pinned: false,
            telemetry: Default::default(),
            diagnostics: Default::default(),
            updated_at: chrono::Utc::now(),
            evidence_ids: vec!["eventlog://run-42".into()],
        };
        let archived_record = macaca_skill::SkillGovernanceRecord {
            provenance: macaca_skill::SkillGovernanceProvenance::new(
                "skill://agent/archived",
                "archived",
                "skill.evolution",
                "proposal",
                macaca_skill::SkillAuthorKind::Agent,
            ),
            lifecycle: macaca_skill::SkillLifecycleState::Archived,
            pinned: false,
            telemetry: Default::default(),
            diagnostics: Default::default(),
            updated_at: chrono::Utc::now(),
            evidence_ids: Vec::new(),
        };

        let commands = build_governed_skill_activation_usage_commands(
            &snapshot,
            &[active_record, archived_record],
            app_id,
            "session-1",
            "coordinator",
            "trace-skill-visible",
        );

        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0].observation.skill_id,
            "skill://agent/skill-exp-active"
        );
        assert_eq!(
            commands[0].observation.event,
            macaca_skill::SkillUsageEventKind::Activated
        );
        assert_eq!(
            commands[0]
                .observation
                .metadata
                .get("activation_surface")
                .map(String::as_str),
            Some("agent_skill_snapshot")
        );
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
