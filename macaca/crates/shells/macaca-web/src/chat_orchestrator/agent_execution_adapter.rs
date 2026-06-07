//! Agent execution service adapter for chat main-thread turns.
//!
//! Routes visible chat entry execution through `service.agent_execution` instead
//! of constructing framework agents inside the web shell.

use std::sync::Arc;

use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, AgentExecutionStatus,
    ApplicationId, KernelServiceId, ServiceBusSource, TraceContext, AGENT_EXECUTION_SERVICE_ID,
};

use crate::state::AppState;

/// Execute the visible chat entry agent through `service.agent_execution`.
///
/// Chat orchestration owns browser session setup, SSE stream lifetime, and
/// session persistence.  The service owns trusted context construction and
/// model/tool runtime execution, so the Web shell no longer builds a separate
/// main-thread agent for non-WASM chat sessions.
pub(crate) async fn run_chat_main_thread_via_agent_service(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &str,
    entry_agent_name: &str,
    prompt: String,
) -> Result<String, String> {
    let mut trace = TraceContext::new(format!(
        "chat-main-thread:{}:{}:{}",
        app_id.0, entry_agent_name, session_id
    ));
    trace.session_id = Some(session_id.to_string());
    trace.agent = Some(entry_agent_name.to_string());

    let mut command = AgentExecutionCommand::new(
        *app_id,
        session_id.to_string(),
        entry_agent_name,
        AgentExecutionIntent::ChatMainThread,
        prompt,
        trace,
    )
    .map_err(|error| error.to_string())?;
    command
        .metadata
        .insert("entrypoint".into(), "macaca.web.chat_orchestrator".into());

    let reply = state
        .service_runtime
        .call(
            &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
            ServiceBusSource::new("macaca.web.chat_orchestrator"),
            command
                .into_service_command()
                .map_err(|error| error.to_string())?,
        )
        .await
        .map_err(|error| error.to_string())?;
    let output = reply
        .output
        .ok_or_else(|| "agent execution service returned no chat output".to_string())?;
    let result: AgentExecutionResult =
        serde_json::from_value(output).map_err(|error| error.to_string())?;

    match result.status {
        AgentExecutionStatus::Completed => Ok(result
            .output
            .get("output")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| result.output.to_string())),
        status => Err(result
            .output
            .get("error")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("agent execution service returned {}", status.as_str()))),
    }
}
