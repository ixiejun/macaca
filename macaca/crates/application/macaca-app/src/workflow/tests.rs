//! Contract tests for workflow prompt assembly (Object Mother fixtures).
//!
//! Fixtures use neutral `fixture-*` identifiers and discover example apps via
//! repository-relative paths without teaching production code application names.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::loader::AppLoader;
use crate::model::{
    AgentSource, AppLayer, AppManifest, InlineAgentConfig, WorkflowDefinition, WorkflowStep,
};

use super::engine::WorkflowEngine;
use super::prompt_strategy::{DefaultWorkflowPromptStrategy, WorkflowPromptStrategy};
use super::types::{WorkflowPromptContext, DEFAULT_WORKFLOW};

fn make_engine() -> WorkflowEngine {
    WorkflowEngine::new()
}

/// Provider-neutral fixture coordinator for workflow unit tests (Object Mother).
const FIXTURE_WORKFLOW_COORDINATOR: &str = "fixture-entry";

fn default_manifest_with_workflow() -> AppManifest {
    let mut workflows = HashMap::new();
    workflows.insert(
        DEFAULT_WORKFLOW.into(),
        WorkflowDefinition {
            description: None,
            steps: vec![WorkflowStep {
                name: "coordinate".into(),
                agent: FIXTURE_WORKFLOW_COORDINATOR.into(),
                prompt_template: None,
                depends_on: vec![],
            }],
        },
    );
    AppManifest {
        id: macaca_proto::ApplicationId::new(),
        name: "workflow-app".into(),
        description: None,
        version: "0.1.0".into(),
        layer: AppLayer::L3Declarative,
        ui_type: None,
        agents: vec![],
        llm_config: None,
        entry_agent: None,
        entrypoint: None,
        workflows: Some(workflows),
        resources: None,
        context: None,
        service_contract: None,
        execution_profile: None,
        workbench: None,
        autonomy: None,
        ui: None,
        execution_control: None,
    }
}

fn example_app_dir(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest_dir.ancestors() {
        let candidate = ancestor.join("examples/apps").join(name);
        if candidate.join("app.yaml").exists() {
            return candidate;
        }
    }
    panic!(
        "failed to locate examples/apps/{name} from {}",
        manifest_dir.display()
    )
}

#[test]
fn default_prompts_are_valid() {
    let workflow = WorkflowEngine::default_workflow_prompt();
    assert!(workflow.contains("SDD"));
    assert!(workflow.contains("Tools"));

    let assistant = WorkflowEngine::default_assistant_prompt();
    assert!(assistant.contains("assistant"));
}

#[test]
fn default_workflow_strategy_matches_default_prompt() {
    let rendered = DefaultWorkflowPromptStrategy.render(&WorkflowPromptContext {
        manifest: default_manifest_with_workflow(),
        workflow_name: DEFAULT_WORKFLOW.into(),
        coordinator: FIXTURE_WORKFLOW_COORDINATOR.into(),
        additional_context: None,
    });
    assert_eq!(rendered, WorkflowEngine::default_workflow_prompt());
}

#[test]
fn build_system_prompt_appends_additional_context() {
    let engine = make_engine();
    let manifest = default_manifest_with_workflow();
    let app_dir = std::env::temp_dir();
    let prompt = engine
        .build_system_prompt(&manifest, &app_dir, DEFAULT_WORKFLOW, Some("EXTRA_CTX"))
        .unwrap();
    assert!(prompt.contains("EXTRA_CTX"));
}

#[test]
fn build_system_prompt_for_fullstack_fixture() {
    let engine = make_engine();
    let app_dir = example_app_dir("fullstack-autodev");
    let manifest = AppLoader::load_manifest(app_dir.join("app.yaml")).unwrap();
    let workflow_name = WorkflowEngine::get_entrypoint_workflow(&manifest);
    let prompt = engine
        .build_system_prompt(&manifest, &app_dir, &workflow_name, None)
        .unwrap();
    assert!(!prompt.trim().is_empty());
    assert!(prompt.contains("Use Tools") || prompt.contains("assistant"));
}

#[test]
fn build_system_prompt_for_newsroom_fixture() {
    let engine = make_engine();
    let app_dir = example_app_dir("newsroom-autowriter");
    let manifest = AppLoader::load_manifest(app_dir.join("app.yaml")).unwrap();
    let workflow_name = WorkflowEngine::get_entrypoint_workflow(&manifest);
    let prompt = engine
        .build_system_prompt(&manifest, &app_dir, &workflow_name, None)
        .unwrap();
    assert!(!prompt.trim().is_empty());
    assert!(prompt.contains("Use Tools") || prompt.contains("assistant"));
}

#[test]
fn build_system_prompt_uses_manifest_execution_tools_instead_of_single_hardcoded_driver() {
    let engine = make_engine();
    let manifest = AppManifest {
        agents: vec![AgentSource::Inline(InlineAgentConfig {
            name: FIXTURE_WORKFLOW_COORDINATOR.into(),
            capabilities: vec![],
            prompt_template: String::new(),
            model: "mock".into(),
            permission_level: "system".into(),
            allowed_tools: vec![
                "opencode_execute".into(),
                "claude_code_execute".into(),
                "file_write".into(),
            ],
            max_tokens: None,
            temperature: None,
            skills: None,
            context_engine: None,
        })],
        ..default_manifest_with_workflow()
    };
    let prompt = engine
        .build_system_prompt(&manifest, &std::env::temp_dir(), DEFAULT_WORKFLOW, None)
        .unwrap();
    assert!(prompt.contains("`opencode_execute`"));
    assert!(prompt.contains("`claude_code_execute`"));
    assert!(prompt.contains("Do not substitute one execution driver"));
}
