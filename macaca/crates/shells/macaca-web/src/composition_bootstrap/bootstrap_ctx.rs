//! Shared bootstrap carrier passed between composition-root phase modules.
//!
//! Each phase reads prior fields and writes its outputs. Fields use `Option` so the
//! orchestrator can construct an empty carrier and phases fail fast on ordering bugs.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use macaca_host_composition::app::{AppRegistry, AppRuntime};
use macaca_host_composition::autonomy_runtime::AutonomyRuntimeBundle;
use macaca_host_composition::framework::runtime_context::AgentSessionStore as FrameworkAgentSessionStore;
use macaca_host_composition::kernel::{AuditLogger, Kernel};
use macaca_host_composition::llm::{LlmProvider, LlmRouter};
use macaca_host_composition::mcp_runtime::McpRuntimeFacade;
use macaca_host_composition::service_runtime::ServiceRuntime;
use macaca_host_composition::tools::ToolCatalog;
use macaca_proto::config::MacacaConfig;
use macaca_proto::ApplicationId;
use macaca_sdk::SharedDomainPackCatalog;

use crate::context_runtime_facade::ExternalAdapterRuntimeRegistry;
use crate::run_trace::RunTracer;
use crate::shell::WebSystemFacadeBundle;
use crate::state::{AppState, ForkSessionMapping};
use crate::wasm_orchestration_backend::WebApplicationOrchestrationBackend;

