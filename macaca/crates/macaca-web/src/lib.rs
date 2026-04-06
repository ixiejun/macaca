//! `macaca-web` — Simple web UI for Macaca OS.
//!
//! Provides an HTTP server with a single-page web interface for interacting
//! with Macaca OS applications. Uses axum for the HTTP layer.

pub mod agent_runner;
pub mod event_persistence;
pub mod hook_consumer;
pub mod metrics;
pub mod routes;
pub mod state;
pub mod workspace;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use macaca_app::{AppRegistry, AppRuntime};
use macaca_driver_claude_code::{ClaudeCodeConfig, ClaudeCodeDriver};
use macaca_kernel::{Kernel, ApplicationExecutorRegistry, AgentInfo};
use macaca_llm::{CostTracker, DashScopeProvider, OpenAiProvider, AnthropicProvider, OpenAiCompatibleProvider, LlmProvider, RateLimiter, ResilientConfig, ResilientLlmWrapper};
use macaca_persist::RedbStore;
use macaca_proto::config::{KernelConfig, MacacaConfig};
use macaca_proto::{ApplicationId, LlmMessage, MacacaResult};
use macaca_sdk::AgentPersona;
use macaca_skill::{SkillCatalog, SkillRegistry};
use macaca_tools::{DefaultToolSet, Tool, ToolSet, DelegateTaskTool, GetTaskResultTool, ListAgentsTool};
use futures::FutureExt;

