//! Web orchestration tool assembly.

use std::sync::Arc;

use futures::FutureExt;
use macaca_host_composition::executor::ApplicationExecutorRegistry;
use macaca_host_composition::kernel::Kernel;
use macaca_host_composition::service_runtime::ServiceRuntime;
use macaca_host_composition::tool_bootstrap::{
    bootstrap_delegate_task_tool, bootstrap_get_task_result_tool, bootstrap_list_agents_tool,
    DelegateTaskToolBootstrapPorts, ForkSessionMappingRecorder,
};
use macaca_host_composition::tools::{CompositeToolSet, Tool, ToolCatalog};
use macaca_proto::ForkId;
use tokio::sync::RwLock;
use tracing::info;

use crate::fork_join_shell_adapter::fork_session_mapping;
use crate::state::ForkSessionMapping;

/// Toolset and late-bound service handles needed while AppState is being built.
pub(crate) struct WebToolAssembly {
    pub tools: Arc<dyn ToolCatalog>,
    pub executor_registry_ref: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
    pub delegate_session_id: Arc<tokio::sync::RwLock<Option<String>>>,
    /// Shared fork-to-session index used by hook_consumer resume delivery.
    pub fork_to_session: Arc<RwLock<std::collections::HashMap<ForkId, ForkSessionMapping>>>,
}

/// Build built-in, skill, and orchestration tools without changing callback behavior.
pub(crate) fn build_web_tools(
    kernel: Arc<Kernel>,
    service_runtime: Arc<ServiceRuntime>,
    fork_to_session: Arc<RwLock<std::collections::HashMap<ForkId, ForkSessionMapping>>>,
    mut all_tools: Vec<Box<dyn Tool>>,
) -> WebToolAssembly {
    let executor_registry_ref: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>> =
        Arc::new(RwLock::new(None));

    all_tools.push(bootstrap_list_agents_tool(
        Arc::clone(&kernel),
        "web-startup-list-agents-tool",
    ));
    info!("ListAgents tool added");

    let delegate_session_id: Arc<tokio::sync::RwLock<Option<String>>> =
        Arc::new(tokio::sync::RwLock::new(None));
    let fork_to_session_for_delegate = Arc::clone(&fork_to_session);
    let fork_session_recorder: ForkSessionMappingRecorder =
        Arc::new(move |fork_id, app_id, parent_agent, parent_session_id| {
            let fork_to_session = Arc::clone(&fork_to_session_for_delegate);
            async move {
                // Register fork-to-session mapping so hook_consumer can correlate
                // ForkValidated/DelegateFailed events with the waiting parent shell stream.
                fork_to_session.write().await.insert(
                    fork_id,
                    fork_session_mapping(parent_session_id, app_id, parent_agent),
                );
                Ok(())
            }
            .boxed()
        });
    all_tools.push(bootstrap_delegate_task_tool(
        Arc::clone(&service_runtime),
        DelegateTaskToolBootstrapPorts {
            executor_registry_ref: Arc::clone(&executor_registry_ref),
            delegate_session_id: Arc::clone(&delegate_session_id),
            fork_session_recorder,
        },
        "web-startup-delegate-task-tool",
    ));
    info!("DelegateTask tool added");

    all_tools.push(bootstrap_get_task_result_tool(
        Arc::clone(&executor_registry_ref),
        "web-startup-get-task-result-tool",
    ));
    info!("GetTaskResult tool added");

    let tool_names: Vec<&str> = all_tools.iter().map(|t| t.name()).collect();
    info!(tools = ?tool_names, "Composite toolset ready");

    WebToolAssembly {
        tools: Arc::new(CompositeToolSet::new(all_tools)),
        executor_registry_ref,
        delegate_session_id,
        fork_to_session,
    }
}
