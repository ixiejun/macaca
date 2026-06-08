//! Integration tests for the fullstack-autodev application.
//!
//! Tests that the Application manifest loads correctly, skills parse,
//! personas load, and the Claude Code Driver integrates with Agent OS.

use std::sync::Arc;

use async_trait::async_trait;

use macaca_app::loader::AppLoader;
use macaca_app::{AppLayer, AppRuntime, AppStatus};
use macaca_agent::{AgentExecutionPort, InProcessAgentExecutionPort, ToolCatalog};
use macaca_kernel::{Kernel, KernelBuilder};
use macaca_llm::LlmProvider;
use macaca_proto::config::KernelConfig;
use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult, TokenUsage};
use macaca_sdk::AgentPersona;
use macaca_skill::{ExecutableSkillToolSet, SkillCatalog};
use macaca_tools::{DefaultToolSet, Tool};

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
            content: "fullstack-autodev-response".into(),
            reasoning_content: None,
            model: "mock".into(),
            usage: TokenUsage {
                prompt_tokens: 5,
                completion_tokens: 5,
                total_tokens: 10,
            },
            finish_reason: "stop".into(),
            tool_calls: None,
        })
    }
}

fn make_kernel() -> Kernel {
    let config = KernelConfig {
        max_agents: 64,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 30000,
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
    let execution_port: Arc<dyn AgentExecutionPort> = Arc::new(InProcessAgentExecutionPort::new(llm, Arc::from(Box::new(DefaultToolSet::new()) as Box<dyn ToolCatalog>)));
    KernelBuilder::from_execution_port(config, execution_port).build()
}

fn app_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest_dir.ancestors() {
        let candidate = ancestor.join("examples/apps/fullstack-autodev");
        if candidate.join("app.yaml").exists() {
            return candidate;
        }
    }
    panic!(
        "failed to locate examples/apps/fullstack-autodev from {}",
        manifest_dir.display()
    )
}

// ---------------------------------------------------------------------------
// App Manifest Tests
// ---------------------------------------------------------------------------

/// Verify the fullstack-autodev app.yaml loads and parses correctly.
#[tokio::test]
async fn app_manifest_loads() {
    let manifest_path = app_dir().join("app.yaml");
    assert!(
        manifest_path.exists(),
        "app.yaml not found at {:?}",
        manifest_path
    );

    let manifest = AppLoader::load_manifest(&manifest_path).unwrap();

    assert_eq!(manifest.name, "fullstack-autodev");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.layer, AppLayer::L3Declarative);
    assert_eq!(manifest.agents.len(), 5);

    // LLM config present
    let llm_config = manifest.llm_config.as_ref().unwrap();
    assert_eq!(llm_config.provider, "volces");
    assert_eq!(llm_config.model, "kimi-k2.5");
}

/// Start the fullstack-autodev app and verify 5 agents are registered.
#[tokio::test]
async fn app_starts_with_three_agents() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest_path = app_dir().join("app.yaml");

    let app_id = runtime
        .bootstrap_manifest_from_path(&manifest_path, &kernel)
        .await
        .unwrap();

    let status = runtime.app_status(&app_id).await.unwrap();
    assert_eq!(status, AppStatus::Running);

    // 5 agents: coordinator, architect, frontend, planner, backend
    let agent_ids = runtime.app_agents(&app_id).await.unwrap();
    assert_eq!(agent_ids.len(), 5);
    assert_eq!(kernel.agent_count().await, 5);

    // Verify agent names
    let manifests = kernel.list_agents().await;
    let names: Vec<&str> = manifests.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"coordinator"));
    assert!(names.contains(&"architect"));
    assert!(names.contains(&"frontend"));
    assert!(names.contains(&"planner"));
    assert!(names.contains(&"backend"));

    // Cleanup
    runtime.stop_app(&app_id, &kernel).await.unwrap();
    assert_eq!(kernel.agent_count().await, 0);
}

// ---------------------------------------------------------------------------
// Skills Tests — Executable Skills (YAML)
// ---------------------------------------------------------------------------

