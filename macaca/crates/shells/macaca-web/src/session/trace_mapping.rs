//! Maps agent execution and driver events into persisted trace steps (Visitor pattern).
//!
//! `AgentTraceStepVisitor` implements `AgentExecutionEventVisitor` so SSE collectors and
//! session persistence share one canonical mapping from proto events to JSON-safe steps.

use macaca_proto::AgentExecutionEventVisitor;

use super::types::AgentTraceStep;

fn driver_trace_step(driver_name: Option<String>, trace: &serde_json::Value) -> AgentTraceStep {
    AgentTraceStep {
        step_type: "driver_trace".to_string(),
        event_type: trace
            .get("type")
            .or_else(|| trace.get("event_type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        content: trace
            .get("content")
            .or_else(|| trace.get("thinking"))
            .or_else(|| trace.get("text"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_name: trace
            .get("tool_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_input: trace.get("tool_input").cloned(),
        tool_output: trace
            .get("tool_output")
            .or_else(|| trace.get("tool_result"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        output: trace
            .get("output")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        is_error: trace.get("is_error").and_then(|v| v.as_bool()),
        driver_id: trace
            .get("driver_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        driver_name: driver_name.or_else(|| {
            trace
                .get("driver_name")
                .or_else(|| trace.get("driver_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }),
        title: trace
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        timestamp: trace.get("timestamp").and_then(|v| v.as_i64()),
        correlation_id: trace
            .get("correlation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        metadata: trace.get("metadata").cloned(),
        ..Default::default()
    }
}

pub(crate) fn delegated_driver_trace_step(payload: &serde_json::Value) -> AgentTraceStep {
    let event = payload
        .get("event")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if event.get("trace").is_some() {
        let driver_name = payload
            .get("driver_name")
            .or_else(|| event.get("driver_name"))
            .or_else(|| event.get("driver_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        return driver_trace_step(
            driver_name,
            event.get("trace").unwrap_or(&serde_json::Value::Null),
        );
    }
    let driver_name = payload
        .get("driver_name")
        .or_else(|| event.get("driver_name"))
        .or_else(|| event.get("driver_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    driver_trace_step(driver_name, &event)
}

struct AgentTraceStepVisitor;

impl AgentExecutionEventVisitor<AgentTraceStep> for AgentTraceStepVisitor {
    fn thinking(&mut self, iteration: usize, content: Option<&str>) -> AgentTraceStep {
        AgentTraceStep {
            step_type: "thinking".to_string(),
            iteration: Some(iteration),
            content: content.map(ToString::to_string),
            ..Default::default()
        }
    }

    fn tool_call(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        call_id: Option<&str>,
    ) -> AgentTraceStep {
        AgentTraceStep {
            step_type: "tool_call".to_string(),
            tool_name: Some(tool_name.to_string()),
            tool_input: Some(tool_input.clone()),
            call_id: call_id.map(ToString::to_string),
            ..Default::default()
        }
    }

    fn tool_result(
        &mut self,
        tool_name: &str,
        output: &str,
        is_error: Option<bool>,
    ) -> AgentTraceStep {
        crate::metrics::record_tool_execution(tool_name, !is_error.unwrap_or(false));
        AgentTraceStep {
            step_type: "tool_result".to_string(),
            tool_name: Some(tool_name.to_string()),
            output: Some(output.to_string()),
            is_error,
            ..Default::default()
        }
    }

    fn assistant(&mut self, content: &str) -> AgentTraceStep {
        AgentTraceStep {
            step_type: "assistant".to_string(),
            content: Some(content.to_string()),
            ..Default::default()
        }
    }

    fn driver_trace(&mut self, driver_name: &str, trace: &serde_json::Value) -> AgentTraceStep {
        driver_trace_step(Some(driver_name.to_string()), trace)
    }

    fn completed(&mut self, success: bool, error: Option<&str>) -> AgentTraceStep {
        AgentTraceStep {
            step_type: "completed".to_string(),
            success: Some(success),
            error: error.map(ToString::to_string),
            ..Default::default()
        }
    }
}

pub(crate) fn trace_step_from_agent_event(
    event: &macaca_proto::AgentExecutionEvent,
) -> AgentTraceStep {
    let mut visitor = AgentTraceStepVisitor;
    event.accept(&mut visitor)
}
