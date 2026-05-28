use std::collections::BTreeMap;
use std::path::Path;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillOperatorCatalogListCommand, SkillOperatorCatalogListResult, SkillOperatorChangedCommand,
    SkillOperatorChangedResult, SkillOperatorConfigWriteCommand, SkillOperatorConfigWriteResult,
    SkillOperatorEnablementCommand, SkillOperatorEnablementResult,
    SkillOperatorMarkdownReadCommand, SkillOperatorMarkdownReadResult,
    SkillOperatorSourceDescriptor, SkillOperatorSourceKind, SkillOperatorWatchCommand,
    SkillOperatorWatchResult, SkillServicePolicyHints, SkillServiceScope,
    SKILL_OPERATOR_CATALOG_LIST_COMMAND, SKILL_OPERATOR_CHANGED_COMMAND,
    SKILL_OPERATOR_CONFIG_WRITE_COMMAND, SKILL_OPERATOR_ENABLEMENT_COMMAND,
    SKILL_OPERATOR_MARKDOWN_READ_COMMAND, SKILL_OPERATOR_WATCH_COMMAND,
};

use crate::SkillSystemServiceProvider;

fn traced_command<T: serde::Serialize>(
    name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).expect("test payload should serialize"),
        trace,
    )
}

async fn write_skill(root: &Path, dir: &str, name: &str, description: &str, body: &str) {
    let skill_dir = root.join(dir);
    tokio::fs::create_dir_all(&skill_dir)
        .await
        .expect("skill dir should be created");
    tokio::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n{body}"),
    )
    .await
    .expect("skill markdown should be written");
}

