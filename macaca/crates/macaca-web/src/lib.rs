//! `macaca-web` — Simple web UI for Macaca OS.
//!
//! Provides an HTTP server with a single-page web interface for interacting
//! with Macaca OS applications. Uses axum for the HTTP layer.

pub mod routes;
pub mod state;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use macaca_app::{AppRegistry, AppRuntime};
use macaca_driver_claude_code::{ClaudeCodeConfig, ClaudeCodeDriver};
use macaca_kernel::Kernel;
use macaca_llm::{DashScopeProvider, LlmProvider};
use macaca_persist::RedbStore;
use macaca_proto::config::{KernelConfig, MacacaConfig};
use macaca_proto::{ApplicationId, LlmMessage, MacacaResult};
use macaca_sdk::AgentPersona;
use macaca_skill::{SkillCatalog, SkillRegistry};
use macaca_tools::{DefaultToolSet, Tool, ToolSet, OrchestrationState, DelegateTaskTool, GetTaskResultTool, ReportResultTool, ListAgentsTool};
use macaca_tools::orchestration::{AgentExecutor, AgentExecutionResult};
use futures::FutureExt;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Composite ToolSet: built-in + skill tools
// ---------------------------------------------------------------------------

/// A ToolSet that combines built-in tools with executable skill tools.
struct CompositeToolSet {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolSet for CompositeToolSet {
    fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }
}

