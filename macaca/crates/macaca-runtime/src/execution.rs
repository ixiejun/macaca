//! Tool command execution boundary for runtime loops.

use std::time::Duration;

use macaca_proto::{AgentExecutionEvent, AgentId, MacacaError, MacacaResult, Permission, ToolCall};
use macaca_tools::{ToolCatalog, TraceEvent};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::permission::PermissionChecker;

/// Executes a tool call through the runtime command boundary.
pub(crate) async fn execute_tool_call(
    agent_id: &AgentId,
    tools: &dyn ToolCatalog,
    tool_call: &ToolCall,
    permission: &Permission,
    permission_checker: Option<&dyn PermissionChecker>,
    timeout_duration: Duration,
    event_tx: Option<&mpsc::Sender<AgentExecutionEvent>>,
) -> MacacaResult<serde_json::Value> {
    let tool_name = &tool_call.name;
    let args_preview = serde_json::to_string(&tool_call.arguments)
        .map(|s| s.chars().take(200).collect::<String>())
        .unwrap_or_else(|_| "<serialize error>".to_string());

    if event_tx.is_some() {
        info!(
            agent_id = %agent_id,
            tool_name = %tool_name,
            args_preview = %args_preview,
            "[TOOL] Executing tool"
        );
    }

    if let Some(checker) = permission_checker {
        checker.check_tool_with_args(
            agent_id,
            permission,
            &tool_call.name,
            &tool_call.arguments,
        )?;
    }

    let tool = macaca_tools::ToolCatalog::find_tool(tools, &tool_call.name).ok_or_else(|| {
        if event_tx.is_some() {
            error!(
                agent_id = %agent_id,
                tool_name = %tool_name,
                "[TOOL] Tool not found"
            );
        }
        MacacaError::NotFound(format!("Tool '{}' not found", tool_call.name))
    })?;

    let (trace_tx, mut trace_rx) = if event_tx.is_some() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<TraceEvent>();
        (Some(tx), rx)
    } else {
        let (_, rx) = tokio::sync::mpsc::unbounded_channel::<TraceEvent>();
        (None, rx)
    };

    let tool_name = tool_call.name.clone();
    let forward_task = if let Some(tx) = event_tx.cloned() {
        Some(tokio::spawn(async move {
            while let Some(trace) = trace_rx.recv().await {
                let driver_name = trace
                    .driver_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                let trace_value = serde_json::to_value(&trace).unwrap_or_default();
                let driver_trace = AgentExecutionEvent::DriverTrace {
                    driver_name,
                    trace: trace_value,
                };
                if tx.send(driver_trace).await.is_err() {
                    break;
                }
            }
        }))
    } else {
        None
    };

    let result = tokio::time::timeout(
        timeout_duration,
        execute_tool_command(
            tool,
            tool_call.arguments.clone(),
            macaca_tools::ToolCommandContext {
                event_tx: trace_tx,
                ..Default::default()
            },
        ),
    )
    .await
    .map_err(|_| {
        if event_tx.is_some() {
            error!(
                agent_id = %agent_id,
                tool_name = %tool_name,
                timeout_secs = timeout_duration.as_secs(),
                "[TOOL] Tool execution timed out"
            );
        }
        MacacaError::Timeout(format!(
            "Tool '{}' timed out after {}s",
            tool_name,
            timeout_duration.as_secs()
        ))
    })??;

    if let Some(task) = forward_task {
        let _ = task.await;
    }

    if event_tx.is_some() {
        let result_preview = serde_json::to_string(&result)
            .map(|s| s.chars().take(200).collect::<String>())
            .unwrap_or_else(|_| "<serialize error>".to_string());

        info!(
            agent_id = %agent_id,
            tool_name = %tool_name,
            result_preview = %result_preview,
            "[TOOL] Tool execution completed"
        );
    }

    Ok(result)
}

async fn execute_tool_command(
    tool: &dyn macaca_tools::Tool,
    input: serde_json::Value,
    context: macaca_tools::ToolCommandContext,
) -> MacacaResult<serde_json::Value> {
    macaca_tools::ToolCommandExecutor::execute_command(
        tool,
        macaca_tools::ToolCommand::with_context(input, context),
    )
    .await
}
