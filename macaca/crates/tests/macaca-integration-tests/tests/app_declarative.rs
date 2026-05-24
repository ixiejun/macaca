//! Integration tests for AppRuntime with declarative agents:
//! start/stop apps, verify agent registration, and app status transitions.

use std::sync::Arc;

use async_trait::async_trait;

use macaca_app::model::{AgentSource, CapabilityRef, InlineAgentConfig};
use macaca_app::{AppLayer, AppManifest, AppRuntime, AppStatus};
use macaca_kernel::{Kernel, KernelBuilder, KernelServiceClientCompat};
use macaca_llm::LlmProvider;
use macaca_proto::config::KernelConfig;
use macaca_proto::{ApplicationId, LlmMessage, LlmOptions, LlmResponse, MacacaResult, TokenUsage};
use macaca_tools::DefaultToolSet;

// ---------------------------------------------------------------------------
// Mock LLM
// ---------------------------------------------------------------------------

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
    ) -> MacacaResult<LlmResponse> {
        Ok(LlmResponse {
            content: "app-test-response".into(),
            reasoning_content: None,
            model: "mock".into(),
            usage: TokenUsage {
                prompt_tokens: 2,
                completion_tokens: 2,
                total_tokens: 4,
            },
            finish_reason: "stop".into(),
            tool_calls: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_kernel() -> Kernel {
    let config = KernelConfig {
        max_agents: 64,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 30000,
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
    KernelBuilder::from_service_clients(
        config,
        KernelServiceClientCompat::from_agent_provider_boxed_tools(
            llm,
            Box::new(DefaultToolSet::new()),
        ),
    )
    .build()
}

fn inline_manifest(name: &str, agent_count: usize) -> AppManifest {
    let agents: Vec<AgentSource> = (0..agent_count)
        .map(|i| {
            AgentSource::Inline(InlineAgentConfig {
                name: format!("{name}-agent-{i}"),
                capabilities: vec![CapabilityRef {
                    name: "test".into(),
                    description: "test capability".into(),
                }],
                prompt_template: format!("You are agent {i} of app {name}."),
                model: "mock".into(),
                permission_level: "user".into(),
                allowed_tools: vec![],
                max_tokens: None,
                temperature: None,
                skills: None,
                context_engine: None,
            })
        })
        .collect();

    AppManifest {
        id: ApplicationId::new(),
        name: name.into(),
        description: None,
        version: "0.1.0".into(),
        layer: AppLayer::L3Declarative,
        ui_type: None,
        agents,
        llm_config: None,
        entry_agent: None,
        entrypoint: None,
        workflows: None,
        resources: None,
        context: None,
        service_contract: None,
        ui: None,
        // These declarative application fixtures intentionally exercise the
        // classic manifest path without autonomous runtime declarations. The
        // application framework owns autonomy as an optional manifest contract,
        // so tests that do not opt in must state the absence explicitly instead
        // of relying on field omission.
        autonomy: None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Start a declarative app, verify it is Running with agents registered.
#[tokio::test]
async fn start_declarative_app_registers_agents() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("test-app", 2);

    let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();

    // Verify app status
    let status = runtime.app_status(&app_id).await.unwrap();
    assert_eq!(status, AppStatus::Running);

    // Verify agents registered in kernel
    assert_eq!(kernel.agent_count().await, 2);

    // Verify app_agents returns correct count
    let agent_ids = runtime.app_agents(&app_id).await.unwrap();
    assert_eq!(agent_ids.len(), 2);

    // Verify the kernel has manifests for each agent
    let manifests = kernel.list_agents().await;
    assert_eq!(manifests.len(), 2);
}

/// Stop an app and verify agents are unregistered from the kernel.
#[tokio::test]
async fn stop_app_unregisters_agents() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("stop-app", 1);

    let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();
    assert_eq!(kernel.agent_count().await, 1);

    runtime.stop_app(&app_id, &kernel).await.unwrap();

    let status = runtime.app_status(&app_id).await.unwrap();
    assert_eq!(status, AppStatus::Stopped);
    assert_eq!(kernel.agent_count().await, 0);
}

/// Start multiple apps and verify they coexist.
#[tokio::test]
async fn multiple_apps_coexist() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();

    let app_a = runtime
        .start_app(inline_manifest("app-a", 2), ".", &kernel)
        .await
        .unwrap();
    let app_b = runtime
        .start_app(inline_manifest("app-b", 3), ".", &kernel)
        .await
        .unwrap();

    assert_eq!(runtime.app_count().await, 2);
    assert_eq!(kernel.agent_count().await, 5);

    let apps = runtime.list_apps().await;
    assert_eq!(apps.len(), 2);

    // Stop one, the other remains
    runtime.stop_app(&app_a, &kernel).await.unwrap();
    assert_eq!(kernel.agent_count().await, 3);

    runtime.stop_app(&app_b, &kernel).await.unwrap();
    assert_eq!(kernel.agent_count().await, 0);
}

/// Stopping and removing an app, then verifying it is gone.
#[tokio::test]
async fn stop_and_remove_app() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("removable", 1);

    let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();
    assert_eq!(runtime.app_count().await, 1);

    runtime.stop_app(&app_id, &kernel).await.unwrap();
    runtime.remove_app(&app_id).await.unwrap();
    assert_eq!(runtime.app_count().await, 0);
}

/// Duplicate app (same manifest/ID) is rejected.
#[tokio::test]
async fn duplicate_app_rejected() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest = inline_manifest("dup", 1);

    runtime
        .start_app(manifest.clone(), ".", &kernel)
        .await
        .unwrap();
    let err = runtime.start_app(manifest, ".", &kernel).await.unwrap_err();
    assert!(err.to_string().contains("already loaded"));
}

/// Removing a running app fails.
#[tokio::test]
async fn remove_running_app_fails() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();

    let app_id = runtime
        .start_app(inline_manifest("still-running", 1), ".", &kernel)
        .await
        .unwrap();
    let err = runtime.remove_app(&app_id).await.unwrap_err();
    assert!(err.to_string().contains("still running"));
}
