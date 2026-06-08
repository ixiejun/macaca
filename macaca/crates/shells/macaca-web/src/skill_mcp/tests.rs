//! Unit and optional integration probes for skill-backed MCP bridging.

use macaca_proto::ApplicationId;
use macaca_sdk::skill::{
    SkillInstallSpec, SkillMcpServerConfig, SkillSnapshot, SkillSnapshotEntry,
};

use super::governance_telemetry::build_governed_skill_activation_usage_commands;
use super::probe::probe_skill_mcp_servers;
use super::server_resolution::resolve_skill_mcp_servers;
use super::types::SkillMcpStatusState;

/// Build a minimal snapshot containing a single skill entry for probe tests.
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

/// Fixture skill that resolves to the bundled playwright MCP mapping registry entry.
fn playwright_entry() -> SkillSnapshotEntry {
    SkillSnapshotEntry {
        name: "playwright-mcp".into(),
        description: "Browser".into(),
        source_location: std::path::PathBuf::from("/tmp/SKILL.md"),
        source_base_dir: std::path::PathBuf::from("/tmp"),
        location: std::path::PathBuf::from("/tmp/SKILL.md"),
        base_dir: std::path::PathBuf::from("/tmp"),
        source: "test".into(),
        source_scope: macaca_sdk::skill::SkillSourceScope::MacacaCentral,
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

/// Returns true when `command` is executable on the current PATH (integration probes only).
fn command_exists(command: &str) -> bool {
    if command.contains(std::path::MAIN_SEPARATOR) {
        return std::path::PathBuf::from(command).is_file();
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(command).is_file())
}

/// Provider-neutral fixture agent id for skill MCP activation tests.
const FIXTURE_SNAPSHOT_AGENT: &str = "fixture-snapshot-agent";

#[test]
fn activation_usage_commands_only_cover_active_governed_snapshot_skills() {
    let app_id = ApplicationId(uuid::Uuid::new_v4());
    let active_name = "skill-exp-active";
    let snapshot = SkillSnapshot {
        agent: FIXTURE_SNAPSHOT_AGENT.into(),
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
                source_scope: macaca_sdk::skill::SkillSourceScope::Application,
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
                source_scope: macaca_sdk::skill::SkillSourceScope::Application,
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
    let active_record = macaca_sdk::skill::SkillGovernanceRecord {
        provenance: macaca_sdk::skill::SkillGovernanceProvenance::new(
            "skill://agent/skill-exp-active",
            active_name,
            "skill.evolution",
            "proposal",
            macaca_sdk::skill::SkillAuthorKind::Agent,
        ),
        lifecycle: macaca_sdk::skill::SkillLifecycleState::Active,
        pinned: false,
        telemetry: Default::default(),
        diagnostics: Default::default(),
        updated_at: chrono::Utc::now(),
        evidence_ids: vec!["eventlog://run-42".into()],
    };
    let archived_record = macaca_sdk::skill::SkillGovernanceRecord {
        provenance: macaca_sdk::skill::SkillGovernanceProvenance::new(
            "skill://agent/archived",
            "archived",
            "skill.evolution",
            "proposal",
            macaca_sdk::skill::SkillAuthorKind::Agent,
        ),
        lifecycle: macaca_sdk::skill::SkillLifecycleState::Archived,
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
        FIXTURE_SNAPSHOT_AGENT,
        "trace-skill-visible",
    );

    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].observation.skill_id,
        "skill://agent/skill-exp-active"
    );
    assert_eq!(
        commands[0].observation.event,
        macaca_sdk::skill::SkillUsageEventKind::Activated
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
