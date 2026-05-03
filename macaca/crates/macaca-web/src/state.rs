//! Shared application state for the web server.

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use axum::response::sse::Event;
use tokio::sync::{mpsc, RwLock};

use macaca_app::{AppRegistry, AppRuntime};
use macaca_driver::{DriverRegistry, DriverRuntime};
use macaca_framework::session::SessionStore as FrameworkSessionStore;
use macaca_kernel::{ApplicationExecutorRegistry, Kernel};
use macaca_llm::{LlmProvider, LlmRouter};
use macaca_persist::{EventLog, PersistBackend};
use macaca_proto::{ApplicationId, ForkId, LlmMessage};
use macaca_skill::SkillCatalog;
use macaca_task::TodoStore;
use macaca_tools::ToolCatalog;

use crate::runtime_resume::RuntimeResumeSignal;
use crate::workspace::AppWorkspace;

/// Mapping from fork_id to session context for hook notifications.
#[derive(Clone, Debug)]
pub struct ForkSessionMapping {
    /// The session ID that owns this fork (coordinator's session).
    pub session_id: String,
    /// The application ID.
    pub app_id: ApplicationId,
    /// The agent name that created this fork (coordinator).
    pub from_agent: String,
}

/// Active session with pausable agentic loop support.
/// Used to resume coordinator loops when delegated tasks complete.
/// Also holds a hot-swappable SSE sender so browser refresh can
/// reconnect to the same coordinator loop.
pub struct ActiveSession {
    /// The session ID.
    pub session_id: String,
    /// The application ID.
    pub app_id: ApplicationId,
    /// Pause signal for the agentic loop.
    pub pause_signal: Arc<AtomicBool>,
    /// Channel to send resume reason to the waiting loop.
    pub resume_tx: mpsc::Sender<RuntimeResumeSignal>,
    /// Hot-swappable SSE event sender. When the browser refreshes,
    /// the stream endpoint replaces this with a new sender so the
    /// coordinator's subsequent events reach the new connection.
    pub sse_tx: Arc<RwLock<mpsc::Sender<Result<Event, Infallible>>>>,
    /// Stop signal for the executor event forwarder task.
    /// Set to `true` when a new POST replaces this session's forwarder.
    pub forwarder_stop: Arc<AtomicBool>,
}

// ---------------------------------------------------------------------------
// Sub-structs for AppState field grouping
// ---------------------------------------------------------------------------

/// Persistence-related state: stores, logs, tracers.
pub struct PersistenceState {
    /// Persistent session store (redb-backed).
    pub session_store: Arc<dyn PersistBackend>,
    /// Shared TodoStore for the Task Board system.
    pub todo_store: Arc<TodoStore>,
    /// Append-only event log (redb-backed, durable before SSE send).
    pub event_log: Arc<EventLog>,
    /// Persistent audit logger (records tool executions, delegation, etc.).
    pub audit_logger: Arc<macaca_kernel::audit::AuditLogger>,
    /// Sparse pipeline checkpoints (`run_trace` events + metrics).
    pub run_tracer: Arc<crate::run_trace::RunTracer>,
}

/// Loop lifecycle state: PlanLoop, WorkerLoop, Scheduler handles and wakers.
pub struct LoopState {
    /// Per-app PlanLoop shutdown handles (for lazy start on first goal).
    pub plan_loop_handles: RwLock<HashMap<ApplicationId, Arc<AtomicBool>>>,
    /// Per-app WorkerLoop shutdown handles (one per worker agent, started alongside PlanLoop).
    pub worker_loop_handles: RwLock<HashMap<ApplicationId, Vec<Arc<AtomicBool>>>>,
    /// Per-app PlanLoop wakers for immediate wakeup on new goals/reviews.
    pub plan_loop_wakers: RwLock<HashMap<ApplicationId, macaca_task::PlanLoopWaker>>,
    /// Per-app WorkerLoop wakers (one per worker agent) for immediate wakeup on new tasks.
    pub worker_loop_wakers: RwLock<HashMap<ApplicationId, Vec<macaca_task::WorkerLoopWaker>>>,
    /// Per-app Scheduler shutdown handles (for lazy start on first schedule).
    pub scheduler_handles: RwLock<HashMap<ApplicationId, Arc<AtomicBool>>>,
}

