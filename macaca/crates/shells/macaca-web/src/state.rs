//! Shared application state for the web server.

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use axum::response::sse::Event;
use tokio::sync::{mpsc, RwLock};

use macaca_host_composition::context::{
    ContextEngineRegistry, ContextProviderRegistry, ProviderHealthLedger,
};
use macaca_host_composition::executor::ApplicationExecutorRegistry;
use macaca_host_composition::framework::runtime_context::AgentSessionStore as FrameworkAgentSessionStore;
use macaca_host_composition::kernel::Kernel;
use macaca_host_composition::persist::{EventLog, PersistBackend};
use macaca_host_composition::service_runtime::ServiceRuntime;
use macaca_host_composition::tools::{TodoStore, ToolCatalog};
use macaca_host_composition::{SystemContextClient, SystemSkillClient};
use macaca_proto::{
    config::ContextConfig, AppendEventCommand, ApplicationId, ForkId, LlmMessage, MacacaResult,
    UiEventCommand,
};
use macaca_sdk::{
    SharedDomainPackCatalog, SkillCatalogEntryView, SystemApplicationClient, SystemDriverClient,
    SystemEntitlementClient, SystemEvmClient, SystemHeartbeatClient, SystemLlmClient,
    SystemMcpClient, SystemMemoryClient, SystemPaymentClient, SystemPluginCapabilityClient,
    SystemPluginControlClient, SystemPluginHookClient, SystemScheduledAgentTaskClient,
    SystemSchedulerClient, SystemStatusClient, SystemStoreClient, SystemToolClient,
    SystemWeb3Client,
};

use crate::app_skill_status_facade::{AppSkillStatusSnapshot, SystemAppSkillStatusClient};
use crate::application_execution_event_log::ApplicationExecutionEventLog;
use crate::context_runtime_facade::{
    ContextProviderRuntimeSnapshot, ExternalAdapterRuntimeRegistry, SystemContextRuntimeClient,
};
use crate::run_trace::RunTraceSink;
use crate::session_inspection_facade::{
    ManualSessionCompactionSnapshot, SessionLineageSnapshot, SystemSessionInspectionClient,
};
use crate::shell::WebSystemFacadeBundle;
use crate::shell_composition_bundle::WebShellCompositionBundle;
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

/// Browser-facing active session state.
///
/// This type intentionally excludes execution-control pause/resume channels.
/// Runtime-host owns those local notification handles so Web session
/// presentation state does not become the execution-control authority.
#[derive(Clone)]
pub struct ActiveSession {
    /// The session ID.
    pub session_id: String,
    /// The application ID.
    pub app_id: ApplicationId,
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
    /// Sparse pipeline checkpoints (`run_trace` events + metrics).
    pub run_tracer: Arc<crate::run_trace::RunTracer>,
}

/// AppState-local adapter from the host-owned EventLog to the stream Observer port.
///
/// The HTTP/SSE route modules depend only on [`ApplicationExecutionEventLog`].
/// Keeping this adapter beside `PersistenceState` makes the concrete EventLog
/// handle part of the Web state persistence boundary instead of the reusable
/// observer contract.
struct StateApplicationExecutionEventLog {
    event_log: Arc<EventLog>,
}

impl StateApplicationExecutionEventLog {
    /// Wrap the shared EventLog without transferring persistence ownership.
    fn new(event_log: Arc<EventLog>) -> Self {
        Self { event_log }
    }
}

