//! Shared application state for the web server.

use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::response::sse::Event;
use serde::Serialize;
use tokio::sync::{mpsc, RwLock};

use macaca_app::{AppRegistry, AppRuntime};
use macaca_context::{
    ContextAdapterSafetyPolicy, ContextEngineInfo, ContextEngineRegistry, ContextFallbackPolicy,
    ContextProviderRegistry, ProviderHealthLedger,
};
use macaca_driver::{DriverRegistry, DriverRuntime};
use macaca_framework::session::SessionStore as FrameworkSessionStore;
use macaca_kernel::{ApplicationExecutorRegistry, Kernel};
use macaca_llm::{LlmProvider, LlmRouter};
use macaca_persist::{EventLog, PersistBackend};
use macaca_proto::{config::ContextConfig, ApplicationId, ForkId, LlmMessage};
use macaca_runtime_host::ServiceRuntime;
use macaca_sdk::{
    SystemApplicationClient, SystemContextClient, SystemDriverClient, SystemEntitlementClient,
    SystemEvmClient, SystemHeartbeatClient, SystemLlmClient, SystemMcpClient, SystemMemoryClient,
    SystemPaymentClient, SystemPluginCapabilityClient, SystemPluginControlClient,
    SystemPluginHookClient, SystemScheduledAgentTaskClient, SystemSchedulerClient,
    SystemSkillClient, SystemStoreClient, SystemToolClient, SystemWeb3Client,
};
use macaca_skill::SkillCatalog;
use macaca_task::TodoStore;
use macaca_tools::ToolCatalog;

use crate::runtime_resume::RuntimeResumeSignal;
use crate::shell::WebSystemFacadeBundle;
use crate::workspace::AppWorkspace;

/// Operator-visible installation record for one external adapter engine.
///
/// The web layer keeps this inventory separate from `ContextEngineRegistry` so
/// `/api/context/provider-runtime` can report installation metadata even before
/// an adapter participates in a request. Today the only concrete install path is
/// a local registry overlay, but the shape is transport-neutral so future
/// process/RPC/WASM loaders can publish into the same surface.
#[derive(Clone, Debug, Serialize)]
pub struct ExternalAdapterRuntimeInstallation {
    /// Stable engine metadata reported by the installed adapter wrapper.
    pub engine: ContextEngineInfo,
    /// Transport family used to reach the adapter implementation.
    pub transport: String,
    /// Which runtime mechanism installed this adapter into the current process.
    pub installation_source: String,
    /// Coarse-grained lifecycle state exported to operator diagnostics.
    pub runtime_state: String,
    /// Default safety guardrails currently associated with this adapter.
    pub default_safety_policy: ContextAdapterSafetyPolicy,
    /// Default fallback policy currently associated with this adapter.
    pub default_fallback_policy: ContextFallbackPolicy,
    /// Current circuit-breaker state. Phase 9 still exposes this as metadata only.
    pub circuit_breaker_runtime_state: String,
    /// Last time the inventory row was synchronized from runtime state.
    pub last_sync_epoch_ms: u128,
}

/// Dedicated inventory for installed external adapter engines.
///
/// This is intentionally a separate registry from `ContextEngineRegistry`:
/// `ContextEngineRegistry` answers "what engines can be selected right now?",
/// while this structure answers "which external adapters are installed, how did
/// they get here, and what runtime defaults/status are attached to them?".
#[derive(Default)]
pub struct ExternalAdapterRuntimeRegistry {
    rows: RwLock<HashMap<String, ExternalAdapterRuntimeInstallation>>,
}

impl ExternalAdapterRuntimeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create the runtime inventory with a precomputed installation snapshot.
    ///
    /// Startup wiring uses this when config-driven adapter installation succeeds before the full
    /// `AppState` exists. The snapshot remains mutable afterwards via `sync_registry_overlay_engines`.
    pub fn with_installations(
        self,
        installations: Vec<ExternalAdapterRuntimeInstallation>,
    ) -> Self {
        let rows = installations
            .into_iter()
            .map(|installation| (installation.engine.id.clone(), installation))
            .collect();
        Self {
            rows: RwLock::new(rows),
        }
    }

    fn now_ms() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0)
    }

    /// Synchronize the explicit inventory with the subset of engine registry rows
    /// that currently represent non-builtin overlay engines.
    ///
    /// Until dedicated install APIs land, registry overlays are the only concrete
    /// installation mechanism. This method keeps the operator surface explicit and
    /// prunes stale rows if an overlay engine disappears from the runtime.
    pub async fn sync_registry_overlay_engines(
        &self,
        builtin_engine_ids: &HashSet<String>,
        registry_engine_rows: &[ContextEngineInfo],
    ) {
        let now_ms = Self::now_ms();
        let mut rows = self.rows.write().await;
        let active_overlay_ids: HashSet<_> = registry_engine_rows
            .iter()
            .filter(|engine| !builtin_engine_ids.contains(&engine.id))
            .map(|engine| engine.id.clone())
            .collect();

        rows.retain(|engine_id, _| active_overlay_ids.contains(engine_id));

        for engine in registry_engine_rows
            .iter()
            .filter(|engine| !builtin_engine_ids.contains(&engine.id))
        {
            let existing = rows.get(&engine.id).cloned();
            rows.insert(
                engine.id.clone(),
                ExternalAdapterRuntimeInstallation {
                    engine: engine.clone(),
                    transport: existing
                        .as_ref()
                        .map(|row| row.transport.clone())
                        .unwrap_or_else(|| "registry_overlay".to_string()),
                    installation_source: existing
                        .as_ref()
                        .map(|row| row.installation_source.clone())
                        .unwrap_or_else(|| "context_engine_registry_overlay".to_string()),
                    runtime_state: existing
                        .as_ref()
                        .map(|row| row.runtime_state.clone())
                        .unwrap_or_else(|| "installed".to_string()),
                    default_safety_policy: existing
                        .as_ref()
                        .map(|row| row.default_safety_policy.clone())
                        .unwrap_or_default(),
                    default_fallback_policy: existing
                        .as_ref()
                        .map(|row| row.default_fallback_policy.clone())
                        .unwrap_or_default(),
                    circuit_breaker_runtime_state: existing
                        .as_ref()
                        .map(|row| row.circuit_breaker_runtime_state.clone())
                        .unwrap_or_else(|| "not_implemented".to_string()),
                    last_sync_epoch_ms: now_ms,
                },
            );
        }
    }

    /// Return a stable, operator-facing snapshot sorted by `engine.id`.
    pub async fn snapshot(&self) -> Vec<ExternalAdapterRuntimeInstallation> {
        let mut rows: Vec<_> = self.rows.read().await.values().cloned().collect();
        rows.sort_by(|left, right| left.engine.id.cmp(&right.engine.id));
        rows
    }
}

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
///
/// Deprecated compatibility boundary: new execution-control ownership should
/// flow through `service.execution_control` and the Web host adapter should only
/// keep the non-serializable local channel endpoint needed by framework
/// middleware. The fields below remain public because older hook and plan-loop
/// adapters still bridge their in-memory completion events into the waiting
/// runtime loop, but new call paths must not add direct channel ownership.
///
/// The session also holds a hot-swappable SSE sender so browser refresh can
/// reconnect to the same coordinator loop.
pub struct ActiveSession {
    /// The session ID.
    pub session_id: String,
    /// The application ID.
    pub app_id: ApplicationId,
    /// Deprecated local pause flag read by the framework middleware.
    pub pause_signal: Arc<AtomicBool>,
    /// Deprecated local resume channel used only by approved Web adapters.
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
    /// Request-scoped LLM route hints keyed by session id.
    ///
    /// This Web-local map is an Adapter cache, not a routing authority. The
    /// selected string is copied from the generic execution request so
    /// framework construction can pass it into `service.llm`/`LlmRouter`
    /// precedence without adding app-specific model logic to agents.
    pub llm_route_hints: RwLock<HashMap<String, String>>,
    /// Framework-level session store for resumable execution primitives
    /// (e.g. execution_context, plan_notebook).
    pub framework_session_store: Arc<dyn FrameworkSessionStore>,
}

