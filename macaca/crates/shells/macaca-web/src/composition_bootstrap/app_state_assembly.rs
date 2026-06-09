//! Bootstrap phase 10: `AppState`, composition bundle, and cyclic runner wiring.
//!
//! Assembles the shared HTTP/session state behind a cyclic weak reference so `WebAgentRunner`
//! and `ApplicationExecutorRegistry` can reference the same composition root safely.

use std::collections::HashMap;
use std::sync::Arc;

use macaca_sdk::framework::session::{InMemorySessionStore as FrameworkInMemorySessionStore, SessionStore as FrameworkSessionStore};
use macaca_sdk::runtime_host::ApplicationExecutorRegistry;
use macaca_proto::MacacaResult;
use crate::agent_runner::WebAgentRunner;
use crate::state::{AppConfig, AppState, LoopState, PersistenceState, SessionState};

use super::bootstrap_ctx::BootstrapCtx;

/// Run the `app-state-assembly` bootstrap slice.
pub(crate) async fn run(ctx: &mut BootstrapCtx) -> MacacaResult<()> {
    let kernel = Arc::clone(ctx.kernel.as_ref().expect("bootstrap: kernel"));
    let runtime = Arc::clone(ctx.runtime.as_ref().expect("bootstrap: runtime"));
    let registry = Arc::clone(ctx.registry.as_ref().expect("bootstrap: registry"));
    let llm = Arc::clone(ctx.llm.as_ref().expect("bootstrap: llm"));
    let llm_router = Arc::clone(ctx.llm_router.as_ref().expect("bootstrap: llm_router"));
    let memory_runtime = ctx.memory_runtime.clone();
    let mcp_runtime = Arc::clone(ctx.mcp_runtime.as_ref().expect("bootstrap: mcp_runtime"));
    let driver_registry = Arc::clone(ctx.driver_registry.as_ref().expect("bootstrap: driver_registry"));
    let driver_runtime = Arc::clone(ctx.driver_runtime.as_ref().expect("bootstrap: driver_runtime"));
    let application_client = Arc::clone(ctx.application_client.as_ref().expect("bootstrap: application_client"));
    let llm_client = Arc::clone(ctx.llm_client.as_ref().expect("bootstrap: llm_client"));
    let memory_client = Arc::clone(ctx.memory_client.as_ref().expect("bootstrap: memory_client"));
    let context_client = Arc::clone(ctx.context_client.as_ref().expect("bootstrap: context_client"));
    let driver_client = Arc::clone(ctx.driver_client.as_ref().expect("bootstrap: driver_client"));
    let skill_client = Arc::clone(ctx.skill_client.as_ref().expect("bootstrap: skill_client"));
    let mcp_client = Arc::clone(ctx.mcp_client.as_ref().expect("bootstrap: mcp_client"));
    let tool_client = Arc::clone(ctx.tool_client.as_ref().expect("bootstrap: tool_client"));
    let store_client = Arc::clone(ctx.store_client.as_ref().expect("bootstrap: store_client"));
    let entitlement_client = Arc::clone(ctx.entitlement_client.as_ref().expect("bootstrap: entitlement_client"));
    let payment_client = Arc::clone(ctx.payment_client.as_ref().expect("bootstrap: payment_client"));
    let scheduler_client = Arc::clone(ctx.scheduler_client.as_ref().expect("bootstrap: scheduler_client"));
    let scheduled_agent_task_client = Arc::clone(ctx.scheduled_agent_task_client.as_ref().expect("bootstrap: scheduled_agent_task_client"));
    let heartbeat_client = Arc::clone(ctx.heartbeat_client.as_ref().expect("bootstrap: heartbeat_client"));
    let web3_client = Arc::clone(ctx.web3_client.as_ref().expect("bootstrap: web3_client"));
    let evm_client = Arc::clone(ctx.evm_client.as_ref().expect("bootstrap: evm_client"));
    let plugin_control_client = Arc::clone(ctx.plugin_control_client.as_ref().expect("bootstrap: plugin_control_client"));
    let plugin_capability_client = Arc::clone(ctx.plugin_capability_client.as_ref().expect("bootstrap: plugin_capability_client"));
    let plugin_hook_client = Arc::clone(ctx.plugin_hook_client.as_ref().expect("bootstrap: plugin_hook_client"));
    let system_facade = ctx.system_facade.clone().expect("bootstrap: system_facade");
    let service_runtime = Arc::clone(ctx.service_runtime.as_ref().expect("bootstrap: service_runtime"));
    let autonomy_runtime = ctx.autonomy_runtime.clone().expect("bootstrap: autonomy_runtime");
    let tools = Arc::clone(ctx.tools.as_ref().expect("bootstrap: tools"));
    let workspace_memory = ctx.workspace_memory.clone();
    let workspace_memory_tombstones = ctx.workspace_memory_tombstones.clone();
    let drivers_dir = ctx.drivers_dir.clone().expect("bootstrap: drivers_dir");
    let session_store = Arc::clone(ctx.session_store.as_ref().expect("bootstrap: session_store"));
    let todo_store = Arc::clone(ctx.todo_store.as_ref().expect("bootstrap: todo_store"));
    let event_log = Arc::clone(ctx.event_log.as_ref().expect("bootstrap: event_log"));
    let audit_logger = Arc::clone(ctx.audit_logger.as_ref().expect("bootstrap: audit_logger"));
    let run_tracer = Arc::clone(ctx.run_tracer.as_ref().expect("bootstrap: run_tracer"));
    let fork_to_session = Arc::clone(ctx.fork_to_session.as_ref().expect("bootstrap: fork_to_session"));
    let delegate_session_id = Arc::clone(ctx.delegate_session_id.as_ref().expect("bootstrap: delegate_session_id"));
    let framework_session_store = Arc::clone(ctx.framework_session_store.as_ref().expect("bootstrap: framework_session_store"));
    let app_dirs = ctx.app_dirs.clone().expect("bootstrap: app_dirs");
    let app_workspaces = Arc::clone(ctx.app_workspaces.as_ref().expect("bootstrap: app_workspaces"));
    let default_model = ctx.default_model.clone().expect("bootstrap: default_model");
    let catalog = ctx.catalog.take().expect("bootstrap: catalog");
    let alert_manager = Arc::clone(ctx.alert_manager.as_ref().expect("bootstrap: alert_manager"));
    let context_engine_registry = Arc::clone(ctx.context_engine_registry.as_ref().expect("bootstrap: context_engine_registry"));
    let external_adapter_runtime_registry = Arc::clone(ctx.external_adapter_runtime_registry.as_ref().expect("bootstrap: external_adapter_runtime_registry"));
    let config = ctx.config.clone().expect("bootstrap: config");
    let domain_pack_catalog = Arc::clone(
        ctx.domain_pack_catalog
            .as_ref()
            .expect("bootstrap: domain_pack_catalog"),
    );

    // 10. Build shared state.
    let state = Arc::new_cyclic(|weak_state| {
        // Create the real agent runner with the actual weak state
        let runner = Arc::new(WebAgentRunner::new(weak_state.clone()));
        let executor_registry = Arc::new(ApplicationExecutorRegistry::new(
            Arc::clone(&runner) as Arc<dyn macaca_sdk::runtime_host::AgentRunner>
        ));

        let composition = crate::shell_composition_bundle::WebShellCompositionBundle::new(
            runtime.clone(),
            Arc::clone(&registry),
            llm.clone(),
            llm_router.clone(),
            memory_runtime.clone(),
            Arc::clone(&mcp_runtime),
            Arc::clone(&driver_registry),
            Arc::clone(&driver_runtime),
        );

        AppState {
            kernel: kernel.clone(),
            composition,
            domain_pack_catalog: Arc::clone(&domain_pack_catalog),
            application_client: Arc::clone(&application_client),
            llm_client: Arc::clone(&llm_client),
            memory_client: Arc::clone(&memory_client),
            context_client: Arc::clone(&context_client),
            driver_client: Arc::clone(&driver_client),
            skill_client: Arc::clone(&skill_client),
            mcp_client: Arc::clone(&mcp_client),
            tool_client: Arc::clone(&tool_client),
            store_client: Arc::clone(&store_client),
            entitlement_client: Arc::clone(&entitlement_client),
            payment_client: Arc::clone(&payment_client),
            scheduler_client: Arc::clone(&scheduler_client),
            scheduled_agent_task_client: Arc::clone(&scheduled_agent_task_client),
            heartbeat_client: Arc::clone(&heartbeat_client),
            web3_client: Arc::clone(&web3_client),
            evm_client: Arc::clone(&evm_client),
            plugin_control_client: Arc::clone(&plugin_control_client),
            plugin_capability_client: Arc::clone(&plugin_capability_client),
            plugin_hook_client: Arc::clone(&plugin_hook_client),
            system_facade: system_facade.clone(),
            service_runtime: Arc::clone(&service_runtime),
            autonomy_runtime: autonomy_runtime.clone(),
            tools,
            executor_registry: executor_registry.clone(),
            workspace_memory: workspace_memory.clone(),
            workspace_memory_tombstones: workspace_memory_tombstones.clone(),
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
                fork_to_session: Arc::clone(&fork_to_session),
                goal_to_session: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
                delegate_session_id: Arc::clone(&delegate_session_id),
                llm_route_hints: tokio::sync::RwLock::new(HashMap::new()),
                framework_session_store: Arc::clone(&framework_session_store),
            },
            config: AppConfig {
                app_dirs: tokio::sync::RwLock::new(app_dirs),
                app_workspaces: Arc::clone(&app_workspaces),
                default_model,
                context: config.context.clone(),
                catalog: tokio::sync::RwLock::new(catalog),
                alert_manager: alert_manager.clone(),
            },
            context_provider_registry: Arc::new(macaca_sdk::context::ContextProviderRegistry::new()),
            context_engine_registry: Arc::clone(&context_engine_registry),
            external_adapter_runtime_registry: Arc::clone(&external_adapter_runtime_registry),
            provider_health_ledger: Arc::new(macaca_sdk::context::ProviderHealthLedger::new()),
        }
    });

    ctx.app_state = Some(state);
    Ok(())
}
