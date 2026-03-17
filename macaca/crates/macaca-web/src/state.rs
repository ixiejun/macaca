//! Shared application state for the web server.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use macaca_app::{AppRegistry, AppRuntime};
use macaca_kernel::{Kernel, ApplicationExecutorRegistry};
use macaca_llm::LlmProvider;
use macaca_persist::RedbStore;
use macaca_proto::{ApplicationId, ForkId, LlmMessage};
use macaca_runtime::agentic_loop::ResumeReason;
use macaca_skill::SkillCatalog;
use macaca_tools::ToolSet;

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
pub struct ActiveSession {
    /// The session ID.
    pub session_id: String,
    /// The application ID.
    pub app_id: ApplicationId,
    /// Pause signal for the agentic loop.
    pub pause_signal: Arc<AtomicBool>,
    /// Channel to send resume reason to the waiting loop.
    pub resume_tx: mpsc::Sender<ResumeReason>,
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
    /// Active sessions with pausable agentic loops.
    /// Used to resume coordinator loops when delegated tasks complete.
    pub active_sessions: RwLock<HashMap<String, ActiveSession>>,
}
