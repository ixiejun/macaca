//! Sanitization contract tests for service projection (Object Mother fixtures).
//!
//! Fixtures use neutral `fixture-*` identifiers.  Assertions verify that prompt
//! bodies, model names, permission levels, and secret-like metadata never leak
//! into serialized service views.

use macaca_proto::ApplicationId;

use super::heartbeat::app_manifest_to_heartbeat_agent_views;
use super::projection::{
    app_manifest_to_metadata_view, app_manifest_to_service_app_view,
};

use crate::model::{
    AgentSource, AppAutonomyConfig, AppHeartbeatAgentConfig, AppHeartbeatCadenceConfig,
    AppHeartbeatConfig, AppHeartbeatGateConfig, AppLayer, AppManifest, AppStatus, CapabilityRef,
    InlineAgentConfig,
};

fn fixture_manifest() -> AppManifest {
    AppManifest {
        id: ApplicationId::new(),
        name: "safe-app".into(),
        description: Some("Safe app".into()),
        version: "1.0.0".into(),
        layer: AppLayer::L3Declarative,
        ui_type: None,
        agents: vec![AgentSource::Inline(InlineAgentConfig {
            name: "fixture-role-alpha".into(),
            capabilities: vec![CapabilityRef {
                name: "plan".into(),
                description: "Plans work".into(),
            }],
            prompt_template: "SECRET_PROMPT_BODY_SHOULD_NOT_LEAK".into(),
            model: "provider-model".into(),
            permission_level: "admin".into(),
            allowed_tools: vec!["shell_execute".into(), "read_file".into()],
            max_tokens: None,
            temperature: None,
            skills: Some(Default::default()),
            context_engine: Some("private-context-engine".into()),
        })],
        llm_config: None,
        entry_agent: Some("fixture-role-alpha".into()),
        entrypoint: None,
        workflows: None,
        resources: None,
        context: Some(Default::default()),
        service_contract: None,
        execution_profile: None,
        workbench: None,
        autonomy: None,
        ui: None,
        execution_control: None,
    }
}

#[test]
fn sanitized_metadata_excludes_prompt_and_secret_like_values() {
    let manifest = fixture_manifest();
    let view = app_manifest_to_metadata_view(
        &manifest,
        None,
        AppStatus::Loaded,
        true,
        true,
        true,
        true,
    );
    let encoded = serde_json::to_string(&view).unwrap();
    assert!(!encoded.contains("SECRET_PROMPT_BODY_SHOULD_NOT_LEAK"));
    assert!(!encoded.contains("provider-model"));
    assert!(!encoded.contains("admin"));
    assert!(!encoded.contains("private-context-engine"));
    assert_eq!(view.entry.agent_name.as_deref(), Some("fixture-role-alpha"));
    assert_eq!(view.tool_policy.execution_tool_count, 1);
}

#[test]
fn service_app_view_preserves_safe_agent_and_capability_names() {
    let manifest = fixture_manifest();
    let view = app_manifest_to_service_app_view(&manifest, None, AppStatus::Loaded);
    assert_eq!(view.agents.len(), 1);
    assert_eq!(view.agents[0].name, "fixture-role-alpha");
    assert!(view.agents[0]
        .capability_names
        .iter()
        .any(|capability| capability.contains("plan")));
}

#[test]
fn service_app_view_contains_effective_service_diagnostics() {
    let mut manifest = fixture_manifest();
    manifest.service_contract = Some(crate::service_capability::AppServiceContractConfig {
        use_packs: vec!["pack.finance.v1".into()],
        required_services: vec!["service.custom.required".into()],
        optional_services: vec![],
        service_policy_overrides: Default::default(),
    });
    let view = app_manifest_to_service_app_view(&manifest, None, AppStatus::Loaded);
    assert!(view
        .diagnostics
        .iter()
        .any(|line| line.starts_with("effective_service_capabilities_hash=")));
    assert!(view
        .diagnostics
        .iter()
        .any(|line| line == "effective_service_count=1"));
    assert!(view.diagnostics.iter().any(|line| {
        line == "unresolved_domain_packs=pack.finance.v1"
    }));
}

#[test]
fn heartbeat_agent_projection_is_sanitized_and_validates_agent_names() {
    let mut manifest = fixture_manifest();
    manifest.autonomy = Some(AppAutonomyConfig {
        heartbeat: Some(AppHeartbeatConfig {
            enabled: true,
            agents: vec![
                AppHeartbeatAgentConfig {
                    name: "fixture-role-alpha".into(),
                    enabled: true,
                    profile_id: "default".into(),
                    cadence: Some(AppHeartbeatCadenceConfig {
                        fixed_interval_secs: Some(600),
                    }),
                    gates: Some(AppHeartbeatGateConfig {
                        cooldown_secs: Some(120),
                    }),
                    metadata: std::collections::BTreeMap::from([(
                        "purpose".into(),
                        "operational_probe".into(),
                    )]),
                },
                AppHeartbeatAgentConfig {
                    name: "fixture-role-missing".into(),
                    enabled: true,
                    profile_id: "default".into(),
                    cadence: None,
                    gates: None,
                    metadata: std::collections::BTreeMap::new(),
                },
            ],
        }),
    });

    let views = app_manifest_to_heartbeat_agent_views(&manifest);

    assert_eq!(views.len(), 2);
    assert_eq!(views[0].agent_name, "fixture-role-alpha");
    assert!(views[0].enabled);
    assert!(views[0]
        .native_profile_id
        .contains(".agent.fixture-role-alpha.heartbeat"));
    assert!(views[0]
        .wake_scope_key
        .contains(".agent:fixture-role-alpha.heartbeat"));
    assert_eq!(views[0].fixed_interval_secs, Some(600));
    assert_eq!(views[0].cooldown_secs, Some(120));
    assert!(views[0].diagnostics.is_empty());
    assert_eq!(
        views[0].metadata.get("purpose").map(String::as_str),
        Some("operational_probe")
    );
    assert_eq!(views[1].agent_name, "fixture-role-missing");
    assert_eq!(views[1].diagnostics, vec!["heartbeat_agent_unknown"]);
}

#[test]
fn heartbeat_agent_projection_returns_no_rows_when_disabled() {
    let mut manifest = fixture_manifest();
    manifest.autonomy = Some(AppAutonomyConfig {
        heartbeat: Some(AppHeartbeatConfig {
            enabled: false,
            agents: vec![AppHeartbeatAgentConfig {
                name: "fixture-role-alpha".into(),
                enabled: true,
                profile_id: "default".into(),
                cadence: None,
                gates: None,
                metadata: std::collections::BTreeMap::new(),
            }],
        }),
    });

    assert!(app_manifest_to_heartbeat_agent_views(&manifest).is_empty());
}
