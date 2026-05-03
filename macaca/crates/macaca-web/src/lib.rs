//! `macaca-web` — Simple web UI for Macaca OS.
//!
//! Provides an HTTP server with a single-page web interface for interacting
//! with Macaca OS applications. Uses axum for the HTTP layer.

pub mod agent_runner;
pub mod chat_orchestrator;
pub mod event_persistence;
pub mod framework_runner;
pub mod framework_toolkit;
pub mod hook_consumer;
pub mod loop_manager;
pub mod mcp_runtime;
pub mod metrics;
pub mod proto_event_visitors;
pub mod routes;
pub mod run_trace;
pub mod runtime_resume;
pub mod session;
pub mod skill_mcp;
pub mod sse;
pub mod state;
pub mod workspace;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

use futures::FutureExt;
use macaca_app::{AppLoader, AppRegistry, AppRuntime};
use macaca_framework::session::{
    InMemorySessionStore as FrameworkInMemorySessionStore, SessionStore as FrameworkSessionStore,
};
use macaca_kernel::{AgentInfo, ApplicationExecutorRegistry, Kernel};
use macaca_llm::{LlmProvider, LlmRouter};
use macaca_persist::RedbStore;
use macaca_proto::config::{KernelConfig, MacacaConfig};
use macaca_proto::{ApplicationId, LlmMessage, MacacaResult};
use macaca_sdk::AgentPersona;
use macaca_skill::{ExecutableSkillToolSet, SkillCatalog};
use macaca_tools::{
    CompositeToolSet, DefaultToolSet, DelegateTaskTool, GetTaskResultTool, ListAgentsTool, Tool,
    ToolCatalog,
};

use crate::agent_runner::WebAgentRunner;
use crate::state::{AppConfig, AppState, LoopState, PersistenceState, SessionState};

