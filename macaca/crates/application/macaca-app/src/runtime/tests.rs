//! Unit tests for [`super::AppRuntime`] bootstrap and lifecycle behavior.
//!
//! Exercises manifest validation, agent registration, duplicate rejection, and
//! layer-specific bootstrap paths (L1 native, L2 WASM, L3 declarative).

use super::*;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_agent::{AgentExecutionPort, InProcessAgentExecutionPort, ToolCatalog};
use macaca_kernel::KernelBuilder;
use macaca_llm::LlmProvider;
use macaca_proto::config::KernelConfig;
use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult as Res, TokenUsage};
use macaca_tools::DefaultToolSet;

use crate::model::{AgentSource, AppLayer, CapabilityRef, InlineAgentConfig};

/// Minimal LLM stub so kernel agent registration can complete in unit tests.
struct MockLlm;

#[async_trait]
impl LlmProvider for MockLlm {
    fn name(&self) -> &str {
        "mock"
    }
    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> Res<LlmResponse> {
        Ok(LlmResponse {
            content: "ok".into(),
            reasoning_content: None,
            model: "mock".into(),
            usage: TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
            },
            finish_reason: "stop".into(),
            tool_calls: None,
        })
    }
}

/// Build an in-memory kernel with a mock execution port for bootstrap tests.
fn make_kernel() -> Kernel {
    let config = KernelConfig {
        max_agents: 64,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 30000,
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
    let execution_port: Arc<dyn AgentExecutionPort> =
        Arc::new(InProcessAgentExecutionPort::new(
            llm,
            Arc::from(Box::new(DefaultToolSet::new()) as Box<dyn ToolCatalog>),
        ));
    KernelBuilder::from_execution_port(config, execution_port).build()
}

/// Construct a minimal L3 declarative manifest for lifecycle assertions.
fn inline_manifest(name: &str) -> AppManifest {
    AppManifest {
        id: ApplicationId::new(),
        name: name.into(),
        description: None,
        version: "0.1.0".into(),
        layer: AppLayer::L3Declarative,
        ui_type: None,
        agents: vec![AgentSource::Inline(InlineAgentConfig {
            name: format!("{name}-agent"),
            capabilities: vec![CapabilityRef {
                name: "test".into(),
                description: "test cap".into(),
            }],
            prompt_template: "You are a test agent.".into(),
            model: "mock".into(),
            permission_level: "user".into(),
            allowed_tools: vec![],
            max_tokens: None,
            temperature: None,
            skills: None,
            context_engine: None,
        })],
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
    }
}

#[tokio::test]
async fn bootstrap_and_list_app() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("app1");
    let expected_id = manifest.id;

    let app_id = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    assert_eq!(app_id, expected_id);

    let apps = runtime.list_apps().await;
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].0, expected_id);
    assert_eq!(apps[0].1, "app1");
    assert_eq!(apps[0].2, AppStatus::Running);
}

#[tokio::test]
async fn bootstrap_duplicate_app_fails() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("dup");

    runtime
        .bootstrap_manifest(manifest.clone(), ".", &kernel)
        .await
        .unwrap();
    let err = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("already loaded"));
}

#[tokio::test]
async fn stop_app() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("stop-test");

    let app_id = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    runtime.stop_app(&app_id, &kernel).await.unwrap();

    let status = runtime.app_status(&app_id).await.unwrap();
    assert_eq!(status, AppStatus::Stopped);
}

#[tokio::test]
async fn stop_already_stopped_is_ok() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("stop2");

    let app_id = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    runtime.stop_app(&app_id, &kernel).await.unwrap();
    // Stopping again is a no-op
    runtime.stop_app(&app_id, &kernel).await.unwrap();
}

#[tokio::test]
async fn remove_stopped_app() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("rm-test");

    let app_id = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    runtime.stop_app(&app_id, &kernel).await.unwrap();
    runtime.remove_app(&app_id).await.unwrap();
    assert_eq!(runtime.app_count().await, 0);
}

#[tokio::test]
async fn remove_running_app_fails() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("rm-running");

    let app_id = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    let err = runtime.remove_app(&app_id).await.unwrap_err();
    assert!(err.to_string().contains("still running"));
}

#[tokio::test]
async fn app_agents_returns_ids() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("agents-test");

    let app_id = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    let agents = runtime.app_agents(&app_id).await.unwrap();
    assert_eq!(agents.len(), 1);

    // Also verify the kernel has the agent
    assert_eq!(kernel.agent_count().await, 1);
}

#[tokio::test]
async fn stop_nonexistent_app_fails() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let fake_id = ApplicationId::new();
    let err = runtime.stop_app(&fake_id, &kernel).await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test]
async fn wasm_app_can_bootstrap_without_declarative_agents() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = AppManifest {
        id: ApplicationId::new(),
        name: "wasm-app".into(),
        description: None,
        version: "0.1.0".into(),
        layer: AppLayer::L2Wasm,
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
    };
    let app_id = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    let agents = runtime.app_agents(&app_id).await.unwrap();
    assert!(agents.is_empty());
    assert_eq!(kernel.agent_count().await, 0);
}

#[tokio::test]
async fn wasm_app_with_declared_agents_registers_app_scoped_agents() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let mut manifest = inline_manifest("wasm-agent-app");
    manifest.layer = AppLayer::L2Wasm;

    let app_id = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    let agents = runtime.app_agents(&app_id).await.unwrap();

    assert_eq!(agents.len(), 1);
    assert_eq!(kernel.agent_count().await, 1);
}

#[tokio::test]
async fn bootstrap_manifest_from_path() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let dir = std::env::temp_dir().join("macaca_app_runtime_test");
    std::fs::create_dir_all(&dir).unwrap();
    let manifest_path = dir.join("app.yaml");
    std::fs::write(
        &manifest_path,
        r#"
name: runtime-file-app
layer: L3Declarative
agents:
  - name: runtime-file-agent
    prompt_template: "You are runtime-file-agent."
    model: "mock"
    capabilities:
      - name: assist
"#,
    )
    .unwrap();

    let app_id = runtime
        .bootstrap_manifest_from_path(&manifest_path, &kernel)
        .await
        .unwrap();
    let status = runtime.app_status(&app_id).await.unwrap();
    assert_eq!(status, AppStatus::Running);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn runtime_builder_preserves_manifest_validation_and_assembly() {
    let manifest = inline_manifest("builder-app");
    let builder = AppRuntimeBuilder::new(manifest.clone(), ".");
    builder.validate().unwrap();
    let loaded = builder.assemble_loaded_app(vec![AgentId::new()]).unwrap();
    assert_eq!(loaded.manifest.name, manifest.name);
    assert_eq!(loaded.status, AppStatus::Running);
    assert_eq!(loaded.agent_ids.len(), 1);
}

#[tokio::test]
async fn native_app_no_agents() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = AppManifest {
        id: ApplicationId::new(),
        name: "native-app".into(),
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
    };
    let app_id = runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    let agents = runtime.app_agents(&app_id).await.unwrap();
    assert!(agents.is_empty());
    assert_eq!(kernel.agent_count().await, 0);
}

#[tokio::test]
async fn find_by_name() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("findme");
    let expected_id = manifest.id;

    runtime
        .bootstrap_manifest(manifest, ".", &kernel)
        .await
        .unwrap();
    let found = runtime.find_by_name("findme").await;
    assert_eq!(found, Some(expected_id));
    assert!(runtime.find_by_name("nonexistent").await.is_none());
}