#[async_trait::async_trait]
impl ApplicationExecutionEventLog for StateApplicationExecutionEventLog {
    async fn query_indexed(
        &self,
        query: macaca_proto::EventLogQuery,
    ) -> Vec<macaca_proto::EventEntry> {
        self.event_log.query_indexed(query).await
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<(String, u64)> {
        self.event_log.subscribe()
    }
}

/// AppState-local adapter from the host EventLog to the run-trace command port.
///
/// `RunTracer` owns checkpoint naming and metrics, while this adapter owns the
/// concrete persistence write so run-trace emission does not import the host
/// composition facade directly.
pub(crate) struct StateRunTraceSink {
    event_log: Arc<EventLog>,
}

impl StateRunTraceSink {
    /// Wrap the shared EventLog without transferring persistence ownership.
    pub(crate) fn new(event_log: Arc<EventLog>) -> Self {
        Self { event_log }
    }
}

#[async_trait::async_trait]
impl RunTraceSink for StateRunTraceSink {
    async fn append_run_trace(&self, session_id: &str, payload: serde_json::Value) {
        self.event_log
            .append_command(AppendEventCommand::new(
                session_id,
                "run_trace",
                "run_tracer",
                payload,
            ))
            .await;
    }
}

/// Loop lifecycle state owned by runtime-host local controllers.
pub struct LoopState {
    /// Runtime-host owned local PlanLoop/WorkerLoop handles.
    ///
    /// Web keeps only this owner handle so presentation code can request local
    /// wake/shutdown effects without storing execution-loop maps itself.
    pub local_runtime: Arc<macaca_host_composition::execution_control::SessionLoopLocalRuntime>,
}

/// Session-related state: active sessions, conversation caches, mappings.
pub struct SessionState {
    /// Conversation history per app session (session_id -> messages) - in-memory cache.
    pub conversations: RwLock<HashMap<String, Vec<LlmMessage>>>,
    /// Active task cancellation flags (app_id -> cancel flag).
    pub cancel_flags: RwLock<HashMap<String, Arc<AtomicBool>>>,
    /// Browser-facing active sessions keyed by session id.
    pub active_sessions: RwLock<HashMap<String, ActiveSession>>,
    /// Runtime-host owned local execution-control notification registry.
    ///
    /// Web stores only the owner handle. The map of pause flags and resume
    /// channels lives in runtime-host so shell state remains a thin adapter
    /// around service-authoritative execution-control decisions.
    pub execution_control_local_notifications:
        Arc<macaca_host_composition::execution_control::ExecutionControlLocalNotificationRuntime>,
    /// Mapping from fork_id to session context for hook notifications.
    ///
    /// Shared with orchestration tool callbacks via `Arc` so `delegate_task` can
    /// register mappings before `AppState` is fully constructed at web bootstrap.
    pub fork_to_session: Arc<RwLock<HashMap<ForkId, ForkSessionMapping>>>,
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
    pub framework_session_store: Arc<dyn FrameworkAgentSessionStore>,
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
    /// Read-only Skill catalog snapshot for operator-facing route rendering.
    pub catalog_entries: Vec<SkillCatalogEntryView>,
    /// Service-backed alert client for provider-neutral alert delivery.
    pub alert_client: Arc<dyn macaca_sdk::SystemAlertClient>,
}

/// Shared state passed to all route handlers via axum's State extractor.
pub struct AppState {
    /// Provider-neutral system status Strategy consumed by the HTTP status route.
    ///
    /// The concrete runtime-backed adapter is installed by the process
    /// composition root. Route code does not inspect kernel, provider, or
    /// application-runtime internals to construct its response.
    pub status_client: Arc<dyn SystemStatusClient>,
    /// Provider-neutral session inspection Facade consumed by session routes.
    ///
    /// The current implementation is installed by Web bootstrap, but routes see
    /// only this port. That keeps session lineage memento semantics replaceable
    /// by the host composition root without changing HTTP handlers.
    pub session_inspection_client: Arc<dyn SystemSessionInspectionClient>,
    /// Provider-neutral app Skill status Facade consumed by Skill routes.
    ///
    /// Route handlers use this port instead of resolving app manifests or
    /// constructing Skill service commands directly.
    pub app_skill_status_client: Arc<dyn SystemAppSkillStatusClient>,
    /// Provider-neutral context runtime snapshot Facade consumed by diagnostics routes.
    ///
    /// AppState keeps the shared registries for remaining framework construction
    /// paths, while route-facing snapshot behavior is delegated to this client.
    pub context_runtime_client: Arc<dyn SystemContextRuntimeClient>,
    /// The kernel managing all agents.
    pub kernel: Arc<Kernel>,
    /// Bootstrap-owned provider anchors accessed only through shell adapters.
    ///
    /// The thin-shell boundary groups runtime/provider handles here so route
    /// handlers depend on SDK clients and adapters instead of direct fields.
    pub composition: WebShellCompositionBundle,
    /// Host-installed catalog for manifest `use_packs` expansion in UI routes.
    ///
    /// This handle mirrors the catalog injected into `AppRuntime` and Application
    /// Service during bootstrap so shell adapters do not reconstruct pack metadata.
    pub domain_pack_catalog: SharedDomainPackCatalog,
    /// The serviceized Application client used by protocol call paths.
    pub application_client: Arc<dyn SystemApplicationClient>,
    /// The serviceized LLM client used by protocol call paths.
    pub llm_client: Arc<dyn SystemLlmClient>,
    /// The serviceized Memory client used by protocol call paths.
    pub memory_client: Arc<dyn SystemMemoryClient>,
    /// The serviceized Context client used by protocol call paths.
    pub context_client: Arc<dyn SystemContextClient>,
    /// The serviceized Driver client used by protocol call paths.
    pub driver_client: Arc<dyn SystemDriverClient>,
    /// The serviceized Skill client used by protocol call paths.
    pub skill_client: Arc<dyn SystemSkillClient>,
    /// The serviceized MCP client used by protocol call paths.
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
    /// Status, optional-module, package, entitlement, and payment routes should
    /// enter system semantics through this bundle or the focused SDK clients
    /// above. Lower-level fields are confined to the current web composition
    /// root while follow-up thin-shell tasks move remaining ownership behind
    /// service/provider boundaries.
    pub system_facade: WebSystemFacadeBundle,
    /// Shared service runtime used by Web-local adapters that must enter system
    /// semantics through service boundaries.
    pub service_runtime: Arc<ServiceRuntime>,
    /// Host-owned autonomy runtime bundle retained for supervisor lifecycle.
    ///
    /// Dropping this bundle would drop the only handle the web host has for
    /// deterministic shutdown and operator diagnostics. The actual background
    /// work remains inside the supervisor task and service providers.
    pub autonomy_runtime: macaca_host_composition::autonomy_runtime::AutonomyRuntimeBundle,
    /// Composite toolset: built-in tools + executable skill tools + claude code tools.
    pub tools: Arc<dyn ToolCatalog>,
    /// Application executor registry for isolated multi-agent execution.
    pub executor_registry: Arc<ApplicationExecutorRegistry>,
    /// Tombstone registry paired with the Memory service runtime for digest and forget coordination.
    pub workspace_memory_tombstones:
        Option<Arc<macaca_host_composition::memory::SharedTombstoneRegistry>>,
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

impl AppState {
    /// Return the focused SDK LLM client used by shell adapters.
    ///
    /// This accessor intentionally exposes only the provider-neutral service
    /// facade. It keeps call sites from reaching into composition-root provider
    /// handles while preserving the unified `service.call` ownership introduced
    /// by the protocol microkernel refactor.
    pub(crate) fn service_llm_client(&self) -> Arc<dyn SystemLlmClient> {
        Arc::clone(&self.llm_client)
    }

