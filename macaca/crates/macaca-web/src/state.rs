//! Shared application state for the web server.

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use axum::response::sse::Event;
use tokio::sync::{mpsc, RwLock};

use macaca_app::{AppRegistry, AppRuntime};
use macaca_task::TodoStore;
use macaca_kernel::{Kernel, ApplicationExecutorRegistry};
use macaca_llm::LlmProvider;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{ApplicationId, ForkId, LlmMessage};
use macaca_runtime::agentic_loop::ResumeReason;
use macaca_skill::SkillCatalog;
use macaca_tools::ToolSet;

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
    pub resume_tx: mpsc::Sender<ResumeReason>,
    /// Hot-swappable SSE event sender. When the browser refreshes,
    /// the stream endpoint replaces this with a new sender so the
    /// coordinator's subsequent events reach the new connection.
    pub sse_tx: Arc<RwLock<mpsc::Sender<Result<Event, Infallible>>>>,
}

/// Shared state passed to all route handlers via axum's State extractor.
pub struct AppState {
    /// The kernel managing all agents.
    pub kernel: Arc<Kernel>,
    /// Application runtime managing app lifecycle.
    pub runtime: AppRuntime,
    /// Application registry for discovering apps.
    pub registry: RwLock<AppRegistry>,
    /// Skill catalog for progressive disclosure (SKILL.md knowledge skills).
    pub catalog: RwLock<SkillCatalog>,
    /// The LLM provider (DashScope by default).
    pub llm: Arc<dyn LlmProvider>,
    /// Default model for LLM requests (e.g. "" for DashScope)
    pub default_model: String,
    /// Map of app_id -> app directory path.
    pub app_dirs: RwLock<HashMap<ApplicationId, PathBuf>>,
    /// Composite toolset: built-in tools + executable skill tools + claude code tools.
    pub tools: Box<dyn ToolSet>,
    /// Conversation history per app session (app_id -> messages) - in-memory cache.
    pub sessions: RwLock<HashMap<String, Vec<LlmMessage>>>,
    /// Active task cancellation flags (app_id -> cancel flag).
    pub cancel_flags: RwLock<HashMap<String, Arc<AtomicBool>>>,
    /// Persistent session store (redb-backed).
    pub session_store: Arc<RedbStore>,
    /// Application executor registry for isolated multi-agent execution.
    pub executor_registry: Arc<ApplicationExecutorRegistry>,
    /// Mapping from fork_id to session context for hook notifications.
    /// Used to notify coordinators when their delegated tasks complete.
    pub fork_to_session: RwLock<HashMap<ForkId, ForkSessionMapping>>,
    /// Mapping from goal_id to session_id for goal completion notifications.
    /// Used to resume coordinators when their created goals complete.
    pub goal_to_session: RwLock<HashMap<String, String>>,
    /// Active sessions with pausable agentic loops.
    /// Used to resume coordinator loops when delegated tasks complete.
    pub active_sessions: RwLock<HashMap<String, ActiveSession>>,
    /// Shared TodoStore for the Task Board system.
    pub todo_store: Arc<TodoStore>,
    /// Per-app PlanLoop shutdown handles (for lazy start on first goal).
    pub plan_loop_handles: RwLock<HashMap<ApplicationId, Arc<std::sync::atomic::AtomicBool>>>,
    /// Per-app Scheduler shutdown handles (for lazy start on first schedule).
    pub scheduler_handles: RwLock<HashMap<ApplicationId, Arc<std::sync::atomic::AtomicBool>>>,
    /// Per-app WorkerLoop shutdown handles (one per worker agent, started alongside PlanLoop).
    pub worker_loop_handles: RwLock<HashMap<ApplicationId, Vec<Arc<std::sync::atomic::AtomicBool>>>>,
    /// Per-app PlanLoop wakers for immediate wakeup on new goals/reviews.
    pub plan_loop_wakers: RwLock<HashMap<ApplicationId, macaca_task::PlanLoopWaker>>,
    /// Per-app WorkerLoop wakers (one per worker agent) for immediate wakeup on new tasks.
    pub worker_loop_wakers: RwLock<HashMap<ApplicationId, Vec<macaca_task::WorkerLoopWaker>>>,
    /// Persistent audit logger (records tool executions, delegation, etc.).
    pub audit_logger: Arc<macaca_kernel::audit::AuditLogger>,
    /// Alert manager (deduplication + routing to log/webhook channels).
    pub alert_manager: Arc<macaca_kernel::alert::AlertManager>,
    /// Per-app workspace directory structure (populated on app startup).
    pub app_workspaces: RwLock<HashMap<ApplicationId, AppWorkspace>>,
    /// Append-only event log (redb-backed, durable before SSE send).
    pub event_log: Arc<EventLog>,
    /// Shared session_id reference for the DelegateTaskTool.
    /// Set to the current session before execute_workflow_steps, cleared after.
    pub delegate_session_id: Arc<tokio::sync::RwLock<Option<String>>>,
}
