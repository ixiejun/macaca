//! Web orchestration tool assembly.

use std::sync::Arc;

use futures::FutureExt;
use macaca_kernel::{ApplicationExecutorRegistry, Kernel, TaskContext};
use macaca_runtime_host::ServiceRuntime;
use macaca_tools::{
    CompositeToolSet, DelegateTaskTool, GetTaskResultTool, ListAgentsTool, Tool, ToolCatalog,
};
use tokio::sync::RwLock;
use tracing::info;

use crate::delegated_task_dispatcher::{
    DelegateViaAgentServiceRequest, ServiceDelegatedTaskDispatcher,
};

/// Generic delegator label used when the orchestration tool does not receive an explicit caller agent.
///
/// This avoids hardcoding application role names (for example coordinator/planner personas) in the
/// OS shell while still giving the executor queue an auditable `from_agent` dimension.
const GENERIC_DELEGATOR_AGENT: &str = "delegator";

/// Toolset and compatibility handles needed while AppState is being built.
pub(crate) struct WebToolAssembly {
    pub tools: Arc<dyn ToolCatalog>,
    pub executor_registry_ref: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
    pub delegate_session_id: Arc<tokio::sync::RwLock<Option<String>>>,
}

/// Build built-in, skill, and orchestration tools without changing callback behavior.
pub(crate) fn build_web_tools(
    kernel: Arc<Kernel>,
    service_runtime: Arc<ServiceRuntime>,
    mut all_tools: Vec<Box<dyn Tool>>,
) -> WebToolAssembly {
    let delegated_task_dispatcher = Arc::new(ServiceDelegatedTaskDispatcher::new(service_runtime));
    let executor_registry_ref: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>> =
        Arc::new(RwLock::new(None));

    let kernel_for_callback = Arc::clone(&kernel);
    let list_agents_tool = ListAgentsTool::new().with_agents_callback(move || {
        let kernel = Arc::clone(&kernel_for_callback);
        async move {
            let agents = kernel.list_agents().await;
            agents
                .into_iter()
                .map(|agent| {
                    let capabilities: Vec<String> =
                        agent.capabilities.into_iter().map(|cap| cap.name).collect();
                    serde_json::json!({
                        "name": agent.name,
                        "capabilities": capabilities
                    })
                })
                .collect()
        }
        .boxed()
    });
    all_tools.push(Box::new(list_agents_tool));
    info!("ListAgents tool added");

    let delegate_session_id: Arc<tokio::sync::RwLock<Option<String>>> =
        Arc::new(tokio::sync::RwLock::new(None));
    let registry_for_delegate = Arc::clone(&executor_registry_ref);
    let dispatcher_for_delegate = Arc::clone(&delegated_task_dispatcher);
    let delegate_tool = DelegateTaskTool::empty_with_session_id(Arc::clone(&delegate_session_id))
        .with_callback(
            move |app_id, to_agent, prompt, priority, parallel, session_id| {
                let registry = Arc::clone(&registry_for_delegate);
                let dispatcher = Arc::clone(&dispatcher_for_delegate);
                async move {
                    let registry_guard = registry.read().await;
                    let registry = registry_guard
                        .as_ref()
                        .ok_or_else(|| "Executor registry not initialized".to_string())?;

                    let app_id = if app_id.is_empty() {
                        let apps = registry.list_applications().await;
                        apps.first()
                            .map(|(id, _)| id.clone())
                            .ok_or_else(|| "No applications registered in executor".to_string())?
                    } else {
                        uuid::Uuid::parse_str(&app_id)
                            .map(macaca_proto::ApplicationId)
                            .map_err(|e| format!("Invalid application ID: {}", e))?
                    };

                    let executor = registry
                        .get(&app_id)
                        .await
                        .ok_or_else(|| format!("App '{}' not found in registry", app_id))?;

                    let fork_manager = executor.fork_manager();

                    let acceptance_criteria = macaca_proto::AcceptanceCriteria {
                        description: format!(
                            "Task delegated to {}: {}",
                            to_agent,
                            prompt.chars().take(100).collect::<String>()
                        ),
                        required_artifacts: vec![],
                        auto_accept: false,
                    };

                    let fork_id = fork_manager
                        .create_fork(
                            None,
                            app_id.clone(),
                            to_agent.clone(),
                            prompt.clone(),
                            vec![],
                            String::new(),
                            acceptance_criteria,
                        )
                        .await
                        .map_err(|e| format!("Fork creation failed: {}", e))?;

                    fork_manager
                        .start_fork(fork_id)
                        .await
                        .map_err(|e| format!("Fork start failed: {}", e))?;

                    let task_context = session_id.map(|sid| TaskContext {
                        session_id: Some(sid),
                        artifacts: vec![],
                        env: std::collections::HashMap::new(),
                    });
                    let task_id = dispatcher
                        .dispatch(
                            executor,
                            DelegateViaAgentServiceRequest {
                                application_id: app_id.clone(),
                                from_agent: GENERIC_DELEGATOR_AGENT.to_string(),
                                to_agent: to_agent.clone(),
                                prompt,
                                priority,
                                parallel,
                                context: task_context,
                            },
                        )
                        .await
                        .map_err(|e| format!("Service-backed delegation failed: {}", e))?;

                    fork_manager
                        .suspend_fork(fork_id, macaca_proto::TaskId(task_id.0))
                        .await
                        .map_err(|e| format!("Fork suspend failed: {}", e))?;

                    Ok(format!("fork:{}", fork_id))
                }
                .boxed()
            },
        );
    all_tools.push(Box::new(delegate_tool));
    info!("DelegateTask tool added");

    let registry_for_result = Arc::clone(&executor_registry_ref);
    let get_result_tool =
        GetTaskResultTool::empty().with_callback(move |app_id, task_or_fork_id| {
            let registry = Arc::clone(&registry_for_result);
            async move {
                let registry_guard = registry.read().await;
                let registry = registry_guard
                    .as_ref()
                    .ok_or_else(|| "Executor registry not initialized".to_string())?;

                let app_id = if app_id.is_empty() {
                    let apps = registry.list_applications().await;
                    apps.first()
                        .map(|(id, _)| id.clone())
                        .ok_or_else(|| "No applications registered in executor".to_string())?
                } else {
                    uuid::Uuid::parse_str(&app_id)
                        .map(macaca_proto::ApplicationId)
                        .map_err(|e| format!("Invalid application ID: {}", e))?
                };

                let executor = registry
                    .get(&app_id)
                    .await
                    .ok_or_else(|| format!("App '{}' not found in registry", app_id))?;

                if let Some(fork_id_str) = task_or_fork_id.strip_prefix("fork:") {
                    let uuid_str = if let Some(uuid_part) = fork_id_str.strip_prefix("fork-") {
                        uuid_part
                    } else {
                        fork_id_str
                    };
                    let fork_id_uuid = uuid::Uuid::parse_str(uuid_str)
                        .map_err(|e| format!("Invalid fork_id '{}': {}", uuid_str, e))?;
                    let fork_id = macaca_proto::ForkId(fork_id_uuid);

                    let fork_manager = executor.fork_manager();
                    let fork = fork_manager
                        .get_fork(fork_id)
                        .await
                        .ok_or_else(|| format!("Fork '{}' not found", fork_id))?;

                    let (status_str, output, error) = match fork.state {
                        macaca_proto::ForkState::Pending => ("pending".to_string(), None, None),
                        macaca_proto::ForkState::Running => ("running".to_string(), None, None),
                        macaca_proto::ForkState::WaitingForHook => {
                            ("waiting".to_string(), None, None)
                        }
                        macaca_proto::ForkState::Completed => {
                            let output = fork
                                .own_messages
                                .iter()
                                .filter_map(|m| {
                                    if m.role == macaca_proto::LlmRole::Assistant {
                                        Some(m.content.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            ("completed".to_string(), Some(output), None)
                        }
                        macaca_proto::ForkState::Failed { ref error } => {
                            ("failed".to_string(), None, Some(error.clone()))
                        }
                        macaca_proto::ForkState::Merged => ("merged".to_string(), None, None),
                        macaca_proto::ForkState::Cancelled => ("cancelled".to_string(), None, None),
                    };

                    return Ok(macaca_tools::orchestration::TaskResultData {
                        status: status_str,
                        output,
                        error,
                    });
                }

                let task_id_uuid = uuid::Uuid::parse_str(&task_or_fork_id)
                    .map_err(|e| format!("Invalid task_id: {}", e))?;
                let task_id = macaca_kernel::TaskId(task_id_uuid);

                let status = executor
                    .get_task_status(&task_id)
                    .await
                    .ok_or_else(|| format!("Task '{}' not found", task_id))?;

                let (status_str, output, error) = match status {
                    macaca_kernel::TaskStatus::Queued => ("queued".to_string(), None, None),
                    macaca_kernel::TaskStatus::Running => ("running".to_string(), None, None),
                    macaca_kernel::TaskStatus::Completed => {
                        if let Some(result) = executor.get_task_result(task_id).await {
                            ("completed".to_string(), Some(result.output), result.error)
                        } else {
                            ("completed".to_string(), None, None)
                        }
                    }
                    macaca_kernel::TaskStatus::Failed => {
                        if let Some(result) = executor.get_task_result(task_id).await {
                            ("failed".to_string(), Some(result.output), result.error)
                        } else {
                            ("failed".to_string(), None, Some("Task failed".to_string()))
                        }
                    }
                    macaca_kernel::TaskStatus::Cancelled => ("cancelled".to_string(), None, None),
                };

                Ok(macaca_tools::TaskResultData {
                    status: status_str,
                    output,
                    error,
                })
            }
            .boxed()
        });
    all_tools.push(Box::new(get_result_tool));
    info!("GetTaskResult tool added");

    let tool_names: Vec<&str> = all_tools.iter().map(|t| t.name()).collect();
    info!(tools = ?tool_names, "Composite toolset ready");

    WebToolAssembly {
        tools: Arc::new(CompositeToolSet::new(all_tools)),
        executor_registry_ref,
        delegate_session_id,
    }
}