/// Start the Macaca OS web server.
pub async fn start_server(port: u16) -> MacacaResult<()> {
    // 1. Load configuration from config/default.toml
    let config = MacacaConfig::load_default();
    info!(default_provider = %config.llm.default_provider, "Configuration loaded");

    // 2. Create LLM provider from configuration.
    let llm: Arc<dyn LlmProvider> = {
        let provider_name = &config.llm.default_provider;
        let provider_config = config.llm.providers.get(provider_name)
            .unwrap_or_else(|| panic!("LLM provider '{}' not found in config", provider_name));
        let api_key = provider_config.resolve_api_key()?;
        let base_url = &provider_config.base_url;

        match provider_name.as_str() {
            "dashscope" => Arc::new(DashScopeProvider::new(api_key).with_base_url(base_url.clone())),
            _ => panic!("Unsupported LLM provider: {}", provider_name),
        }
    };

    info!(provider = llm.name(), "LLM provider initialized");

    // 3. Create kernel.
    let kernel_config = KernelConfig {
        max_agents: 64,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 60000,
    };
    let kernel = Arc::new(Kernel::new(&kernel_config, Arc::clone(&llm), Box::new(DefaultToolSet::new())));

    // 4. Initialize app registry and discover apps.
    let mut registry = AppRegistry::new();
    let discovered = registry.discover_apps()?;
    info!(count = discovered.len(), "Apps discovered from standard directories");

    // 5. Start the runtime and load ALL discovered apps.
    let runtime = AppRuntime::new();
    let mut app_dirs = HashMap::new();
    let mut skills_dirs = Vec::new();

    // Auto-start all discovered apps
    for app in &discovered {
        let manifest_path = app.manifest_path.clone();
        if manifest_path.exists() {
            match runtime.start_app_from_file(&manifest_path, &kernel).await {
                Ok(app_id) => {
                    let agent_count = kernel.agent_count().await;
                    app_dirs.insert(app_id, app.path.clone());
                    skills_dirs.push(app.path.join("skills"));
                    info!(
                        app_id = %app_id.0,
                        app_name = %app.name,
                        agents = agent_count,
                        "App started"
                    );
                }
                Err(e) => {
                    tracing::warn!(app_name = %app.name, error = %e, "Failed to start app");
                }
            }
        }
    }

    // 6. Load skill catalog (SKILL.md knowledge skills).
    let mut catalog = SkillCatalog::new();
    for dir in &skills_dirs {
        if dir.exists() {
            match catalog.load_from_directory(&dir).await {
                Ok(n) => info!(count = n, "Knowledge skills loaded into catalog"),
                Err(e) => tracing::warn!("Failed to load knowledge skills: {e}"),
            }
        }
    }

    // 7. Build composite toolset: built-in tools + executable skill tools.
    let mut all_tools: Vec<Box<dyn Tool>> = vec![
        Box::new(macaca_tools::FileReadTool),
        Box::new(macaca_tools::FileWriteTool),
        Box::new(macaca_tools::ShellTool::default()),
    ];

    // Load executable skill tools from all app skills directories.
    for dir in &skills_dirs {
        if dir.exists() {
            let mut skill_registry = SkillRegistry::new();
            match skill_registry.load_from_directory(dir).await {
                Ok(n) => {
                    let skill_tools = skill_registry.instantiate_all_tools();
                    info!(count = n, "Executable skill tools loaded");
                    all_tools.extend(skill_tools);
                }
                Err(e) => tracing::warn!("Failed to load executable skills: {e}"),
            }
        }
    }

    // Load Claude Code driver tools (claude_code_execute, resume, status).
    let cc_work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cc_config = ClaudeCodeConfig::new(cc_work_dir)
        .dangerously_skip_permissions()
        .with_timeout(600);
    let cc_driver = ClaudeCodeDriver::new(cc_config);
    let cc_tools = macaca_driver::driver::SoftwareDriver::tools(&cc_driver);
    info!(count = cc_tools.len(), "Claude Code driver tools loaded");
    all_tools.extend(cc_tools);

    // 8. Initialize orchestration state and add orchestration tools.
    let orchestration: Arc<tokio::sync::RwLock<OrchestrationState>> = Arc::new(tokio::sync::RwLock::new(OrchestrationState::new()));

    // NOTE: DelegateTaskTool is added WITHOUT executor for now.
    // The delegated tasks will be stored in pending_tasks but not automatically executed.
    // This is a limitation that can be addressed in a future iteration.
    // For now, the coordinator can use claude_code_execute to run actual code.
    all_tools.push(Box::new(DelegateTaskTool::new(Arc::clone(&orchestration))));
    all_tools.push(Box::new(GetTaskResultTool::new(Arc::clone(&orchestration))));
    all_tools.push(Box::new(ReportResultTool::new(Arc::clone(&orchestration))));

    // Create dynamic ListAgentsTool that fetches from kernel
    let kernel_for_callback = Arc::clone(&kernel);
    let list_agents_tool = ListAgentsTool::new()
        .with_agents_callback(move || {
            let kernel = Arc::clone(&kernel_for_callback);
            async move {
                let agents = kernel.list_agents().await;
                agents
                    .into_iter()
                    .map(|agent| {
                        let capabilities: Vec<String> = agent
                            .capabilities
                            .into_iter()
                            .map(|cap| cap.name)
                            .collect();
                        serde_json::json!({
                            "name": agent.name,
                            "capabilities": capabilities
                        })
                    })
                    .collect()
            }
            .boxed()
        });
    all_tools.push(Box::new(list_agents_tool));
    info!("Orchestration tools added: delegate_task, get_task_result, report_result, list_agents");

    let tool_names: Vec<&str> = all_tools.iter().map(|t| t.name()).collect();
    info!(tools = ?tool_names, "Composite toolset ready");

    let tools: Box<dyn ToolSet> = Box::new(CompositeToolSet { tools: all_tools });

    // 9. Initialize persistent session store.
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("macaca");
    std::fs::create_dir_all(&data_dir).ok();
    let session_db_path = data_dir.join("sessions.db");
    let session_store = Arc::new(RedbStore::open(&session_db_path)?);
    info!(path = %session_db_path.display(), "Session store initialized");

    // 10. Build shared state.
    let state = Arc::new(AppState {
        kernel,
        runtime,
        registry: tokio::sync::RwLock::new(registry),
        catalog: tokio::sync::RwLock::new(catalog),
        llm,
        app_dirs: tokio::sync::RwLock::new(app_dirs),
        tools,
        sessions: tokio::sync::RwLock::new(HashMap::new()),
        cancel_flags: tokio::sync::RwLock::new(HashMap::new()),
        session_store,
        orchestration,
    });

    // 11. Build axum router.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(routes::root_not_found))
        .route("/api/status", get(routes::get_status))
        .route("/api/apps", get(routes::get_apps))
        .route("/api/apps/{id}", get(routes::get_app))
        .route("/api/apps/{id}/agents", get(routes::get_app_agents))
        .route("/api/apps/{id}/agents/stream", get(routes::stream_agent_status))
        .route("/api/apps/{id}/sessions", get(routes::list_app_sessions))
        .route("/api/apps/reload", post(routes::reload_apps))
        .route("/api/sessions", get(routes::list_sessions))
        .route("/api/sessions/{app_id}", get(routes::get_session))
        .route("/api/sessions/detail/{session_id}", get(routes::get_session_by_id))
        .route("/api/sessions/detail/{session_id}", axum::routing::delete(routes::delete_session))
        .route("/api/skills", get(routes::get_skills))
        .route("/api/chat", post(routes::post_chat))
        .route("/api/chat/stop", post(routes::post_stop))
        .layer(cors)
        .with_state(state);

    // 12. Start server.
    let addr = format!("0.0.0.0:{port}");
    info!(addr = %addr, "Macaca OS API server starting");
    println!("\n  Macaca OS API server: http://localhost:{port}\n");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| macaca_proto::MacacaError::Io(e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| macaca_proto::MacacaError::Io(e))?;

    Ok(())
}
