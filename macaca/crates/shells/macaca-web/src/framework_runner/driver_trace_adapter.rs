//! Driver trace routing adapter for executor, runtime, and coordinator emit paths.

use super::build_mode::{should_forward_driver_trace, DriverTraceRoute};
use axum::response::sse::Event;
use macaca_sdk::framework::tool::{ToolTraceEvent, Toolkit};
use macaca_sdk::runtime_host::persist::{AppendEventCommand, EventLog};
use std::convert::Infallible;
pub(crate) async fn attach_driver_trace_route(toolkit: &mut Toolkit, route: DriverTraceRoute) {
    let (trace_tx, mut trace_rx) = tokio::sync::mpsc::unbounded_channel::<ToolTraceEvent>();
    toolkit.set_event_tx(trace_tx);

    tokio::spawn(async move {
        while let Some(trace) = trace_rx.recv().await {
            let trace = sdk_trace_from_framework(trace);
            if !should_forward_driver_trace(&trace) {
                tracing::debug!(
                    route = route.label(),
                    event_type = %trace.event_type,
                    tool_name = trace.tool_name.as_deref().unwrap_or(""),
                    correlation_id = trace.correlation_id.as_deref().unwrap_or(""),
                    "suppressed framework wrapper trace already represented by semantic tool event"
                );
                continue;
            }
            let driver_name = trace
                .driver_id
                .clone()
                .unwrap_or_else(|| "macaca-framework".to_string());
            let trace_value = serde_json::to_value(&trace).unwrap_or_default();

            match &route {
                DriverTraceRoute::Executor {
                    executor,
                    task_id,
                    agent_name,
                    ..
                } => {
                    // Executor routes publish through the executor broadcast channel only.
                    // post_chat_v2 owns the SSE forwarding for that channel; sending here
                    // as well would duplicate delegated_driver_trace events in the live UI.
                    executor.broadcast_event(
                        macaca_sdk::runtime_host::executor::ExecutorEvent::AgentEvent {
                            task_id: *task_id,
                            agent: agent_name.clone(),
                            event: macaca_proto::AgentExecutionEvent::DriverTrace {
                                driver_name: driver_name.clone(),
                                trace: trace_value.clone(),
                            },
                        },
                    );
                }
                DriverTraceRoute::Runtime { tx } => {
                    let _ = tx
                        .send(macaca_proto::AgentExecutionEvent::DriverTrace {
                            driver_name: driver_name.clone(),
                            trace: trace_value.clone(),
                        })
                        .await;
                }
                DriverTraceRoute::Coordinator {
                    tx,
                    event_log,
                    agent_name,
                    session_id,
                } => {
                    if let Some(sid) = session_id {
                        event_log
                            .append_command(AppendEventCommand::new(
                                sid,
                                "driver_trace",
                                agent_name,
                                trace_value.clone(),
                            ))
                            .await;
                    }
                    let event = Event::default().event("driver_trace").data(
                        serde_json::json!({
                            "driver_name": driver_name,
                            "event": trace_value,
                        })
                        .to_string(),
                    );
                    let _ = tx.send(Ok(event)).await;
                }
            }
        }
    });
}

fn sdk_trace_from_framework(event: ToolTraceEvent) -> macaca_sdk::tools::TraceEvent {
    macaca_sdk::tools::TraceEvent {
        event_type: event.event_type,
        driver_id: event.driver_id,
        timestamp: event.timestamp,
        correlation_id: event.correlation_id,
        title: event.title,
        content: event.content,
        tool_name: event.tool_name,
        tool_input: event.tool_input,
        tool_output: event.tool_output,
        is_error: event.is_error,
        metadata: event.metadata,
    }
}