/// Verify YAML skill files load correctly via ExecutableSkillToolSet.
#[tokio::test]
async fn yaml_skills_load_from_registry() {
    let skills_dir = app_dir().join("skills");
    assert!(skills_dir.exists(), "skills directory not found");

    let mut toolset = ExecutableSkillToolSet::new();
    let loaded = toolset.load_from_directory(&skills_dir).await.unwrap();
    let snapshot = toolset.snapshot();

    // YAML skills: root-level `openspec.yaml` only. SKILL.md dirs (openspec/, figma-mcp/, etc.)
    // are agent skills (`SkillCatalog`), not executable YAML tools here.
    assert_eq!(loaded, 1, "Expected 1 YAML skill, got {loaded}");
    assert!(
        snapshot.skills.iter().any(|skill| skill.name == "openspec"),
        "openspec YAML skill missing"
    );
    assert!(
        !snapshot
            .skills
            .iter()
            .any(|skill| skill.name == "figma-mcp"),
        "figma-mcp must not appear in YAML executable snapshot when only SKILL.md is present"
    );

    // SKILL.md skills should NOT be in the executable skill snapshot.
    assert!(
        !snapshot.skills.iter().any(|skill| skill.name == "golang"),
        "golang should not be in executable skill snapshot"
    );
    assert!(
        !snapshot
            .skills
            .iter()
            .any(|skill| skill.name == "shadcn-ui"),
        "shadcn-ui should not be in executable skill snapshot"
    );
}

/// Verify YAML skills can be instantiated as tools.
#[tokio::test]
async fn yaml_skills_instantiate_as_tools() {
    let skills_dir = app_dir().join("skills");
    let mut toolset = ExecutableSkillToolSet::new();
    toolset.load_from_directory(&skills_dir).await.unwrap();

    // YAML skill → executable tool
    let openspec_tool = toolset.tool("openspec").unwrap();
    assert_eq!(openspec_tool.name(), "openspec");
}

// ---------------------------------------------------------------------------
// Skills Tests — Agent Skills (SKILL.md / agentskills.io)
// ---------------------------------------------------------------------------

/// Verify SKILL.md files load correctly via SkillCatalog.
#[tokio::test]
async fn agent_skills_load_via_catalog() {
    let skills_dir = app_dir().join("skills");

    let mut catalog = SkillCatalog::new();
    let loaded = catalog.load_from_directory(&skills_dir).await.unwrap();

    // SKILL.md dirs: golang, openspec/, figma-mcp/, shadcn-ui/.
    assert_eq!(loaded, 4, "Expected 4 agent skills, got {loaded}");

    // Verify catalog entries (tier 1 — name + description only).
    let golang = catalog
        .get("golang")
        .expect("golang skill missing from catalog");
    assert!(golang.description.contains("Go"));

    let openspec = catalog
        .get("openspec")
        .expect("openspec skill missing from catalog");
    assert!(openspec.description.len() > 0);

    let figma = catalog
        .get("figma-mcp")
        .expect("figma-mcp skill missing from catalog");
    assert!(
        figma.description.contains("Figma") || figma.description.contains("figma"),
        "figma-mcp catalog description should mention Figma"
    );

    let shadcn = catalog
        .get("shadcn-ui")
        .expect("shadcn-ui skill missing from catalog");
    assert!(shadcn.description.contains("shadcn"));
}

/// Verify agent skills can be activated (tier 2 — full content loaded).
#[tokio::test]
async fn agent_skills_activate() {
    let skills_dir = app_dir().join("skills");

    let mut catalog = SkillCatalog::new();
    catalog.load_from_directory(&skills_dir).await.unwrap();

    // Activate golang skill (tier 2).
    let golang = catalog.activate("golang").await.unwrap();
    assert_eq!(golang.name, "golang");
    assert!(
        golang.content.contains("chi router"),
        "golang skill should mention chi router"
    );

    // Activate shadcn-ui skill (tier 2).
    let shadcn = catalog.activate("shadcn-ui").await.unwrap();
    assert_eq!(shadcn.name, "shadcn-ui");
    assert!(
        shadcn.content.contains("Radix UI"),
        "shadcn-ui skill should mention Radix UI"
    );

    // Both should be marked as activated.
    assert!(catalog.is_activated("golang"));
    assert!(catalog.is_activated("shadcn-ui"));
    assert_eq!(catalog.activated_count(), 2);
}

