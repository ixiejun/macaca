//! Host-owned bootstrap helper for local built-in tools.
//!
//! Presentation shells need a generic tool vector during startup, but they
//! should not construct concrete OS tool providers directly. This module keeps
//! local tool construction in runtime-host, which is the composition root for
//! host-owned service/provider objects.

use std::sync::Arc;

use futures::{future::BoxFuture, FutureExt};
use macaca_kernel::Kernel;
use macaca_proto::{AcceptanceCriteria, ApplicationId, ForkId, ForkState, LlmRole, TraceContext};
use macaca_tools::{
    DelegateTaskTool, FileReadTool, FileWriteTool, GetTaskResultTool, ListAgentsTool, ShellTool,
    TaskResultData, Tool,
};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    ApplicationExecutorRegistry, DelegateViaAgentServiceRequest,
    ExecutionControlForkJoinCoordinator, ForkJoinChildForkRequest, ForkJoinParentWaitRequest,
    ServiceDelegatedTaskDispatcher, ServiceRuntime, TaskContext, TaskId, TaskStatus,
};

/// Generic delegator label used when the orchestration tool does not receive an explicit caller.
///
/// The value is intentionally role-neutral. It gives service traces a stable
/// initiator label without encoding application persona names into OS code.
const GENERIC_DELEGATOR_AGENT: &str = "delegator";

/// Host callback used to let a shell record fork-to-session wake metadata.
///
/// Runtime-host owns fork creation, service-backed delegation, and execution
/// control registration. The shell remains the owner of presentation wake
/// delivery, so it supplies this narrow writer port instead of exposing its
/// concrete mapping type to runtime-host.
pub type ForkSessionMappingRecorder = Arc<
    dyn Fn(ForkId, ApplicationId, String, String) -> BoxFuture<'static, Result<(), String>>
        + Send
        + Sync,
>;

/// Ports required to bootstrap the service-backed `delegate_task` tool.
pub struct DelegateTaskToolBootstrapPorts {
    /// Late-bound executor registry populated after application execution startup.
    pub executor_registry_ref: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
    /// Shared session id slot updated by request handling before tool invocation.
    pub delegate_session_id: Arc<RwLock<Option<String>>>,
    /// Shell-owned writer for fork-to-session wake metadata.
    pub fork_session_recorder: ForkSessionMappingRecorder,
}

/// Build the default local tool set owned by the runtime host.
///
/// The helper intentionally returns only trait objects. This applies a small
/// Abstract Factory boundary: shells can compose the returned tools into their
/// toolkit surface, while runtime-host remains the owner of concrete tool
/// selection and construction.
pub fn bootstrap_local_base_tools(trace_label: impl Into<String>) -> Vec<Box<dyn Tool>> {
    let trace_label = trace_label.into();
    tracing::info!(
        trace_label = %trace_label,
        "runtime-host local base tool bootstrap requested"
    );

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(FileReadTool),
        Box::new(FileWriteTool),
        Box::new(ShellTool::default()),
    ];

    tracing::info!(
        count = tools.len(),
        trace_label = %trace_label,
        "runtime-host local base tool bootstrap completed"
    );
    tools
}