    /// Build the sanitized context-provider runtime snapshot for operator routes.
    ///
    /// This delegates to the focused Facade so route code and AppState do not own
    /// host-composition registry traversal or external-adapter inventory shaping.
    pub(crate) async fn context_provider_runtime_snapshot(&self) -> ContextProviderRuntimeSnapshot {
        self.context_runtime_client.snapshot().await
    }

    /// Create a successor lineage segment for an operator-requested compaction.
    ///
    /// Design pattern: **Facade + Memento**. The route supplies only the session
    /// id and optional focus topic; this method owns the persistence memento
    /// updates, emits bounded audit events, and returns a sanitized read model.
    /// No raw transcript content is copied into the response or event payloads.
    pub(crate) async fn manual_session_compaction_snapshot(
        &self,
        session_id: String,
        focus_topic: Option<String>,
    ) -> MacacaResult<ManualSessionCompactionSnapshot> {
        self.session_inspection_client
            .manual_compaction_snapshot(session_id, focus_topic)
            .await
    }

    /// Load a bounded lineage snapshot for session inspection routes.
    ///
    /// This keeps host-owned persistence helpers out of HTTP handlers while
    /// preserving the protocol DTO response shape expected by clients.
    pub(crate) async fn session_lineage_snapshot(
        &self,
        session_id: String,
    ) -> MacacaResult<SessionLineageSnapshot> {
        self.session_inspection_client
            .lineage_snapshot(session_id)
            .await
    }

    /// Build application-scoped Skill status rows for a route request.
    ///
    /// Design pattern: **Facade + Adapter**. This method adapts application
    /// manifest agent declarations into Skill service snapshot commands, then
    /// maps the service result into route-safe rows. The HTTP handler remains
    /// responsible only for parsing the app id and serializing the response.
    pub(crate) async fn app_skill_status_snapshots(
        &self,
        app_id: ApplicationId,
        agent_filter: Option<String>,
    ) -> MacacaResult<Vec<AppSkillStatusSnapshot>> {
        self.app_skill_status_client
            .app_skill_status_snapshots(app_id, agent_filter)
            .await
    }

    /// Persist a GenUI event command through the shared append-only EventLog.
    ///
    /// The route layer owns HTTP validation and protocol command construction,
    /// while AppState keeps the concrete persistence adapter behind this narrow
    /// Facade. This preserves durable audit/replay behavior without forcing the
    /// route module to import host-composition persistence types.
    pub(crate) async fn persist_genui_event_command(
        &self,
        app_id: &str,
        session_id: &str,
        command: &UiEventCommand,
    ) -> u64 {
        self.persist
            .event_log
            .append_command(
                AppendEventCommand::new(
                    session_id,
                    "genui_event",
                    "genui_web_shell",
                    serde_json::to_value(command).unwrap_or_else(|error| {
                        tracing::warn!(
                            app_id,
                            session_id,
                            error = %error,
                            "GenUI event command serialization failed before persistence"
                        );
                        serde_json::json!({})
                    }),
                )
                .with_app_id(app_id.to_string()),
            )
            .await
    }

    /// Return the durable application execution event observer port.
    ///
    /// Stream routes use this port to replay and subscribe to EventLog rows
    /// without importing host persistence types. The concrete adapter is created
    /// per request around the shared EventLog handle and remains stateless.
    pub(crate) fn application_execution_event_log(&self) -> Arc<dyn ApplicationExecutionEventLog> {
        Arc::new(StateApplicationExecutionEventLog::new(Arc::clone(
            &self.persist.event_log,
        )))
    }
}
