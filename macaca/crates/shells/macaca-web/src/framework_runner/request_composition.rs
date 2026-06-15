//! Agent build request composition from persona, capabilities, and application semantics.

use super::context_prompt_builder;
use super::FrameworkRunner;
use crate::state::AppState;
use macaca_host_composition::app::app_agent_prompt_semantics;
use macaca_host_composition::framework::construction::*;
use macaca_proto::ApplicationId;
use std::sync::Arc;

pub(crate) async fn build_request(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<String>,
    task_id: macaca_proto::TaskId,
    goal_id: Option<macaca_proto::TaskId>,
    intent: AgentBuildIntent,
    tools: AgentToolConfig,
) -> Result<AgentBuildRequest, String> {
    let capabilities =
        FrameworkRunner::resolve_agent_capability_set(state, app_id, agent_name).await;
    let system_prompt = context_prompt_builder::build_context_system_prompt(
        state,
        app_id,
        agent_name,
        session_id.clone(),
        &capabilities,
    )
    .await;
    build_request_with_system_prompt(
        state,
        app_id,
        agent_name,
        session_id,
        task_id,
        goal_id,
        intent,
        tools,
        capabilities,
        system_prompt,
    )
    .await
}

pub(crate) async fn build_request_with_system_prompt(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<String>,
    task_id: macaca_proto::TaskId,
    goal_id: Option<macaca_proto::TaskId>,
    intent: AgentBuildIntent,
    tools: AgentToolConfig,
    capabilities: AgentCapabilitySet,
    system_prompt: String,
) -> Result<AgentBuildRequest, String> {
    let app_manifest = {
        let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
        registry.get_app(app_id).map(|app| app.manifest.clone())
    };
    let application = app_manifest.as_ref().map(|manifest| {
        let semantics = app_agent_prompt_semantics(manifest, agent_name);
        ApplicationSemantics {
            workflow_name: Some(semantics.workflow_name),
            entry_agent: Some(semantics.entry_agent),
            prompt_parts: semantics.prompt_parts.map(|parts| ApplicationPromptParts {
                role: parts.role,
                constraints: parts.constraints,
                tools: parts.tools,
                handoff: parts.handoff,
            }),
            tool_policy: ApplicationToolPolicy {
                allowed_tool_names: semantics.tool_policy.allowed_tools,
                execution_tool_names: semantics.tool_policy.execution_tools,
                is_entry_agent: semantics.tool_policy.is_entry_agent,
            },
        }
    });

    AgentBuildRequestBuilder::new(
        AgentIdentity {
            app_id: *app_id,
            agent_name: agent_name.to_string(),
            session_id: session_id.clone(),
        },
        intent,
    )
    .system_prompt(system_prompt)
    .services(AgentServices::default())
    .capabilities(capabilities)
    .lifecycle(AgentLifecycleConfig::default())
    .trace(AgentTraceContext {
        session_id,
        task_id: Some(task_id),
        source_agent: agent_name.to_string(),
    })
    .tools(AgentToolConfig { goal_id, ..tools })
    .application_opt(application)
    .build()
}