/// Session-related state: active sessions, conversation caches, mappings.
pub struct SessionState {
    /// Conversation history per app session (session_id -> messages) - in-memory cache.
    pub conversations: RwLock<HashMap<String, Vec<LlmMessage>>>,
    /// Active task cancellation flags (app_id -> cancel flag).
    pub cancel_flags: RwLock<HashMap<String, Arc<AtomicBool>>>,
    /// Active sessions with pausable agentic loops.
    /// Used to resume coordinator loops when delegated tasks complete.
    pub active_sessions: RwLock<HashMap<String, ActiveSession>>,
    /// Mapping from fork_id to session context for hook notifications.
    /// Used to notify coordinators when their delegated tasks complete.
    pub fork_to_session: RwLock<HashMap<ForkId, ForkSessionMapping>>,
    /// Mapping from goal_id to session_id for goal completion notifications.
    /// Used to resume coordinators when their created goals complete.
    pub goal_to_session: Arc<RwLock<HashMap<String, String>>>,
    /// Shared session_id reference for the DelegateTaskTool.
    /// Set to the current session before execute_workflow_steps, cleared after.
    pub delegate_session_id: Arc<tokio::sync::RwLock<Option<String>>>,
    /// Framework-level session store for resumable execution primitives
    /// (e.g. execution_context, plan_notebook).
    pub framework_session_store: Arc<dyn FrameworkSessionStore>,
}

/// Application configuration: directories, models, skills, alerts.
pub struct AppConfig {
    /// Map of app_id -> app directory path.
    pub app_dirs: RwLock<HashMap<ApplicationId, PathBuf>>,
    /// Per-app workspace directory structure (populated on app startup).
    pub app_workspaces: RwLock<HashMap<ApplicationId, AppWorkspace>>,
    /// Default model for LLM requests (e.g. "" for DashScope).
    pub default_model: String,
    /// Skill catalog for progressive disclosure (SKILL.md knowledge skills).
    pub catalog: RwLock<SkillCatalog>,
    /// Alert manager (deduplication + routing to log/webhook channels).
    pub alert_manager: Arc<macaca_kernel::alert::AlertManager>,
}

/// Shared state passed to all route handlers via axum's State extractor.
pub struct AppState {
    /// The kernel managing all agents.
    pub kernel: Arc<Kernel>,
    /// Application runtime managing app lifecycle.
    pub runtime: AppRuntime,
    /// Application registry for discovering apps.
    pub registry: RwLock<AppRegistry>,
    /// The LLM provider (DashScope by default).
    pub llm: Arc<dyn LlmProvider>,
    /// Shared router/resolver used by framework-based agents.
    pub llm_router: Arc<LlmRouter>,
    /// Composite toolset: built-in tools + executable skill tools + claude code tools.
    pub tools: Arc<dyn ToolCatalog>,
    /// Application executor registry for isolated multi-agent execution.
    pub executor_registry: Arc<ApplicationExecutorRegistry>,
    /// Agent OS level MCP runtime and registry.
    pub mcp_runtime: Arc<macaca_runtime_host::McpRuntimeFacade>,
    /// Driver registry for managing loaded software drivers.
    pub driver_registry: Arc<DriverRegistry>,
    /// Driver runtime facade for lifecycle, inventory, and tool collection.
    pub driver_runtime: Arc<DriverRuntime>,
    /// Path to the drivers directory (for reload).
    pub drivers_dir: String,
    /// Persistence: session store, todo store, event log, audit logger, run tracer.
    pub persist: PersistenceState,
    /// Loop lifecycle: PlanLoop, WorkerLoop, Scheduler handles and wakers.
    pub loops: LoopState,
    /// Session state: active sessions, conversation caches, mappings.
    pub sessions: SessionState,
    /// Application configuration: directories, models, skills, alerts.
    pub config: AppConfig,
}
