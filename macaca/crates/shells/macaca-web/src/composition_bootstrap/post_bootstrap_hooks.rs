//! Bootstrap phases 10a–10d: executor registry, workspaces, agent services, hook consumer.
//!
//! Hot-swaps kernel execution onto `service.agent_execution`, registers started applications,
//! prepares per-app workspaces, and spawns the hook event consumer for fork-join auto-continue.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::info;

use macaca_proto::{ApplicationId, KernelServiceId, MacacaResult, TraceContext};
use macaca_runtime_host::AgentInfo;
use crate::agent_context_backend::WebAgentContextBackend;
use crate::hook_consumer;
use crate::loop_manager;
use crate::skill_self_evolution_execution_observer::SkillSelfEvolutionObservedAgentExecutionBackend;
use crate::web_agent_execution_adapters::build_composed_web_agent_execution_backend;

use super::bootstrap_ctx::BootstrapCtx;

/// Run the `post-bootstrap-hooks` bootstrap slice.
pub(crate) async fn run(ctx: &mut BootstrapCtx) -> MacacaResult<()> {
    let config = ctx.config.clone().expect("bootstrap: config");
    let kernel = Arc::clone(ctx.kernel.as_ref().expect("bootstrap: kernel"));
    let service_runtime = Arc::clone(ctx.service_runtime.as_ref().expect("bootstrap: service_runtime"));
    let state = Arc::clone(ctx.app_state.as_ref().expect("bootstrap: app_state"));
    let executor_registry_ref = Arc::clone(ctx.executor_registry_ref.as_ref().expect("bootstrap: executor_registry_ref"));
    let application_orchestration_registry_ref = Arc::clone(ctx.application_orchestration_registry_ref.as_ref().expect("bootstrap: application_orchestration_registry_ref"));
    let started_apps = ctx.started_apps.clone().expect("bootstrap: started_apps");

    // 10a. Set the executor registry reference for the delegate tool
    {
        let mut guard = executor_registry_ref.write().await;
        *guard = Some(Arc::clone(&state.executor_registry));
    }
    {
        let mut guard = application_orchestration_registry_ref.write().await;
        *guard = Some(Arc::clone(&state.executor_registry));
    }

    // Register the unified Agent Context and Agent Execution services after
    // `AppState` exists because Web is the composition root for personas,
    // skills, tools, workspaces, and executor event broadcasts.  Application
    // runtimes, including WASM guests, call these services instead of reaching
    // into Web executor internals directly.
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::agent_context_service_descriptor(),
                    Arc::new(macaca_runtime_host::AgentContextSystemServiceProvider::new(
                        Arc::new(WebAgentContextBackend::new(Arc::clone(&state))),
                    )),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_proto::AGENT_CONTEXT_SERVICE_ID),
            TraceContext::new("web-startup-agent-context-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::execution_control_service_descriptor(),
                    Arc::new(
                        macaca_runtime_host::ExecutionControlSystemServiceProvider::new(Arc::new(
                            macaca_runtime_host::ExecutionControlRuntimeCapability::new(),
                        )),
                    ),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_proto::EXECUTION_CONTROL_SERVICE_ID),
            TraceContext::new("web-startup-execution-control-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::agent_execution_service_descriptor(),
                    Arc::new(
                        macaca_runtime_host::AgentExecutionSystemServiceProvider::new(Arc::new(
                            SkillSelfEvolutionObservedAgentExecutionBackend::new(
                                Arc::new(build_composed_web_agent_execution_backend(
                                    Arc::clone(&state),
                                    Arc::clone(&service_runtime),
                                )),
                                Arc::clone(&state),
                            ),
                        )),
                    ),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_proto::AGENT_EXECUTION_SERVICE_ID),
            TraceContext::new("web-startup-agent-execution-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;

    // Hot-swap kernel execution onto the unified service path now that
    // `service.agent_execution` is registered and backed by ComposedAgentExecutionBackend.
    macaca_runtime_host::wire_kernel_to_agent_execution_service(
        kernel.as_ref(),
        Arc::clone(&service_runtime),
        ApplicationId::from_name("host-kernel-execution"),
    )
    .await;
    info!(
        service_id = macaca_proto::AGENT_EXECUTION_SERVICE_ID,
        "Kernel execution port wired to service.agent_execution"
    );


    // 10b. Register all started apps to the executor registry and create workspaces
    {
        let kernel_ref = Arc::clone(&kernel);
        let registry_ref = state.executor_registry.clone();
        let apps_to_register = started_apps.clone();
        let todo_store_for_recovery = Arc::clone(&state.persist.todo_store);
        let state_ref = Arc::clone(&state);

        tokio::spawn(async move {
            // Get all agents from kernel, then register each executor with
            // only the agents declared by that application.
            let all_agents = kernel_ref.list_agents().await;
            let agents_by_name: HashMap<_, _> =
                all_agents.iter().map(|m| (m.name.clone(), m)).collect();

            // Register each app to executor registry
            for (app_id, app_name, app_agent_names) in apps_to_register {
                let app_agents: Vec<AgentInfo> = app_agent_names
                    .iter()
                    .filter_map(|name| agents_by_name.get(name.as_str()).copied())
                    .map(|m| AgentInfo {
                        id: m.id.0.to_string(),
                        name: m.name.clone(),
                        capabilities: m.capabilities.iter().map(|c| c.name.clone()).collect(),
                        current_load: 0,
                        max_load: 4,
                        available: true,
                    })
                    .collect();
                let workspace_agent_names: Vec<String> =
                    app_agents.iter().map(|agent| agent.name.clone()).collect();

                // Workspace identity belongs to the generic application
                // platform, not to executor registration.  UI-only and
                // WASM-only applications still need a Macaca-registered
                // workspace so app-owned UI bridge calls can inject
                // `workspace_root` from `workspace.root_dir` instead of
                // accepting model- or caller-supplied host paths.
                match crate::app_workspace_bootstrap::prepare_app_workspace(
                    &config.workspace.root_dir,
                    &app_id,
                    &workspace_agent_names,
                ) {
                    Ok(workspace) => {
                        state_ref
                            .config
                            .app_workspaces
                            .write()
                            .await
                            .insert(app_id, workspace);
                    }
                    Err(e) => {
                        tracing::error!(
                            app_id = %app_id.0,
                            error = %e,
                            "Failed to prepare application workspace"
                        );
                    }
                }

                if app_agents.is_empty() {
                    // Security boundary enforcement:
                    // Never fall back to global agents when an application has no
                    // app-scoped runtime identity. Falling back would let one app
                    // execute through another app's agents/tools, which breaks
                    // isolation and violates the generic multi-tenant OS contract.
                    //
                    // We keep the app discoverable/listed at control-plane level,
                    // but skip executor registration so request handling cannot
                    // accidentally route into unrelated global workers.
                    tracing::warn!(
                        service_id = "application.executor",
                        command = "register",
                        reason_code = "no_app_scoped_agents",
                        application_id = %app_id.0,
                        "No app-scoped agents resolved; skipping executor registration to preserve app isolation"
                    );
                    continue;
                }

                // Register this app to executor registry
                let _executor = registry_ref
                    .register_application(app_id, app_name, app_agents.clone())
                    .await;
                tracing::info!(app_id = %app_id.0, "App registered to executor");

                // Recover crashed tasks: rollback InProgress/Assigned → Pending
                todo_store_for_recovery.rollback_in_progress(&app_id).await;

                // Auto-start PlanLoop and WorkerLoops for this app so pending
                // tasks (e.g., PendingReview from before restart) are processed.
                crate::loop_manager::ensure_plan_and_worker_loops(&state_ref, &app_id, None).await;
            }
        });
    }


    // 10c. Note: executor_registry is available in state for task delegation
    // The executor_registry allows agents to delegate tasks to other agents
    // using capability-based routing or direct agent targeting.
    // Route handlers can access it via state.executor_registry
    {
        let _registry = state.executor_registry.clone();
        info!("ApplicationExecutorRegistry initialized and apps registered");
    }


    // 10d. Start hook event consumer for coordinator auto-continue
    {
        let consumer_state = Arc::clone(&state);
        tokio::spawn(async move {
            hook_consumer::start_hook_event_consumer(consumer_state).await;
        });
        info!("Hook event consumer started");
    }

    Ok(())
}