use crate::state::AppState;
use crate::agent_runner::WebAgentRunner;

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

        let base_provider: Arc<dyn LlmProvider> = match provider_name.as_str() {
            "dashscope" => Arc::new(DashScopeProvider::new(api_key).with_base_url(base_url.clone())),
            "openai" => Arc::new(OpenAiProvider::new(api_key).with_base_url(base_url.clone())),
            "anthropic" => Arc::new(AnthropicProvider::new(api_key).with_base_url(base_url.clone())),
            // Any other provider name → treat as OpenAI-compatible (DeepSeek, Ollama, vLLM, etc.)
            other => {
                info!(provider = other, base_url = %base_url, "Using OpenAI-compatible provider");
                Arc::new(OpenAiCompatibleProvider::new(other, base_url.clone(), api_key))
            }
        };

        let cost_tracker = CostTracker::new();
        let rate_limiter = RateLimiter::per_minute(60);
        // Get default_model for fallback
        let default_model_name = config.llm.providers.get(&config.llm.default_provider)
            .and_then(|p| p.default_model.clone())
            .unwrap_or_default();
        let resilient_config = ResilientConfig {
            max_retries: 3,
            backoff_base_ms: 60_000,   // 1 minute between retries
            backoff_max_ms: 60_000,    // Cap at 1 minute
            retry_on_status: vec![429, 500, 502, 503],
            max_budget_usd: None,
            // Fallback models: try the provider's default_model as a last resort.
            // In production, configure multiple fallback models in config.
            fallback_models: if default_model_name.is_empty() { Vec::new() } else { vec![default_model_name] },
        };
        Arc::new(
            ResilientLlmWrapper::new(base_provider)
                .with_config(resilient_config)
                .with_rate_limiter(rate_limiter)
                .with_cost_tracker(cost_tracker),
        )
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
    let mut started_apps: Vec<(macaca_proto::ApplicationId, String)> = Vec::new();

    // Auto-start all discovered apps
    for app in &discovered {
        let manifest_path = app.manifest_path.clone();
        if manifest_path.exists() {
            match runtime.start_app_from_file(&manifest_path, &kernel).await {
                Ok(app_id) => {
                    let agent_count = kernel.agent_count().await;
                    app_dirs.insert(app_id, app.path.clone());
                    skills_dirs.push(app.path.join("skills"));
                    started_apps.push((app_id.clone(), app.name.clone()));
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

    // 8. Initialize orchestration tools.
    // We need to create a shared reference for the executor registry that can be
    // populated after state creation. This allows delegate_task tool to access the registry.
    let executor_registry_ref: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>> =
        Arc::new(RwLock::new(None));

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
    info!("ListAgents tool added");

    // Create DelegateTaskTool with callback to executor registry
    // Uses Fork-Join workflow: creates a Fork that inherits parent context
    let delegate_session_id: Arc<tokio::sync::RwLock<Option<String>>> =
        Arc::new(tokio::sync::RwLock::new(None));
    let registry_for_delegate = Arc::clone(&executor_registry_ref);
    let delegate_tool = DelegateTaskTool::empty_with_session_id(Arc::clone(&delegate_session_id))
        .with_callback(move |app_id, to_agent, prompt, priority, parallel, session_id| {
            let registry = Arc::clone(&registry_for_delegate);
            async move {
                // Get the registry reference
                let registry_guard = registry.read().await;
                let registry = registry_guard.as_ref()
                    .ok_or_else(|| "Executor registry not initialized".to_string())?;

                // If app_id is empty, use the first registered app
                let app_id = if app_id.is_empty() {
                    let apps = registry.list_applications().await;
                    apps.first()
                        .map(|(id, _)| id.clone())
                        .ok_or_else(|| "No applications registered in executor".to_string())?
                } else {
                    // Parse string to ApplicationId
                    uuid::Uuid::parse_str(&app_id)
                        .map(macaca_proto::ApplicationId)
                        .map_err(|e| format!("Invalid application ID: {}", e))?
                };

                // Get the executor for this app
                let executor = registry.get(&app_id).await
                    .ok_or_else(|| format!("App '{}' not found in registry", app_id))?;

                // Get the ForkManager from the executor
                let fork_manager = executor.fork_manager();

                // Create acceptance criteria
                let acceptance_criteria = macaca_proto::AcceptanceCriteria {
                    description: format!("Task delegated to {}: {}", to_agent, prompt.chars().take(100).collect::<String>()),
                    required_artifacts: vec![],
                    auto_accept: false,
                };

                // Create a Fork for this delegation
                let fork_id = fork_manager.create_fork(
                    None,  // parent_fork_id - None for now, will be set by caller context
                    app_id.clone(),
                    to_agent.clone(),
                    prompt.clone(),
                    vec![],  // inherited_messages - will be populated by caller
                    String::new(),  // system_prompt - will be set by caller
                    acceptance_criteria,
                ).await.map_err(|e| format!("Fork creation failed: {}", e))?;

                // Start the fork (transition from Pending to Running)
                fork_manager.start_fork(fork_id).await
                    .map_err(|e| format!("Fork start failed: {}", e))?;

                // Also delegate the actual task to the executor
                let task_context = session_id.map(|sid| macaca_kernel::TaskContext {
                    session_id: Some(sid),
                    artifacts: vec![],
                    env: std::collections::HashMap::new(),
                });
                let task_id = executor.delegate_task(
                    "coordinator",  // from_agent
                    &to_agent,
                    prompt,
                    priority,
                    parallel,
                    task_context,
                ).await.map_err(|e| format!("Delegation failed: {}", e))?;

                // Suspend the fork waiting for the task to complete
                fork_manager.suspend_fork(fork_id, macaca_proto::TaskId(task_id.0)).await
                    .map_err(|e| format!("Fork suspend failed: {}", e))?;

                // Return the fork_id - caller can use get_fork_result to check status
                Ok(format!("fork:{}", fork_id))
            }
            .boxed()
        });
    all_tools.push(Box::new(delegate_tool));
    info!("DelegateTask tool added");

    // Create GetTaskResultTool with callback to executor registry
    // Supports both fork_id (format: "fork:uuid") and task_id (format: "uuid")
    let registry_for_result = Arc::clone(&executor_registry_ref);
    let get_result_tool = GetTaskResultTool::empty()
        .with_callback(move |app_id, task_or_fork_id| {
            let registry = Arc::clone(&registry_for_result);
            async move {
                // Get the registry reference
                let registry_guard = registry.read().await;
                let registry = registry_guard.as_ref()
                    .ok_or_else(|| "Executor registry not initialized".to_string())?;

                // If app_id is empty, use the first registered app
                let app_id = if app_id.is_empty() {
                    let apps = registry.list_applications().await;
                    apps.first()
                        .map(|(id, _)| id.clone())
                        .ok_or_else(|| "No applications registered in executor".to_string())?
                } else {
                    // Parse string to ApplicationId
                    uuid::Uuid::parse_str(&app_id)
                        .map(macaca_proto::ApplicationId)
                        .map_err(|e| format!("Invalid application ID: {}", e))?
                };

                // Get the executor for this app
                let executor = registry.get(&app_id).await
                    .ok_or_else(|| format!("App '{}' not found in registry", app_id))?;

                // Check if this is a fork_id (format: "fork:fork-{uuid}" or "fork:{uuid}")
                if let Some(fork_id_str) = task_or_fork_id.strip_prefix("fork:") {
                    // Handle both "fork-{uuid}" format (from Display) and plain UUID
                    let uuid_str = if let Some(uuid_part) = fork_id_str.strip_prefix("fork-") {
                        uuid_part
                    } else {
                        fork_id_str
                    };
                    let fork_id_uuid = uuid::Uuid::parse_str(uuid_str)
                        .map_err(|e| format!("Invalid fork_id '{}': {}", uuid_str, e))?;
                    let fork_id = macaca_proto::ForkId(fork_id_uuid);

                    let fork_manager = executor.fork_manager();
                    let fork = fork_manager.get_fork(fork_id).await
                        .ok_or_else(|| format!("Fork '{}' not found", fork_id))?;

                    let (status_str, output, error) = match fork.state {
                        macaca_proto::ForkState::Pending => ("pending".to_string(), None, None),
                        macaca_proto::ForkState::Running => ("running".to_string(), None, None),
                        macaca_proto::ForkState::WaitingForHook => ("waiting".to_string(), None, None),
                        macaca_proto::ForkState::Completed => {
                            let output = fork.own_messages.iter()
                                .filter_map(|m| if m.role == macaca_proto::LlmRole::Assistant {
                                    Some(m.content.clone())
                                } else { None })
                                .collect::<Vec<_>>()
                                .join("\n");
                            ("completed".to_string(), Some(output), None)
                        }
                        macaca_proto::ForkState::Failed { ref error } => {
                            ("failed".to_string(), None, Some(error.clone()))
                        }
                        macaca_proto::ForkState::Merged => ("merged".to_string(), None, None),
                        macaca_proto::ForkState::Cancelled => ("cancelled".to_string(), None, None),
                    };

                    return Ok(macaca_tools::orchestration::TaskResultData {
                        status: status_str,
                        output,
                        error,
                    });
                }

                // Otherwise treat as task_id
                let task_id_uuid = uuid::Uuid::parse_str(&task_or_fork_id)
                    .map_err(|e| format!("Invalid task_id: {}", e))?;
                let task_id = macaca_kernel::TaskId(task_id_uuid);

                // Get task status and result
                let status = executor.get_task_status(&task_id).await
                    .ok_or_else(|| format!("Task '{}' not found", task_id))?;

                let (status_str, output, error) = match status {
                    macaca_kernel::TaskStatus::Queued => ("queued".to_string(), None, None),
                    macaca_kernel::TaskStatus::Running => ("running".to_string(), None, None),
                    macaca_kernel::TaskStatus::Completed => {
                        if let Some(result) = executor.get_task_result(task_id).await {
                            ("completed".to_string(), Some(result.output), result.error)
                        } else {
                            ("completed".to_string(), None, None)
                        }
                    }
                    macaca_kernel::TaskStatus::Failed => {
                        if let Some(result) = executor.get_task_result(task_id).await {
                            ("failed".to_string(), Some(result.output), result.error)
                        } else {
                            ("failed".to_string(), None, Some("Task failed".to_string()))
                        }
                    }
                    macaca_kernel::TaskStatus::Cancelled => ("cancelled".to_string(), None, None),
                };

                Ok(macaca_tools::TaskResultData {
                    status: status_str,
                    output,
                    error,
                })
            }
            .boxed()
        });
    all_tools.push(Box::new(get_result_tool));
    info!("GetTaskResult tool added");

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
    let todo_store = Arc::new(macaca_task::TodoStore::new(Arc::clone(&session_store)));
    let event_log = Arc::new(macaca_persist::EventLog::new(Arc::clone(&session_store)));
    info!(path = %session_db_path.display(), "Session store initialized");

    // 9a. Initialize audit logger and alert manager.
    let audit_logger = Arc::new(macaca_kernel::audit::AuditLogger::new(Arc::clone(&session_store)));
    let alert_config = macaca_kernel::alert::AlertConfig::default();
    let alert_manager = Arc::new(macaca_kernel::alert::AlertManager::new(alert_config));
    info!("AuditLogger and AlertManager initialized");

    // Get default model from provider config
    let provider_config = config.llm.providers.get(&config.llm.default_provider);
    let default_model = provider_config
        .and_then(|p| p.default_model.clone())
        .unwrap_or_default();

    // 10. Build shared state.
    let state = Arc::new_cyclic(|weak_state| {
        // Create the real agent runner with the actual weak state
        let runner = Arc::new(WebAgentRunner::new(weak_state.clone()));
        let executor_registry = Arc::new(ApplicationExecutorRegistry::new(Arc::clone(&runner) as Arc<dyn macaca_kernel::AgentRunner>));

        AppState {
            kernel: kernel.clone(),
            runtime: runtime.clone(),
            registry: tokio::sync::RwLock::new(registry),
            catalog: tokio::sync::RwLock::new(catalog),
            llm: llm.clone(),
            default_model,
            app_dirs: tokio::sync::RwLock::new(app_dirs),
            tools,
            sessions: tokio::sync::RwLock::new(HashMap::new()),
            cancel_flags: tokio::sync::RwLock::new(HashMap::new()),
            session_store,
            executor_registry: executor_registry.clone(),
            fork_to_session: tokio::sync::RwLock::new(HashMap::new()),
            goal_to_session: tokio::sync::RwLock::new(HashMap::new()),
            active_sessions: tokio::sync::RwLock::new(HashMap::new()),
            todo_store,
            plan_loop_handles: tokio::sync::RwLock::new(HashMap::new()),
            scheduler_handles: tokio::sync::RwLock::new(HashMap::new()),
            worker_loop_handles: tokio::sync::RwLock::new(HashMap::new()),
            plan_loop_wakers: tokio::sync::RwLock::new(HashMap::new()),
            worker_loop_wakers: tokio::sync::RwLock::new(HashMap::new()),
            audit_logger: audit_logger.clone(),
            alert_manager: alert_manager.clone(),
            app_workspaces: tokio::sync::RwLock::new(HashMap::new()),
            event_log: event_log.clone(),
            delegate_session_id: Arc::clone(&delegate_session_id),
        }
    });

    // 10a. Set the executor registry reference for the delegate tool
    {
        let mut guard = executor_registry_ref.write().await;
        *guard = Some(state.executor_registry.clone());
    }

    // 10b. Register all started apps to the executor registry and create workspaces
    {
        let kernel_ref = Arc::clone(&kernel);
        let registry_ref = state.executor_registry.clone();
        let apps_to_register = started_apps.clone();
        let todo_store_for_recovery = Arc::clone(&state.todo_store);
        let state_ref = Arc::clone(&state);

        tokio::spawn(async move {
            // Get all agents from kernel
            let all_agents = kernel_ref.list_agents().await;
            let app_agents: Vec<AgentInfo> = all_agents
                .iter()
                .map(|m| AgentInfo {
                    id: m.id.0.to_string(),
                    name: m.name.clone(),
                    capabilities: m.capabilities.iter().map(|c| c.name.clone()).collect(),
                    current_load: 0,
                    max_load: 4,
                    available: true,
                })
                .collect();

            // Register each app to executor registry
            for (app_id, app_name) in apps_to_register {
                // Register this app to executor registry
                let _executor = registry_ref.register_application(
                    app_id,
                    app_name,
                    app_agents.clone(),
                ).await;
                tracing::info!(app_id = %app_id.0, "App registered to executor");

                // Recover crashed tasks: rollback InProgress/Assigned → Pending
                todo_store_for_recovery.rollback_in_progress(&app_id).await;

                // Create workspace directories for this app
                let agent_names: Vec<String> = all_agents.iter().map(|a| a.name.clone()).collect();
                let workspace = crate::workspace::AppWorkspace::new(&config.workspace.root_dir, &app_id);
                match workspace.ensure_dirs(&agent_names) {
                    Ok(()) => {
                        tracing::info!(
                            app_id = %app_id.0,
                            workspace = %workspace.root.display(),
                            "Workspace directories created"
                        );
                    }
                    Err(e) => {
                        tracing::error!(app_id = %app_id.0, error = %e, "Failed to create workspace directories");
                    }
                }
                state_ref.app_workspaces.write().await.insert(app_id, workspace);
            }
        });
    }

    // 10c. Note: executor_registry is available in state for task delegation
    // The executor_registry allows agents to delegate tasks to other agents
    // using capability-based routing or direct agent targeting.
    // Route handlers can access it via state.executor_registry
    {
        let _registry = state.executor_registry.clone();
        info!("ApplicationExecutorRegistry initialized and apps registered");
    }

    // 10d. Start hook event consumer for coordinator auto-continue
    {
        let consumer_state = Arc::clone(&state);
        tokio::spawn(async move {
            hook_consumer::start_hook_event_consumer(consumer_state).await;
        });
        info!("Hook event consumer started");
    }

    // 11. Build axum router.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/metrics", get(metrics::metrics_handler))
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
        .route("/api/sessions/stream/{session_id}", get(routes::stream_session_events))
        .route("/api/skills", get(routes::get_skills))
        .route("/api/chat", post(routes::post_chat))
        .route("/api/chat/stop", post(routes::post_chat_stop))
        .route("/api/apps/{app_id}/todos", get(routes::list_todos))
        .route("/api/apps/{app_id}/todos/progress", get(routes::get_todo_progress))
        .route("/api/apps/{app_id}/todos/{agent_name}", get(routes::list_agent_todos))
        .route("/api/apps/{app_id}/goals", get(routes::list_goals).post(routes::create_goal))
        .route("/api/apps/{app_id}/schedules", get(routes::list_schedules).post(routes::create_schedule))
        .route("/api/apps/{app_id}/schedules/{id}", get(routes::get_schedule).delete(routes::delete_schedule))
        .route("/api/apps/{app_id}/schedules/{id}/toggle", axum::routing::put(routes::toggle_schedule))
        .route("/api/sessions/{id}/events", get(routes::get_session_events))
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