/// Verify the catalog generates a prompt suitable for injection.
#[tokio::test]
#[allow(deprecated)] // `catalog_prompt` is deprecated with migration to composer capability providers.
async fn agent_skills_catalog_prompt() {
    let skills_dir = app_dir().join("skills");

    let mut catalog = SkillCatalog::new();
    catalog.load_from_directory(&skills_dir).await.unwrap();

    let prompt = catalog.catalog_prompt();
    assert!(prompt.contains("<available_skills>"));
    assert!(prompt.contains("golang"));
    assert!(prompt.contains("shadcn-ui"));
    assert!(prompt.contains("</available_skills>"));
}

// ---------------------------------------------------------------------------
// Persona Tests
// ---------------------------------------------------------------------------

/// Verify architect persona loads correctly.
#[tokio::test]
async fn architect_persona_loads() {
    let persona_dir = app_dir().join("personas/architect");
    assert!(persona_dir.exists(), "architect persona dir not found");

    let persona = AgentPersona::load_from_directory(&persona_dir)
        .await
        .unwrap();
    assert!(!persona.is_empty());
    assert!(persona.identity.is_some());
    assert!(persona.tools.is_some());

    // Verify system prompt generation
    let prompt = persona.to_system_prompt(Some("Base prompt."));
    assert!(prompt.contains("Base prompt."));
    assert!(prompt.contains("Architect Agent"));
}

/// Verify frontend persona loads correctly.
#[tokio::test]
async fn frontend_persona_loads() {
    let persona_dir = app_dir().join("personas/frontend");
    let persona = AgentPersona::load_from_directory(&persona_dir)
        .await
        .unwrap();
    assert!(persona.identity.is_some());
    assert!(persona.tools.is_some());

    let prompt = persona.to_system_prompt(None);
    assert!(prompt.contains("Frontend Agent"));
    assert!(prompt.contains("shadcn-ui"));
}

/// Verify backend persona loads correctly.
#[tokio::test]
async fn backend_persona_loads() {
    let persona_dir = app_dir().join("personas/backend");
    let persona = AgentPersona::load_from_directory(&persona_dir)
        .await
        .unwrap();
    assert!(persona.identity.is_some());
    assert!(persona.tools.is_some());

    let prompt = persona.to_system_prompt(None);
    assert!(prompt.contains("Backend Agent"));
    assert!(prompt.contains("Go"));
}

// ---------------------------------------------------------------------------
// Claude Code Driver Tests (disabled — driver moved to external plugin)
// ---------------------------------------------------------------------------

/// Verify the Claude Code Driver implements SoftwareDriver correctly.
#[tokio::test]
#[ignore = "claude-code driver moved to external plugin"]
async fn claude_code_driver_lifecycle() {
    // This test requires the macaca-driver-claude-code crate which has been
    // extracted as an external driver plugin. Re-enable once a DriverLoader-
    // based integration test is available.
}

/// Verify execute tool validates parameters.
#[tokio::test]
#[ignore = "claude-code driver moved to external plugin"]
async fn claude_code_execute_validates_params() {}

/// Verify status tool returns a result even if claude is not installed.
#[tokio::test]
#[ignore = "claude-code driver moved to external plugin"]
async fn claude_code_status_graceful() {}

// ---------------------------------------------------------------------------
// E2E Test (requires real Claude Code CLI)
// ---------------------------------------------------------------------------

/// End-to-end test: start app, execute architect agent, verify output.
/// Requires: claude CLI installed, valid API key.
#[tokio::test]
#[ignore]
async fn e2e_fullstack_autodev_architect_executes() {
    let runtime = AppRuntime::new();
    let kernel = make_kernel();
    let manifest_path = app_dir().join("app.yaml");

    let app_id = runtime
        .bootstrap_manifest_from_path(&manifest_path, &kernel)
        .await
        .unwrap();

    // Get the architect agent ID
    let agent_ids = runtime.app_agents(&app_id).await.unwrap();
    assert!(!agent_ids.is_empty());

    // Execute architect agent
    let architect_id = &agent_ids[0];
    let output = kernel.execute_agent(architect_id).await.unwrap();
    assert!(!output.result.is_empty());

    runtime.stop_app(&app_id, &kernel).await.unwrap();
}