/// Start the Macaca OS web server.
pub async fn start_server(port: u16) -> MacacaResult<()> {
    // 1. Load configuration from config/default.toml
    let config = MacacaConfig::load_default();
    info!(default_provider = %config.llm.default_provider, "Configuration loaded");

    // 1b. Publish [mcp.env] entries into the current process environment so
    //     every stdio MCP child process (which inherits parent env by default)
    //     automatically receives secrets such as FIGMA_API_KEY.
    let mcp_env_outcomes = macaca_runtime_host::apply_mcp_env(&config.mcp.env);
    if !mcp_env_outcomes.is_empty() {
        info!(
            entries = mcp_env_outcomes.len(),
            "Applied [mcp.env] to process environment for MCP child inheritance"
        );
    }

    // 2. Create LLM router/provider registry from configuration.
    let llm_router = Arc::new(LlmRouter::from_config(&config.llm)?);
    let llm: Arc<dyn LlmProvider> = llm_router.clone();

    info!(provider = llm.name(), "LLM provider initialized");

    // 3. Create kernel.
    let kernel_config = KernelConfig {
        max_agents: 64,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 60000,
    };
    let kernel = Arc::new(Kernel::new(
        &kernel_config,
        Arc::clone(&llm),
        Box::new(DefaultToolSet::new()),
    ));

    // 4. Initialize app registry and discover apps.
    let mut registry = AppRegistry::new();
    let discovered = registry.discover_apps()?;
    info!(
        count = discovered.len(),
        "Apps discovered from standard directories"
    );

    // 5. Start the runtime and load ALL discovered apps.
    let runtime = AppRuntime::new();
    let mut app_dirs = HashMap::new();
    let mut skills_dirs = Vec::new();
    let mut started_apps: Vec<(macaca_proto::ApplicationId, String, Vec<String>)> = Vec::new();

    // Auto-start all discovered apps
    for app in &discovered {
        let manifest_path = app.manifest_path.clone();
        if manifest_path.exists() {
            match runtime.start_app_from_file(&manifest_path, &kernel).await {
                Ok(app_id) => {
                    let agent_count = kernel.agent_count().await;
                    app_dirs.insert(app_id, app.path.clone());
                    skills_dirs.push(app.path.join("skills"));
                    let app_agent_names =
                        AppLoader::resolve_agent_configs(&app.manifest, &app.path)
                            .map(|configs| {
                                configs
                                    .into_iter()
                                    .map(|config| config.name)
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_else(|error| {
                                tracing::warn!(
                                    app_name = %app.name,
                                    error = %error,
                                    "Failed to resolve app agent names for executor registration"
                                );
                                Vec::new()
                            });
                    started_apps.push((app_id.clone(), app.name.clone(), app_agent_names));
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
            let mut skill_tools = ExecutableSkillToolSet::new();
            match skill_tools.load_from_directory(dir).await {
                Ok(n) => {
                    let skill_tools = skill_tools.into_tools();
                    info!(count = n, "Executable skill tools loaded");
                    all_tools.extend(skill_tools);
                }
                Err(e) => tracing::warn!("Failed to load executable skills: {e}"),
            }
        }
    }

    // Load external driver plugins from configured directory.
    // Driver tools are NOT added to the static CompositeToolSet; instead they
    // are aggregated dynamically from `DriverRegistry` in `build_toolkit` so
    // that `/api/drivers/reload` picks up new driver tools at runtime.
    let drivers_dir =
        std::env::var("MACACA_DRIVERS_DIR").unwrap_or_else(|_| config.drivers.directory.clone());
    let driver_registry = Arc::new(macaca_driver::DriverRegistry::new());
    let driver_runtime = Arc::new(macaca_driver::DriverRuntime::new(
        drivers_dir.clone(),
        Arc::clone(&driver_registry),
    ));
    if config.drivers.auto_load {
        let report = driver_runtime.load_all().await;
        for entry in &report.entries {
            match entry.status {
                macaca_driver::DriverLoadStatus::Loaded => {
                    info!(
                        name = %entry.name,
                        tools = entry.tool_count.unwrap_or_default(),
                        "External driver loaded"
                    );
                }
                macaca_driver::DriverLoadStatus::Failed => {
                    error!(
                        name = %entry.name,
                        error = %entry.error.as_deref().unwrap_or("unknown error"),
                        "Failed to load external driver"
                    );
                }
            }
        }
    }

    // 8. Initialize orchestration tools.
    // We need to create a shared reference for the executor registry that can be
    // populated after state creation. This allows delegate_task tool to access the registry.
    let executor_registry_ref: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>> =
        Arc::new(RwLock::new(None));

    // Create dynamic ListAgentsTool that fetches from kernel
    let kernel_for_callback = Arc::clone(&kernel);
    let list_agents_tool = ListAgentsTool::new().with_agents_callback(move || {
        let kernel = Arc::clone(&kernel_for_callback);
        async move {
            let agents = kernel.list_agents().await;
            agents
                .into_iter()
                .map(|agent| {
                    let capabilities: Vec<String> =
                        agent.capabilities.into_iter().map(|cap| cap.name).collect();
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
        .with_callback(
            move |app_id, to_agent, prompt, priority, parallel, session_id| {
                let registry = Arc::clone(&registry_for_delegate);
                async move {
                    // Get the registry reference
                    let registry_guard = registry.read().await;
                    let registry = registry_guard
                        .as_ref()
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
                    let executor = registry
                        .get(&app_id)
                        .await
                        .ok_or_else(|| format!("App '{}' not found in registry", app_id))?;

                    // Get the ForkManager from the executor
                    let fork_manager = executor.fork_manager();

                    // Create acceptance criteria
                    let acceptance_criteria = macaca_proto::AcceptanceCriteria {
                        description: format!(
                            "Task delegated to {}: {}",
                            to_agent,
                            prompt.chars().take(100).collect::<String>()
                        ),
                        required_artifacts: vec![],
                        auto_accept: false,
                    };

                    // Create a Fork for this delegation
                    let fork_id = fork_manager
                        .create_fork(
                            None, // parent_fork_id - None for now, will be set by caller context
                            app_id.clone(),
                            to_agent.clone(),
                            prompt.clone(),
                            vec![],        // inherited_messages - will be populated by caller
                            String::new(), // system_prompt - will be set by caller
                            acceptance_criteria,
                        )
                        .await
                        .map_err(|e| format!("Fork creation failed: {}", e))?;

                    // Start the fork (transition from Pending to Running)
                    fork_manager
                        .start_fork(fork_id)
                        .await
                        .map_err(|e| format!("Fork start failed: {}", e))?;

                    // Also delegate the actual task to the executor
                    let task_context = session_id.map(|sid| macaca_kernel::TaskContext {
                        session_id: Some(sid),
                        artifacts: vec![],
                        env: std::collections::HashMap::new(),
                    });
                    let task_id = executor
                        .delegate_task(
                            "coordinator", // from_agent
                            &to_agent,
                            prompt,
                            priority,
                            parallel,
                            task_context,
                        )
                        .await
                        .map_err(|e| format!("Delegation failed: {}", e))?;

                    // Suspend the fork waiting for the task to complete
                    fork_manager
                        .suspend_fork(fork_id, macaca_proto::TaskId(task_id.0))
                        .await
                        .map_err(|e| format!("Fork suspend failed: {}", e))?;

                    // Return the fork_id - caller can use get_fork_result to check status
                    Ok(format!("fork:{}", fork_id))
                }
                .boxed()
            },
        );
    all_tools.push(Box::new(delegate_tool));
    info!("DelegateTask tool added");

    // Create GetTaskResultTool with callback to executor registry
    // Supports both fork_id (format: "fork:uuid") and task_id (format: "uuid")
    let registry_for_result = Arc::clone(&executor_registry_ref);
    let get_result_tool =
        GetTaskResultTool::empty().with_callback(move |app_id, task_or_fork_id| {
            let registry = Arc::clone(&registry_for_result);
            async move {
                // Get the registry reference
                let registry_guard = registry.read().await;
                let registry = registry_guard
                    .as_ref()
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
                let executor = registry
                    .get(&app_id)
                    .await
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
                    let fork = fork_manager
                        .get_fork(fork_id)
                        .await
                        .ok_or_else(|| format!("Fork '{}' not found", fork_id))?;

                    let (status_str, output, error) = match fork.state {
                        macaca_proto::ForkState::Pending => ("pending".to_string(), None, None),
                        macaca_proto::ForkState::Running => ("running".to_string(), None, None),
                        macaca_proto::ForkState::WaitingForHook => {
                            ("waiting".to_string(), None, None)
                        }
                        macaca_proto::ForkState::Completed => {
                            let output = fork
                                .own_messages
                                .iter()
                                .filter_map(|m| {
                                    if m.role == macaca_proto::LlmRole::Assistant {
                                        Some(m.content.clone())
                                    } else {
                                        None
                                    }
                                })
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
                let status = executor
                    .get_task_status(&task_id)
                    .await
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

    let tools: Arc<dyn ToolCatalog> = Arc::new(CompositeToolSet::new(all_tools));

    // 9. Initialize persistent session store.
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("macaca");
    std::fs::create_dir_all(&data_dir).ok();
    let session_db_path = data_dir.join("sessions.db");
    let session_store_impl = Arc::new(RedbStore::open(&session_db_path)?);
    let session_store_shared: Arc<dyn macaca_persist::PersistBackend> = session_store_impl.clone();
    let todo_store = Arc::new(macaca_task::TodoStore::new(Arc::clone(
        &session_store_shared,
    )));
    let event_log = Arc::new(macaca_persist::EventLog::new(Arc::clone(
        &session_store_impl,
    )));
    let run_tracer = Arc::new(crate::run_trace::RunTracer::new(Arc::clone(&event_log)));
    info!(path = %session_db_path.display(), "Session store initialized");

    // 9a. Initialize audit logger and alert manager.
    let audit_logger = Arc::new(macaca_kernel::audit::AuditLogger::new(Arc::clone(
        &session_store_shared,
    )));
    let session_store = session_store_shared;
    let alert_config = macaca_kernel::alert::AlertConfig::default();
    let alert_manager = Arc::new(macaca_kernel::alert::AlertManager::new(alert_config));
    info!("AuditLogger and AlertManager initialized");

    let default_model = llm_router.default_model_reference();
    let framework_session_store: Arc<dyn FrameworkSessionStore> =
        Arc::new(FrameworkInMemorySessionStore::new());
    let mcp_runtime = Arc::new(mcp_runtime::McpRuntimeManager::load_default().await);

    // 10. Build shared state.
    let state = Arc::new_cyclic(|weak_state| {
        // Create the real agent runner with the actual weak state
        let runner = Arc::new(WebAgentRunner::new(weak_state.clone()));
        let executor_registry = Arc::new(ApplicationExecutorRegistry::new(
            Arc::clone(&runner) as Arc<dyn macaca_kernel::AgentRunner>
        ));

        AppState {
            kernel: kernel.clone(),
            runtime: runtime.clone(),
            registry: tokio::sync::RwLock::new(registry),
            llm: llm.clone(),
            llm_router: llm_router.clone(),
            tools,
            executor_registry: executor_registry.clone(),
            mcp_runtime: Arc::clone(&mcp_runtime),
            driver_registry: Arc::clone(&driver_registry),
            driver_runtime: Arc::clone(&driver_runtime),
            drivers_dir: drivers_dir.clone(),
            persist: PersistenceState {
                session_store,
                todo_store,
                event_log: event_log.clone(),
                audit_logger: audit_logger.clone(),
                run_tracer: Arc::clone(&run_tracer),
            },
            loops: LoopState {
                plan_loop_handles: tokio::sync::RwLock::new(HashMap::new()),
                worker_loop_handles: tokio::sync::RwLock::new(HashMap::new()),
                plan_loop_wakers: tokio::sync::RwLock::new(HashMap::new()),
                worker_loop_wakers: tokio::sync::RwLock::new(HashMap::new()),
                scheduler_handles: tokio::sync::RwLock::new(HashMap::new()),
            },
            sessions: SessionState {
                conversations: tokio::sync::RwLock::new(HashMap::new()),
                cancel_flags: tokio::sync::RwLock::new(HashMap::new()),
                active_sessions: tokio::sync::RwLock::new(HashMap::new()),
                fork_to_session: tokio::sync::RwLock::new(HashMap::new()),
                goal_to_session: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
                delegate_session_id: Arc::clone(&delegate_session_id),
                framework_session_store: Arc::clone(&framework_session_store),
            },
            config: AppConfig {
                app_dirs: tokio::sync::RwLock::new(app_dirs),
                app_workspaces: tokio::sync::RwLock::new(HashMap::new()),
                default_model,
                catalog: tokio::sync::RwLock::new(catalog),
                alert_manager: alert_manager.clone(),
            },
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
        let todo_store_for_recovery = Arc::clone(&state.persist.todo_store);
        let state_ref = Arc::clone(&state);

        tokio::spawn(async move {
            // Get all agents from kernel, then register each executor with
            // only the agents declared by that application.
            let all_agents = kernel_ref.list_agents().await;
            let agents_by_name: HashMap<_, _> =
                all_agents.iter().map(|m| (m.name.clone(), m)).collect();

            // Register each app to executor registry
            for (app_id, app_name, app_agent_names) in apps_to_register {
                let mut app_agents: Vec<AgentInfo> = app_agent_names
                    .iter()
                    .filter_map(|name| agents_by_name.get(name.as_str()).copied())
                    .map(|m| AgentInfo {
                        id: m.id.0.to_string(),
                        name: m.name.clone(),
                        capabilities: m.capabilities.iter().map(|c| c.name.clone()).collect(),
                        current_load: 0,
                        max_load: 4,
                        available: true,
                    })
                    .collect();
                if app_agents.is_empty() {
                    tracing::warn!(
                        app_id = %app_id.0,
                        app_name = %app_name,
                        "No app-scoped agents resolved; falling back to all registered agents"
                    );
                    app_agents = all_agents
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
                }
                let workspace_agent_names: Vec<String> =
                    app_agents.iter().map(|agent| agent.name.clone()).collect();

                // Register this app to executor registry
                let _executor = registry_ref
                    .register_application(app_id, app_name, app_agents.clone())
                    .await;
                tracing::info!(app_id = %app_id.0, "App registered to executor");

                // Recover crashed tasks: rollback InProgress/Assigned → Pending
                todo_store_for_recovery.rollback_in_progress(&app_id).await;

                // Create workspace directories for this app
                let workspace =
                    crate::workspace::AppWorkspace::new(&config.workspace.root_dir, &app_id);
                match workspace.ensure_dirs(&workspace_agent_names) {
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
                state_ref
                    .config
                    .app_workspaces
                    .write()
                    .await
                    .insert(app_id, workspace);

                // Auto-start PlanLoop and WorkerLoops for this app so pending
                // tasks (e.g., PendingReview from before restart) are processed.
                crate::loop_manager::ensure_plan_and_worker_loops(&state_ref, &app_id, None).await;
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
        .route("/api/apps/{id}/skills", get(routes::get_app_skills))
        .route(
            "/api/apps/{id}/agents/stream",
            get(routes::stream_agent_status),
        )
        .route("/api/apps/{id}/sessions", get(session::list_app_sessions))
        .route("/api/apps/reload", post(routes::reload_apps))
        .route("/api/mcp", get(routes::get_mcp_status))
        .route("/api/sessions", get(session::list_sessions))
        .route("/api/sessions/{app_id}", get(session::get_session))
        .route(
            "/api/sessions/detail/{session_id}",
            get(session::get_session_by_id),
        )
        .route(
            "/api/sessions/stream/{session_id}",
            get(session::stream_session_events),
        )
        .route("/api/skills", get(routes::get_skills))
        .route("/api/chat/v2", post(chat_orchestrator::post_chat_v2))
        .route("/api/chat/stop", post(chat_orchestrator::post_chat_stop))
        .route("/api/apps/{app_id}/todos", get(routes::list_todos))
        .route(
            "/api/apps/{app_id}/todos/progress",
            get(routes::get_todo_progress),
        )
        .route(
            "/api/apps/{app_id}/todos/claim-diagnostics",
            get(routes::get_todo_claim_diagnostics),
        )
        .route(
            "/api/apps/{app_id}/todos/{agent_name}",
            get(routes::list_agent_todos),
        )
        .route(
            "/api/apps/{app_id}/goals",
            get(routes::list_goals).post(loop_manager::create_goal),
        )
        .route(
            "/api/apps/{app_id}/schedules",
            get(routes::list_schedules).post(routes::create_schedule),
        )
        .route(
            "/api/apps/{app_id}/schedules/{id}",
            get(routes::get_schedule).delete(routes::delete_schedule),
        )
        .route(
            "/api/apps/{app_id}/schedules/{id}/toggle",
            axum::routing::put(routes::toggle_schedule),
        )
        .route("/api/sessions/{id}/events", get(routes::get_session_events))
        .route(
            "/api/sessions/{id}/run-trace",
            get(routes::get_session_run_trace),
        )
        .route("/api/drivers", get(routes::get_drivers))
        .route("/api/drivers/reload", post(routes::reload_drivers))
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