/// Mutable state threaded through web bootstrap phases (Composition Root carrier).
#[derive(Default)]
pub(crate) struct BootstrapCtx {
    pub config: Option<MacacaConfig>,
    pub llm_router: Option<Arc<LlmRouter>>,
    pub llm: Option<Arc<dyn LlmProvider>>,
    pub kernel: Option<Arc<Kernel>>,
    pub runtime: Option<Arc<AppRuntime>>,
    /// Host-installed domain-pack catalog shared across runtime and UI routes.
    pub domain_pack_catalog: Option<SharedDomainPackCatalog>,
    pub registry: Option<Arc<tokio::sync::RwLock<AppRegistry>>>,
    pub discovered: Option<Vec<macaca_host_composition::app::DiscoveredApp>>,
    pub service_runtime: Option<Arc<ServiceRuntime>>,
    pub autonomy_runtime: Option<AutonomyRuntimeBundle>,
    pub application_orchestration_registry_ref: Option<
        Arc<
            tokio::sync::RwLock<
                Option<Arc<macaca_host_composition::executor::ApplicationExecutorRegistry>>,
            >,
        >,
    >,
    pub app_workspaces:
        Option<Arc<tokio::sync::RwLock<HashMap<ApplicationId, crate::workspace::AppWorkspace>>>>,
    pub orchestration_backend: Option<Arc<WebApplicationOrchestrationBackend>>,
    pub app_dirs: Option<HashMap<ApplicationId, PathBuf>>,
    pub skills_dirs: Option<Vec<PathBuf>>,
    pub started_apps: Option<Vec<(ApplicationId, String, Vec<String>)>>,
    pub service_audit_bundle:
        Option<macaca_host_composition::service_runtime::ServiceAuditRuntimeBundle>,
    pub wasm_host_import_bridge: Option<
        macaca_host_composition::application_bootstrap::wasm_runtime_provider::WasmHostImportBridge,
    >,
    pub generic_service_client: Option<Arc<dyn macaca_sdk::SystemServiceClient>>,
    pub application_client: Option<Arc<dyn macaca_sdk::SystemApplicationClient>>,
    pub catalog_entries: Option<Vec<macaca_sdk::SkillCatalogEntryView>>,
    pub tools: Option<Arc<dyn ToolCatalog>>,
    pub drivers_dir: Option<String>,
    pub fork_to_session:
        Option<Arc<tokio::sync::RwLock<HashMap<macaca_proto::ForkId, ForkSessionMapping>>>>,
    pub executor_registry_ref: Option<
        Arc<
            tokio::sync::RwLock<
                Option<Arc<macaca_host_composition::executor::ApplicationExecutorRegistry>>,
            >,
        >,
    >,
    pub delegate_session_id: Option<Arc<tokio::sync::RwLock<Option<String>>>>,
    pub data_dir: Option<PathBuf>,
    pub session_db_path: Option<PathBuf>,
    pub session_store_impl: Option<Arc<macaca_host_composition::persist::RedbStore>>,
    pub session_store_shared: Option<Arc<dyn macaca_host_composition::persist::PersistBackend>>,
    pub todo_store: Option<Arc<macaca_host_composition::tools::TodoStore>>,
    pub event_log: Option<Arc<macaca_host_composition::persist::EventLog>>,
    pub entitlement_store: Option<Arc<dyn macaca_host_composition::persist::EntitlementStore>>,
    pub payment_store: Option<Arc<dyn macaca_host_composition::persist::PaymentStore>>,
    pub run_tracer: Option<Arc<RunTracer>>,
    pub audit_logger: Option<Arc<AuditLogger>>,
    pub session_store: Option<Arc<dyn macaca_host_composition::persist::PersistBackend>>,
    pub alert_client: Option<Arc<dyn macaca_sdk::SystemAlertClient>>,
    pub default_model: Option<String>,
    pub framework_session_store: Option<Arc<dyn FrameworkAgentSessionStore>>,
    pub mcp_runtime: Option<Arc<McpRuntimeFacade>>,
    pub memory_runtime: Option<Arc<macaca_host_composition::memory::FabricMemoryRuntime>>,
    pub workspace_memory_tombstones:
        Option<Arc<macaca_host_composition::memory::SharedTombstoneRegistry>>,
    pub memory_client: Option<Arc<dyn macaca_sdk::SystemMemoryClient>>,
    pub external_adapter_runtime_registry: Option<Arc<ExternalAdapterRuntimeRegistry>>,
    pub context_engine_registry:
        Option<Arc<macaca_host_composition::context::ContextEngineRegistry>>,
    pub llm_client: Option<Arc<dyn macaca_sdk::SystemLlmClient>>,
    pub context_client: Option<Arc<dyn macaca_host_composition::SystemContextClient>>,
    pub driver_client: Option<Arc<dyn macaca_sdk::SystemDriverClient>>,
    pub skill_client: Option<Arc<dyn macaca_host_composition::SystemSkillClient>>,
    pub mcp_client: Option<Arc<dyn macaca_sdk::SystemMcpClient>>,
    pub tool_client: Option<Arc<dyn macaca_sdk::SystemToolClient>>,
    pub store_client: Option<Arc<dyn macaca_sdk::SystemStoreClient>>,
    pub entitlement_client: Option<Arc<dyn macaca_sdk::SystemEntitlementClient>>,
    pub payment_client: Option<Arc<dyn macaca_sdk::SystemPaymentClient>>,
    pub scheduler_client: Option<Arc<dyn macaca_sdk::SystemSchedulerClient>>,
    pub scheduled_agent_task_client: Option<Arc<dyn macaca_sdk::SystemScheduledAgentTaskClient>>,
    pub heartbeat_client: Option<Arc<dyn macaca_sdk::SystemHeartbeatClient>>,
    pub web3_client: Option<Arc<dyn macaca_sdk::SystemWeb3Client>>,
    pub evm_client: Option<Arc<dyn macaca_sdk::SystemEvmClient>>,
    pub plugin_control_client: Option<Arc<dyn macaca_sdk::SystemPluginControlClient>>,
    pub plugin_capability_client: Option<Arc<dyn macaca_sdk::SystemPluginCapabilityClient>>,
    pub plugin_hook_client: Option<Arc<dyn macaca_sdk::SystemPluginHookClient>>,
    pub application_execution_client: Option<Arc<dyn macaca_sdk::SystemApplicationExecutionClient>>,
    pub system_facade: Option<WebSystemFacadeBundle>,
    pub app_state: Option<Arc<AppState>>,
}