/// Application configuration: directories, models, skills, alerts.
pub struct AppConfig {
    /// Map of app_id -> app directory path.
    pub app_dirs: RwLock<HashMap<ApplicationId, PathBuf>>,
    /// Per-app workspace directory structure (populated on app startup).
    pub app_workspaces: Arc<RwLock<HashMap<ApplicationId, AppWorkspace>>>,
    /// Default model for LLM requests (e.g. "" for DashScope).
    pub default_model: String,
    /// Runtime context-engine configuration.
    pub context: ContextConfig,
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
    ///
    /// Deprecated compatibility anchor: new Web call paths should use
    /// [`Self::application_client`] so application lifecycle behavior crosses
    /// the same traceable service boundary as other Route C capabilities.
    #[deprecated(note = "Use AppState::application_client for new application lifecycle paths")]
    pub runtime: Arc<AppRuntime>,
    /// Application registry for discovering apps.
    ///
    /// Deprecated compatibility anchor retained so older route helpers and
    /// tests can still inspect manifests during the gradual migration.  The
    /// service provider receives the same shared registry handle, which keeps
    /// direct fallback reads consistent with service-backed snapshots.
    #[deprecated(note = "Use AppState::application_client for new application discovery paths")]
    pub registry: Arc<RwLock<AppRegistry>>,
    /// The serviceized Application client used by new Route C call paths.
    pub application_client: Arc<dyn SystemApplicationClient>,
    /// The serviceized LLM client used by new Route C call paths.
    pub llm_client: Arc<dyn SystemLlmClient>,
    /// The serviceized Memory client used by new Route C call paths.
    pub memory_client: Arc<dyn SystemMemoryClient>,
    /// The serviceized Context client used by new Route C call paths.
    pub context_client: Arc<dyn SystemContextClient>,
    /// The serviceized Driver client used by new Route C call paths.
    pub driver_client: Arc<dyn SystemDriverClient>,
    /// The serviceized Skill client used by new Route C call paths.
    pub skill_client: Arc<dyn SystemSkillClient>,
    /// The serviceized MCP client used by new Route C call paths.
    pub mcp_client: Arc<dyn SystemMcpClient>,
    /// The serviceized Tool Capability Plane client used for production
    /// framework tool invocation.  Web still adapts descriptors into framework
    /// tools, but invocation routing, policy, result budget, and audit now
    /// cross `service.tool`.
    pub tool_client: Arc<dyn SystemToolClient>,
    /// The serviceized Store client used by package-manager shell paths.
    pub store_client: Arc<dyn SystemStoreClient>,
    /// The serviceized Entitlement client used by package authorization/audit paths.
    pub entitlement_client: Arc<dyn SystemEntitlementClient>,
    /// The serviceized Payment client used by A2A payment and settlement paths.
    pub payment_client: Arc<dyn SystemPaymentClient>,
    /// The serviceized Scheduler client used by application-scoped autonomy routes.
    ///
    /// Web stores only the SDK facade. Provider construction, lease state, and
    /// daemon lifecycle stay in `macaca-runtime-host`, which preserves the
    /// microkernel/serviceization boundary while letting applications request
    /// recurring work through declared service capability paths.
    pub scheduler_client: Arc<dyn SystemSchedulerClient>,
    /// The serviceized Scheduled Agent Task client used by manual task setup
    /// routes and future entry-agent tool adapters.
    ///
    /// Web keeps this as a focused SDK facade. Prompt payload ownership,
    /// Scheduler job creation, and Agent Execution dispatch remain in
    /// service/runtime-host layers rather than presentation code.
    pub scheduled_agent_task_client: Arc<dyn SystemScheduledAgentTaskClient>,
    /// The serviceized Heartbeat client used by application-scoped Heartbeat
    /// Operations routes.
    ///
    /// Web stores only the focused SDK facade. Native cadence, profile state,
    /// wake coalescing, gates, and run mementos remain in `service.heartbeat`
    /// and runtime-host providers.
    pub heartbeat_client: Arc<dyn SystemHeartbeatClient>,
    /// The serviceized Web3 client used by optional Web3 shell/status paths.
    pub web3_client: Arc<dyn SystemWeb3Client>,
    /// The serviceized EVM client used by optional EVM shell/status paths.
    pub evm_client: Arc<dyn SystemEvmClient>,
    /// The serviceized Plugin Control client used by plugin management routes.
    pub plugin_control_client: Arc<dyn SystemPluginControlClient>,
    /// The serviceized Plugin Capability client used by capability-plane routes.
    pub plugin_capability_client: Arc<dyn SystemPluginCapabilityClient>,
    /// The serviceized Plugin Hook client used by hook-plane routes and adapters.
    pub plugin_hook_client: Arc<dyn SystemPluginHookClient>,
    /// Primary Web-local facade bundle for low-risk system route adapters.
    ///
    /// New status, optional-module, package, entitlement, and payment routes
    /// should enter system semantics through this bundle or the focused SDK
    /// clients above. Deprecated lower-level fields remain only for high-risk
    /// chat/session/framework compatibility paths that are still being migrated.
    pub system_facade: WebSystemFacadeBundle,
    /// Shared service runtime used by Web-local compatibility adapters that
    /// must enter new system semantics through service boundaries.
    pub service_runtime: Arc<ServiceRuntime>,
    /// Host-owned autonomy runtime bundle retained for supervisor lifecycle.
    ///
    /// Dropping this bundle would drop the only handle the web host has for
    /// deterministic shutdown and operator diagnostics. The actual background
    /// work remains inside the supervisor task and service providers.
    pub autonomy_runtime: macaca_runtime_host::AutonomyRuntimeBundle,
    /// The LLM provider (DashScope by default).
    #[deprecated(note = "Use AppState::llm_client for new LLM call paths")]
    pub llm: Arc<dyn LlmProvider>,
    /// Shared router/resolver used by framework-based agents.
    #[deprecated(note = "Use AppState::llm_client for new model dispatch paths")]
    pub llm_router: Arc<LlmRouter>,
    /// Composite toolset: built-in tools + executable skill tools + claude code tools.
    pub tools: Arc<dyn ToolCatalog>,
    /// Application executor registry for isolated multi-agent execution.
    pub executor_registry: Arc<ApplicationExecutorRegistry>,
    /// Canonical web memory runtime used by production tools, active recall, and knowledge digest.
    ///
    /// The runtime is optional because memory exposure is still controlled by
    /// `context.recall.expose_memory_tools`. When present, upper web code must prefer this facade
    /// over concrete managers so provider/runtime implementations remain swappable.
    #[deprecated(note = "Use AppState::memory_client for new memory call paths")]
    pub memory_runtime: Option<Arc<crate::memory_runtime::WebMemoryRuntime>>,
    /// Legacy builtin backing store retained for compatibility and default runtime construction.
    pub workspace_memory: Option<Arc<macaca_memory::TestMemoryManager>>,
    /// Tombstone registry paired with [`Self::workspace_memory`] for digest + `memory_forget` coordination.
    pub workspace_memory_tombstones: Option<Arc<macaca_memory::SharedTombstoneRegistry>>,
    /// Agent OS level MCP runtime and registry.
    #[deprecated(note = "Use AppState::mcp_client for new MCP call paths")]
    pub mcp_runtime: Arc<macaca_runtime_host::McpRuntimeFacade>,
    /// Driver registry for managing loaded software drivers.
    #[deprecated(note = "Use AppState::driver_client for new driver call paths")]
    pub driver_registry: Arc<DriverRegistry>,
    /// Driver runtime facade for lifecycle, inventory, and tool collection.
    #[deprecated(note = "Use AppState::driver_client for new driver call paths")]
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
    /// Neutral context-provider factory registry (extensions / tests) — empty by default.
    pub context_provider_registry: Arc<ContextProviderRegistry>,
    /// Neutral context-engine registry (extensions / tests) overlaid on top of builtin engines.
    pub context_engine_registry: Arc<ContextEngineRegistry>,
    /// Explicit installation/runtime inventory for external adapter engines.
    pub external_adapter_runtime_registry: Arc<ExternalAdapterRuntimeRegistry>,
    /// Last-known invocation summaries for `GET /api/context/provider-runtime`.
    pub provider_health_ledger: Arc<ProviderHealthLedger>,
}

