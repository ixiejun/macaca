//! Shared bootstrap carrier passed between composition-root phase modules.
//!
//! Each phase reads prior fields and writes its outputs. Fields use `Option` so the
//! orchestrator can construct an empty carrier and phases fail fast on ordering bugs.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use macaca_app::{AppRegistry, AppRuntime};
use macaca_sdk::driver::{DriverRegistry, DriverRuntime};
use macaca_framework::session::SessionStore as FrameworkSessionStore;
use macaca_sdk::kernel::{audit::AuditLogger, alert::AlertManager, Kernel};
use macaca_sdk::llm::{LlmProvider, LlmRouter};
use macaca_sdk::memory::TestMemoryManager;
use macaca_proto::config::MacacaConfig;
use macaca_proto::ApplicationId;
use macaca_runtime_host::{AutonomyRuntimeBundle, McpRuntimeFacade, ServiceRuntime};
use macaca_sdk::skill::SkillCatalog;
use macaca_sdk::tools::ToolCatalog;

use crate::memory_runtime::WebMemoryRuntime;
use crate::run_trace::RunTracer;
use crate::shell::WebSystemFacadeBundle;
use crate::state::{AppState, ExternalAdapterRuntimeRegistry, ForkSessionMapping};
use crate::wasm_orchestration_backend::WebApplicationOrchestrationBackend;

/// Mutable state threaded through web bootstrap phases (Composition Root carrier).
#[derive(Default)]
pub(crate) struct BootstrapCtx {
    pub config: Option<MacacaConfig>,
    pub llm_router: Option<Arc<LlmRouter>>,
    pub llm: Option<Arc<dyn LlmProvider>>,
    pub kernel: Option<Arc<Kernel>>,
    pub runtime: Option<Arc<AppRuntime>>,
    pub registry: Option<Arc<tokio::sync::RwLock<AppRegistry>>>,
    pub discovered: Option<Vec<macaca_app::DiscoveredApp>>,
    pub service_runtime: Option<Arc<ServiceRuntime>>,
    pub autonomy_runtime: Option<AutonomyRuntimeBundle>,
    pub application_orchestration_registry_ref:
        Option<Arc<tokio::sync::RwLock<Option<Arc<macaca_runtime_host::ApplicationExecutorRegistry>>>>>,
    pub app_workspaces:
        Option<Arc<tokio::sync::RwLock<HashMap<ApplicationId, crate::workspace::AppWorkspace>>>>,
    pub orchestration_backend: Option<Arc<WebApplicationOrchestrationBackend>>,
    pub app_dirs: Option<HashMap<ApplicationId, PathBuf>>,
    pub skills_dirs: Option<Vec<PathBuf>>,
    pub started_apps: Option<Vec<(ApplicationId, String, Vec<String>)>>,
    pub service_audit_bundle: Option<macaca_runtime_host::ServiceAuditRuntimeBundle>,
    pub wasm_host_import_bridge:
        Option<macaca_runtime_host::wasm_runtime_provider::WasmHostImportBridge>,
    pub generic_service_client: Option<Arc<dyn macaca_sdk::SystemServiceClient>>,
    pub application_client: Option<Arc<dyn macaca_sdk::SystemApplicationClient>>,
    pub catalog: Option<SkillCatalog>,
    pub tools: Option<Arc<dyn ToolCatalog>>,
    pub driver_registry: Option<Arc<DriverRegistry>>,
    pub driver_runtime: Option<Arc<DriverRuntime>>,
    pub drivers_dir: Option<String>,
    pub fork_to_session:
        Option<Arc<tokio::sync::RwLock<HashMap<macaca_proto::ForkId, ForkSessionMapping>>>>,
    pub executor_registry_ref:
        Option<Arc<tokio::sync::RwLock<Option<Arc<macaca_runtime_host::ApplicationExecutorRegistry>>>>>,
    pub delegate_session_id: Option<Arc<tokio::sync::RwLock<Option<String>>>>,
    pub data_dir: Option<PathBuf>,
    pub session_db_path: Option<PathBuf>,
    pub session_store_impl: Option<Arc<macaca_runtime_host::persist::RedbStore>>,
    pub session_store_shared: Option<Arc<dyn macaca_runtime_host::persist::PersistBackend>>,
    pub todo_store: Option<Arc<macaca_sdk::task::TodoStore>>,
    pub event_log: Option<Arc<macaca_runtime_host::persist::EventLog>>,
    pub entitlement_store: Option<Arc<dyn macaca_runtime_host::persist::EntitlementStore>>,
    pub payment_store: Option<Arc<dyn macaca_runtime_host::persist::PaymentStore>>,
    pub entitlement_facade: Option<Arc<macaca_runtime_host::EntitlementRuntimeFacade>>,
    pub run_tracer: Option<Arc<RunTracer>>,
    pub kernel_persistence: Option<Arc<crate::persistence_adapter::RedbKernelPersistenceAdapter>>,
    pub audit_logger: Option<Arc<AuditLogger>>,
    pub session_store: Option<Arc<dyn macaca_runtime_host::persist::PersistBackend>>,
    pub alert_manager: Option<Arc<AlertManager>>,
    pub default_model: Option<String>,
    pub framework_session_store: Option<Arc<dyn FrameworkSessionStore>>,
    pub mcp_runtime: Option<Arc<McpRuntimeFacade>>,
    pub memory_runtime: Option<Arc<WebMemoryRuntime>>,
    pub workspace_memory: Option<Arc<TestMemoryManager>>,
    pub workspace_memory_tombstones: Option<Arc<macaca_sdk::memory::SharedTombstoneRegistry>>,
    pub memory_client: Option<Arc<dyn macaca_sdk::SystemMemoryClient>>,
    pub external_adapter_runtime_registry: Option<Arc<ExternalAdapterRuntimeRegistry>>,
    pub context_engine_registry: Option<Arc<macaca_context::ContextEngineRegistry>>,
    pub llm_client: Option<Arc<dyn macaca_sdk::SystemLlmClient>>,
    pub context_client: Option<Arc<dyn macaca_sdk::SystemContextClient>>,
    pub driver_client: Option<Arc<dyn macaca_sdk::SystemDriverClient>>,
    pub skill_client: Option<Arc<dyn macaca_sdk::SystemSkillClient>>,
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