/// Build the host-owned `list_agents` tool from the shared kernel registry.
///
/// Shells receive a generic [`Tool`] object and do not need to construct the
/// concrete orchestration tool or translate kernel agent records themselves.
/// The callback is intentionally read-only and returns sanitized agent names
/// plus capability names for operator/tool planning visibility.
pub fn bootstrap_list_agents_tool(
    kernel: Arc<Kernel>,
    trace_label: impl Into<String>,
) -> Box<dyn Tool> {
    let trace_label = trace_label.into();
    tracing::info!(
        trace_label = %trace_label,
        "runtime-host list_agents tool bootstrap requested"
    );

    let tool = ListAgentsTool::new().with_agents_callback(move || {
        let kernel = Arc::clone(&kernel);
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

    tracing::info!(
        trace_label = %trace_label,
        tool = tool.name(),
        "runtime-host list_agents tool bootstrap completed"
    );
    Box::new(tool)
}

/// Build the host-owned `delegate_task` orchestration tool.
///
/// Runtime-host owns the Abstract Factory for the concrete tool and the
/// Strategy objects that translate a tool call into fork/join plus
/// `service.agent_execution` traffic. The only shell-specific behavior is the
/// optional wake mapping write, which is represented as a narrow callback port.
pub fn bootstrap_delegate_task_tool(
    service_runtime: Arc<ServiceRuntime>,
    ports: DelegateTaskToolBootstrapPorts,
    trace_label: impl Into<String>,
) -> Box<dyn Tool> {
    let trace_label = trace_label.into();
    info!(
        trace_label = %trace_label,
        "runtime-host delegate_task tool bootstrap requested"
    );

    let delegated_task_dispatcher = Arc::new(ServiceDelegatedTaskDispatcher::new(Arc::clone(
        &service_runtime,
    )));
    let fork_join_coordinator = Arc::new(ExecutionControlForkJoinCoordinator::new(service_runtime));
    let registry_for_delegate = Arc::clone(&ports.executor_registry_ref);
    let dispatcher_for_delegate = Arc::clone(&delegated_task_dispatcher);
    let fork_join_for_delegate = Arc::clone(&fork_join_coordinator);
    let fork_session_recorder = Arc::clone(&ports.fork_session_recorder);

    let tool = DelegateTaskTool::empty_with_session_id(Arc::clone(&ports.delegate_session_id))
        .with_callback(
            move |app_id, to_agent, prompt, priority, parallel, session_id| {
                let registry = Arc::clone(&registry_for_delegate);
                let dispatcher = Arc::clone(&dispatcher_for_delegate);
                let fork_join = Arc::clone(&fork_join_for_delegate);
                let fork_session_recorder = Arc::clone(&fork_session_recorder);
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
                            .map(ApplicationId)
                            .map_err(|e| format!("Invalid application ID: {}", e))?
                    };

                    let executor = registry
                        .get(&app_id)
                        .await
                        .ok_or_else(|| format!("App '{}' not found in registry", app_id))?;

                    let fork_manager = executor.fork_manager();
                    let acceptance_criteria = AcceptanceCriteria {
                        description: format!(
                            "Task delegated to {}: {}",
                            to_agent,
                            prompt.chars().take(100).collect::<String>()
                        ),
                        required_artifacts: vec![],
                        auto_accept: false,
                    };

                    let fork_id = ExecutionControlForkJoinCoordinator::create_and_start_child_fork(
                        &fork_manager,
                        ForkJoinChildForkRequest {
                            application_id: app_id.clone(),
                            target_agent: to_agent.clone(),
                            task_prompt: prompt.clone(),
                            inherited_messages: vec![],
                            system_prompt: String::new(),
                            acceptance_criteria,
                        },
                    )
                    .await
                    .map_err(|e| format!("Fork creation failed: {}", e))?;

                    let task_context = session_id.clone().map(|sid| TaskContext {
                        session_id: Some(sid),
                        artifacts: vec![],
                        env: std::collections::HashMap::new(),
                    });
                    let task_id = dispatcher
                        .dispatch(
                            executor.clone(),
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

                    ExecutionControlForkJoinCoordinator::suspend_child_fork(
                        &fork_manager,
                        fork_id,
                        macaca_proto::TaskId(task_id.0),
                    )
                    .await
                    .map_err(|e| format!("Fork suspend failed: {}", e))?;

                    if let Some(parent_session_id) = session_id {
                        let trace = TraceContext::new(format!(
                            "fork-join:{}:{}:{}",
                            app_id.0, fork_id.0, task_id.0
                        ));
                        if let Err(error) = fork_join
                            .register_parent_fork_wait(ForkJoinParentWaitRequest {
                                application_id: app_id.clone(),
                                session_id: parent_session_id.clone(),
                                parent_agent: GENERIC_DELEGATOR_AGENT.to_string(),
                                fork_id,
                                delegate_task_id: macaca_proto::TaskId(task_id.0),
                                trace,
                            })
                            .await
                        {
                            warn!(
                                fork_id = %fork_id.0,
                                task_id = %task_id.0,
                                error = %error,
                                "Fork-join parent pause registration via execution control failed"
                            );
                        }

                        fork_session_recorder(
                            fork_id,
                            app_id,
                            GENERIC_DELEGATOR_AGENT.to_string(),
                            parent_session_id,
                        )
                        .await?;
                        info!(
                            fork_id = %fork_id.0,
                            task_id = %task_id.0,
                            "Recorded fork-to-session mapping through shell writer port"
                        );
                    }

                    Ok(format!("fork:{}", fork_id))
                }
                .boxed()
            },
        );

    info!(
        trace_label = %trace_label,
        tool = tool.name(),
        "runtime-host delegate_task tool bootstrap completed"
    );
    Box::new(tool)
}

/// Build the host-owned `get_task_result` orchestration tool.
///
/// The tool is read-only. It resolves either a fork id (`fork:<uuid>`) or a
/// task id through the runtime-host executor registry and returns the stable
/// `TaskResultData` DTO expected by existing tool callers.
pub fn bootstrap_get_task_result_tool(
    executor_registry_ref: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
    trace_label: impl Into<String>,
) -> Box<dyn Tool> {
    let trace_label = trace_label.into();
    tracing::info!(
        trace_label = %trace_label,
        "runtime-host get_task_result tool bootstrap requested"
    );

    let tool = GetTaskResultTool::empty().with_callback(move |app_id, task_or_fork_id| {
        let registry = Arc::clone(&executor_registry_ref);
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
                let fork_id = ForkId(fork_id_uuid);

                let fork_manager = executor.fork_manager();
                let fork = ExecutionControlForkJoinCoordinator::get_fork(&fork_manager, fork_id)
                    .await
                    .ok_or_else(|| format!("Fork '{}' not found", fork_id))?;

                let (status, output, error) = match fork.state {
                    ForkState::Pending => ("pending".to_string(), None, None),
                    ForkState::Running => ("running".to_string(), None, None),
                    ForkState::WaitingForHook => ("waiting".to_string(), None, None),
                    ForkState::Completed => {
                        let output = fork
                            .own_messages
                            .iter()
                            .filter_map(|message| {
                                if message.role == LlmRole::Assistant {
                                    Some(message.content.clone())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        ("completed".to_string(), Some(output), None)
                    }
                    ForkState::Failed { ref error } => {
                        ("failed".to_string(), None, Some(error.clone()))
                    }
                    ForkState::Merged => ("merged".to_string(), None, None),
                    ForkState::Cancelled => ("cancelled".to_string(), None, None),
                };

                return Ok(TaskResultData {
                    status,
                    output,
                    error,
                });
            }

            let task_id_uuid = uuid::Uuid::parse_str(&task_or_fork_id)
                .map_err(|e| format!("Invalid task_id: {}", e))?;
            let task_id = TaskId(task_id_uuid);

            let status = executor
                .get_task_status(&task_id)
                .await
                .ok_or_else(|| format!("Task '{}' not found", task_id))?;

            let (status, output, error) = match status {
                TaskStatus::Queued => ("queued".to_string(), None, None),
                TaskStatus::Running => ("running".to_string(), None, None),
                TaskStatus::Completed => {
                    if let Some(result) = executor.get_task_result(task_id).await {
                        ("completed".to_string(), Some(result.output), result.error)
                    } else {
                        ("completed".to_string(), None, None)
                    }
                }
                TaskStatus::Failed => {
                    if let Some(result) = executor.get_task_result(task_id).await {
                        ("failed".to_string(), Some(result.output), result.error)
                    } else {
                        ("failed".to_string(), None, Some("Task failed".to_string()))
                    }
                }
                TaskStatus::Cancelled => ("cancelled".to_string(), None, None),
            };

            Ok(TaskResultData {
                status,
                output,
                error,
            })
        }
        .boxed()
    });

    tracing::info!(
        trace_label = %trace_label,
        tool = tool.name(),
        "runtime-host get_task_result tool bootstrap completed"
    );
    Box::new(tool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_local_base_tools_returns_default_tool_family() {
        let tools = bootstrap_local_base_tools("local-base-tool-bootstrap-test");

        let names: Vec<_> = tools.iter().map(|tool| tool.name()).collect();
        assert_eq!(names, vec!["file_read", "file_write", "shell"]);
    }

    #[tokio::test]
    async fn bootstrap_list_agents_tool_returns_orchestration_tool() {
        let kernel = Arc::new(
            macaca_kernel::KernelBuilder::from_execution_port(
                macaca_proto::config::KernelConfig {
                    max_agents: 4,
                    heartbeat_interval_ms: 1_000,
                    agent_timeout_ms: 5_000,
                },
                Arc::new(macaca_kernel::UnavailableAgentExecutionPort::new(
                    "list_agents bootstrap test",
                )),
            )
            .build(),
        );
        let tool = bootstrap_list_agents_tool(kernel, "list-agents-tool-bootstrap-test");

        assert_eq!(tool.name(), "list_agents");
    }

    #[test]
    fn bootstrap_get_task_result_tool_returns_orchestration_tool() {
        let registry_ref = Arc::new(RwLock::new(None));
        let tool = bootstrap_get_task_result_tool(registry_ref, "get-task-result-bootstrap-test");

        assert_eq!(tool.name(), "get_task_result");
    }
}