#[cfg(test)]
mod tests {
    use super::ExternalAdapterRuntimeRegistry;
    use std::collections::HashSet;

    use macaca_context::ContextEngineInfo;

    #[tokio::test]
    async fn external_adapter_runtime_registry_syncs_only_overlay_engines() {
        let registry = ExternalAdapterRuntimeRegistry::new();
        registry
            .sync_registry_overlay_engines(
                &HashSet::from(["legacy".to_string()]),
                &[
                    ContextEngineInfo::new("legacy", "Legacy Context Engine"),
                    ContextEngineInfo::new("custom-external", "Custom External Engine"),
                ],
            )
            .await;

        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].engine.id, "custom-external");
        assert_eq!(
            snapshot[0].installation_source,
            "context_engine_registry_overlay"
        );
        assert_eq!(snapshot[0].runtime_state, "installed");
    }

    #[tokio::test]
    async fn external_adapter_runtime_registry_prunes_removed_overlay_engines() {
        let registry = ExternalAdapterRuntimeRegistry::new();
        registry
            .sync_registry_overlay_engines(
                &HashSet::new(),
                &[ContextEngineInfo::new(
                    "custom-external",
                    "Custom External Engine",
                )],
            )
            .await;
        registry
            .sync_registry_overlay_engines(
                &HashSet::new(),
                &[ContextEngineInfo::new(
                    "replacement-external",
                    "Replacement External Engine",
                )],
            )
            .await;

        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].engine.id, "replacement-external");
    }
}
