//! Coordinator-specific factory build path with SSE and execution-control middleware.

use std::sync::Arc;
use macaca_framework::agent::{HookRegistry, HookedAgent};
use macaca_framework::construction::AgentBuildRequest;
use macaca_framework::react_agent::ReActAgent;
use super::agent_factory_build::{PreparedAgentParts, WebTracedAgentFactory};
use super::build_mode::{DriverTraceRoute, FrameworkRunnerBuildMode};
use super::driver_trace_adapter::attach_driver_trace_route;
use super::execution_control_middleware::ExecutionControlMiddleware;
use super::sse_emitter_adapter::{SseEmitterHook, SseToolMiddleware};
use super::FrameworkRunner;

pub(crate) async fn build_coordinator_agent(
    factory: WebTracedAgentFactory,
    request: AgentBuildRequest,
) -> Result<(HookedAgent<ReActAgent>, tokio_util::sync::CancellationToken), String> {
    let llm_client = Arc::clone(&factory.state.llm_client);
    let context_client = Arc::clone(&factory.state.context_client);
    let PreparedAgentParts {
        selection,
        mut toolkit,
    } = factory.prepare_agent_parts(&request, Some(None)).await?;

    let FrameworkRunnerBuildMode::Coordinator {
        sse_tx,
        execution_control,
    } = factory.build_mode
    else {
        return Err("invalid build mode for coordinator".into());
    };

    attach_driver_trace_route(
        &mut toolkit,
        DriverTraceRoute::Coordinator {
            tx: sse_tx.clone(),
            event_log: Arc::clone(&factory.state.persist.event_log),
            agent_name: request.identity.agent_name.clone(),
            session_id: request.identity.session_id.clone(),
        },
    )
    .await;

    toolkit.add_middleware(Box::new(SseToolMiddleware {
        tx: sse_tx.clone(),
        agent_name: request.identity.agent_name.clone(),
        event_log: Some(Arc::clone(&factory.state.persist.event_log)),
        session_id: request.identity.session_id.clone(),
    }));
    toolkit.add_middleware(Box::new(ExecutionControlMiddleware {
        pause_signal: Arc::clone(&execution_control.pause_signal),
        resume_rx: Arc::clone(&execution_control.resume_rx),
        policy: execution_control.policy.clone(),
        execution_id: execution_control.execution_id.clone(),
    }));

    let merged_ctx = FrameworkRunner::resolve_context_config(
        &factory.state,
        &request.identity.app_id,
        &request.identity.agent_name,
    )
    .await;
    let profile_root = FrameworkRunner::resolve_agent_profile_root(
        &factory.state,
        &request.identity.app_id,
        &request.identity.agent_name,
        &merged_ctx.agent_profile,
    )
    .await;
    let routing_agent_id = factory
        .state
        .kernel
        .get_agent_by_name(&request.identity.agent_name)
        .await
        .map(|m| m.id);
    let (
        skill_capability_catalog,
        mcp_capability_catalog,
        runtime_tool_capability_catalog,
        ready_mcp_server_ids,
    ) = WebTracedAgentFactory::resolve_framework_capability_catalogs(
        &factory.state,
        &request,
        &toolkit,
        &merged_ctx,
    )
    .await;

    let agent = WebTracedAgentFactory::build_react_agent(
        llm_client,
        context_client,
        Arc::clone(&factory.state.memory_client),
        Arc::clone(&factory.state.persist.event_log),
        Arc::clone(&factory.state.persist.session_store),
        factory.state.workspace_memory_tombstones.clone(),
        merged_ctx,
        profile_root,
        &request,
        &selection,
        toolkit,
        50,
        None,
        routing_agent_id,
        skill_capability_catalog,
        mcp_capability_catalog,
        runtime_tool_capability_catalog,
        ready_mcp_server_ids,
        Some(Arc::clone(&factory.state.provider_health_ledger)),
        Arc::clone(&factory.state.context_engine_registry),
    );

    let cancel_token = agent.cancel_token();
    let mut hooks = HookRegistry::new();
    hooks.register_instance_hook(Box::new(SseEmitterHook {
        tx: sse_tx,
        agent_name: request.identity.agent_name,
        event_log: Some(Arc::clone(&factory.state.persist.event_log)),
        session_id: request.identity.session_id,
    }));
    Ok((HookedAgent::new(agent, hooks), cancel_token))
}