#[tokio::test]
async fn skill_operator_catalog_uses_source_precedence_and_reads_bounded_markdown() {
    let provider = SkillSystemServiceProvider::new();
    let temp = tempfile::tempdir().expect("tempdir should exist");
    let app_root = temp.path().join("app");
    let workspace_root = temp.path().join("workspace");
    let plugin_root = temp.path().join("plugin");
    write_skill(
        &plugin_root,
        "shared",
        "shared",
        "Plugin description",
        "plugin body",
    )
    .await;
    write_skill(
        &workspace_root,
        "shared",
        "shared",
        "Workspace description",
        "workspace body",
    )
    .await;
    write_skill(
        &app_root,
        "app-only",
        "app-only",
        "App description",
        "app body",
    )
    .await;

    let trace = TraceContext::new("trace-skill-operator-catalog");
    let result = provider
        .call(traced_command(
            SKILL_OPERATOR_CATALOG_LIST_COMMAND,
            SkillOperatorCatalogListCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                sources: vec![
                    SkillOperatorSourceDescriptor::new(
                        SkillOperatorSourceKind::PluginProvided,
                        plugin_root,
                    ),
                    SkillOperatorSourceDescriptor::new(
                        SkillOperatorSourceKind::Workspace,
                        workspace_root.clone(),
                    ),
                    SkillOperatorSourceDescriptor::new(
                        SkillOperatorSourceKind::Application,
                        app_root,
                    ),
                ],
                include_disabled: false,
                policy: SkillServicePolicyHints::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("catalog list should succeed");
    let catalog: SkillOperatorCatalogListResult =
        serde_json::from_value(result.output).expect("catalog result should decode");

    let shared = catalog
        .skills
        .iter()
        .find(|skill| skill.name == "shared")
        .expect("shared skill should be listed");
    assert_eq!(shared.description, "Workspace description");
    assert_eq!(shared.source_kind, SkillOperatorSourceKind::Workspace);
    assert_eq!(catalog.skipped_shadowed, 1);

    let markdown = provider
        .call(traced_command(
            SKILL_OPERATOR_MARKDOWN_READ_COMMAND,
            SkillOperatorMarkdownReadCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                skill_name: "shared".into(),
                sources: vec![SkillOperatorSourceDescriptor::new(
                    SkillOperatorSourceKind::Workspace,
                    workspace_root,
                )],
                max_bytes: 9,
                policy: SkillServicePolicyHints::default(),
            },
            trace,
        ))
        .await
        .expect("markdown read should succeed");
    let markdown: SkillOperatorMarkdownReadResult =
        serde_json::from_value(markdown.output).expect("markdown result should decode");
    assert_eq!(markdown.skill_name, "shared");
    assert_eq!(markdown.bytes_read, 9);
    assert!(markdown.truncated);
    assert!(markdown.markdown.contains("workspace"));
}

#[tokio::test]
async fn skill_operator_config_watch_enablement_are_audited_and_redacted() {
    let provider = SkillSystemServiceProvider::new();
    let temp = tempfile::tempdir().expect("tempdir should exist");
    let workspace_root = temp.path().join("workspace");
    write_skill(
        &workspace_root,
        "operator",
        "operator",
        "Operator description",
        "operator body",
    )
    .await;

    let trace = TraceContext::new("trace-skill-operator-state");
    let config = provider
        .call(traced_command(
            SKILL_OPERATOR_CONFIG_WRITE_COMMAND,
            SkillOperatorConfigWriteCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                skill_name: "operator".into(),
                values: BTreeMap::from([
                    ("mode".into(), "strict".into()),
                    ("api_token".into(), "raw-secret-token".into()),
                ]),
                policy: SkillServicePolicyHints {
                    entitlement_ready: Some(true),
                    package_ready: Some(true),
                    ..Default::default()
                },
            },
            trace.clone(),
        ))
        .await
        .expect("config write should succeed");
    let config: SkillOperatorConfigWriteResult =
        serde_json::from_value(config.output).expect("config result should decode");
    assert_eq!(config.redacted_values["api_token"], "[REDACTED]");
    assert_eq!(config.redacted_values["mode"], "strict");
    assert_eq!(config.audit_event_ids.len(), 1);

    let watch = provider
        .call(traced_command(
            SKILL_OPERATOR_WATCH_COMMAND,
            SkillOperatorWatchCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                source: SkillOperatorSourceDescriptor::new(
                    SkillOperatorSourceKind::Workspace,
                    workspace_root.clone(),
                ),
            },
            trace.clone(),
        ))
        .await
        .expect("watch should succeed");
    let watch: SkillOperatorWatchResult =
        serde_json::from_value(watch.output).expect("watch result should decode");
    assert_eq!(watch.watched_sources, 1);

    let changed = provider
        .call(traced_command(
            SKILL_OPERATOR_CHANGED_COMMAND,
            SkillOperatorChangedCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                skill_name: "operator".into(),
                source: SkillOperatorSourceDescriptor::new(
                    SkillOperatorSourceKind::Workspace,
                    workspace_root,
                ),
                change_kind: "modified".into(),
                summary: "operator markdown changed with raw-secret-token".into(),
            },
            trace.clone(),
        ))
        .await
        .expect("changed notification should succeed");
    let changed: SkillOperatorChangedResult =
        serde_json::from_value(changed.output).expect("changed result should decode");
    assert_eq!(changed.changed.skill_name, "operator");
    assert_eq!(changed.visibility_generation, 1);
    assert!(!changed
        .changed
        .sanitized_summary
        .contains("raw-secret-token"));

    let enablement = provider
        .call(traced_command(
            SKILL_OPERATOR_ENABLEMENT_COMMAND,
            SkillOperatorEnablementCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                skill_name: "operator".into(),
                enabled: false,
                reason: "temporary operator pause".into(),
                policy: SkillServicePolicyHints {
                    entitlement_ready: Some(true),
                    package_ready: Some(true),
                    ..Default::default()
                },
            },
            trace,
        ))
        .await
        .expect("enablement should succeed");
    let enablement: SkillOperatorEnablementResult =
        serde_json::from_value(enablement.output).expect("enablement result should decode");
    assert!(!enablement.enabled);
    assert_eq!(enablement.visibility_generation, 2);
    assert_eq!(enablement.audit_event_ids.len(), 1);
}
