//! Contract tests for application manifest serde and capability composition.
//!
//! Fixtures use neutral `fixture-*` identifiers so migration-debt inventory
//! scans do not flag forbidden production role tokens.

use macaca_proto::ApplicationId;

use super::agent_config::{AgentSource, CapabilityRef, InlineAgentConfig};
use super::capability::{AppCapabilitySet, AppCapabilitySource};
use super::core::{AppLayer, AppStatus};
use super::loaded::LoadedApp;
use super::manifest::AppManifest;
use super::workflow::EntrypointType;

#[test]
fn app_layer_serialization_roundtrip() {
    let layer = AppLayer::L3Declarative;
    let json = serde_json::to_string(&layer).unwrap();
    let parsed: AppLayer = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, AppLayer::L3Declarative);
}

#[test]
fn app_status_values() {
    assert_ne!(AppStatus::Loaded, AppStatus::Running);
    assert_ne!(AppStatus::Running, AppStatus::Stopped);
    assert_ne!(AppStatus::Stopped, AppStatus::Failed);
}

#[test]
fn app_manifest_from_yaml() {
    let yaml = r#"
name: my-app
version: "1.0.0"
layer: L3Declarative
agents:
  - name: helper
    prompt_template: "You help."
    capabilities:
      - name: assist
"#;
    let manifest: AppManifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(manifest.name, "my-app");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.layer, AppLayer::L3Declarative);
    assert_eq!(manifest.agents.len(), 1);
}

#[test]
fn app_manifest_file_path_agents() {
    let yaml = r#"
name: path-app
layer: L3Declarative
agents:
  - "agents/fixture-alpha.yaml"
  - "agents/fixture-beta.yaml"
"#;
    let manifest: AppManifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(manifest.agents.len(), 2);
    match &manifest.agents[0] {
        AgentSource::FilePath(p) => assert_eq!(p, "agents/fixture-alpha.yaml"),
        _ => panic!("Expected FilePath"),
    }
}

#[test]
fn loaded_app_defaults() {
    let app = LoadedApp {
        manifest: AppManifest {
            id: ApplicationId::new(),
            name: "test".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L1Native,
            ui_type: None,
            agents: vec![],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: None,
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        },
        agent_ids: vec![],
        status: AppStatus::Loaded,
    };
    assert_eq!(app.status, AppStatus::Loaded);
    assert!(app.agent_ids.is_empty());
}

#[test]
fn entrypoint_config_parsing() {
    let yaml = r#"
name: test-app
layer: L3Declarative
entrypoint:
  type: workflow
  name: fixture-workflow
workflows:
  fixture-workflow:
    description: Fixture workflow graph
    steps:
      - name: step-alpha
        agent: fixture-role-alpha
        prompt_template: "Analyze: {{input}}"
      - name: step-beta
        agent: fixture-role-beta
        depends_on: [step-alpha]
"#;
    let manifest: AppManifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(manifest.name, "test-app");

    let entrypoint = manifest.entrypoint.unwrap();
    assert_eq!(entrypoint.type_, EntrypointType::Workflow);
    assert_eq!(entrypoint.name, "fixture-workflow");

    let workflows = manifest.workflows.unwrap();
    let workflow = workflows.get("fixture-workflow").unwrap();
    assert_eq!(workflow.steps.len(), 2);
    assert_eq!(workflow.steps[1].depends_on, vec!["step-alpha"]);
}

/// Documents the YAML tag encoding required by `serde_yaml` for externally
/// tagged `ExecutionControlTrigger` / `ExecutionControlResumeSource` enums.
/// Application manifests MUST use `!Variant` tags so `AppLoader` can parse
/// `execution_control` blocks without map/tag ambiguity.
#[test]
fn execution_control_policy_parses_yaml_variant_tags() {
    use macaca_proto::{
        ExecutionControlCheckpointMode, ExecutionControlMode, ExecutionControlResumeSource,
        ExecutionControlTrigger,
    };

    let yaml = r#"
name: execution-control-fixture
layer: L3Declarative
execution_control:
  mode: Enabled
  triggers:
    - !ToolCallBarrier
      tool_name: create_goal
  resume_sources:
    - !GoalLifecycle
  checkpoint_mode: ReferenceOnly
  allow_command_overrides: false
"#;
    let manifest: AppManifest = serde_yaml::from_str(yaml).unwrap();
    let policy = manifest.execution_control.expect("execution_control block");
    assert_eq!(policy.mode, ExecutionControlMode::Enabled);
    assert_eq!(
        policy.triggers,
        vec![ExecutionControlTrigger::tool_call_barrier("create_goal")]
    );
    assert_eq!(
        policy.resume_sources,
        vec![ExecutionControlResumeSource::goal_lifecycle()]
    );
    assert_eq!(
        policy.checkpoint_mode,
        ExecutionControlCheckpointMode::ReferenceOnly
    );
    assert!(!policy.allow_command_overrides);
}

#[test]
fn resource_config_parsing() {
    let yaml = r#"
name: test-app
layer: L3Declarative
resources:
  personas: personas/
  skills: skills/
  prompts: prompts/
  workflows: workflows/
"#;
    let manifest: AppManifest = serde_yaml::from_str(yaml).unwrap();
    let resources = manifest.resources.unwrap();
    assert_eq!(resources.personas, Some("personas/".into()));
    assert_eq!(resources.skills, Some("skills/".into()));
}

#[test]
fn app_capability_set_flattens_manifest_capabilities() {
    let agents = vec![AgentSource::Inline(InlineAgentConfig {
        name: "fixture-alpha".into(),
        capabilities: vec![
            CapabilityRef {
                name: "capability-design".into(),
                description: "design cap".into(),
            },
            CapabilityRef {
                name: "capability-build".into(),
                description: "build cap".into(),
            },
        ],
        prompt_template: String::new(),
        model: String::new(),
        permission_level: "user".into(),
        allowed_tools: vec![],
        max_tokens: None,
        temperature: None,
        skills: None,
        context_engine: None,
    })];

    let set = AppCapabilitySet::from_manifest_agents(&agents);
    let flattened = set.flatten();
    assert_eq!(flattened.len(), 2);
    assert_eq!(flattened[0].name, "capability-design");
    assert_eq!(flattened[1].name, "capability-build");
}

#[test]
fn app_capability_set_preserves_visible_capability_count_across_sources() {
    let agents = vec![AgentSource::Inline(InlineAgentConfig {
        name: "fixture-alpha".into(),
        capabilities: vec![CapabilityRef {
            name: "capability-design".into(),
            description: "design cap".into(),
        }],
        prompt_template: String::new(),
        model: String::new(),
        permission_level: "user".into(),
        allowed_tools: vec![],
        max_tokens: None,
        temperature: None,
        skills: None,
        context_engine: None,
    })];

    let mut set = AppCapabilitySet::from_manifest_agents(&agents);
    set.push_group(
        AppCapabilitySource::Driver,
        vec![CapabilityRef {
            name: "capability.driver.fixture".into(),
            description: "driver capability".into(),
        }],
    );
    set.push_group(
        AppCapabilitySource::Skill,
        vec![CapabilityRef {
            name: "capability.skill.fixture".into(),
            description: "skill capability".into(),
        }],
    );

    assert_eq!(set.roots().len(), 3);
    let flattened = set.flatten();
    assert_eq!(flattened.len(), 3);
    assert_eq!(
        flattened
            .iter()
            .map(|cap| cap.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "capability-design",
            "capability.driver.fixture",
            "capability.skill.fixture"
        ]
    );
}
