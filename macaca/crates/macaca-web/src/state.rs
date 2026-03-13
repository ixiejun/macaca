//! Shared application state for the web server.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::RwLock;

use macaca_app::{AppRegistry, AppRuntime};
use macaca_kernel::{Kernel, ApplicationExecutorRegistry};
use macaca_llm::LlmProvider;
use macaca_persist::RedbStore;
use macaca_proto::{ApplicationId, LlmMessage};
use macaca_skill::SkillCatalog;
use macaca_tools::ToolSet;

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
}
