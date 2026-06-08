//! Verbose stderr tracing and agent-event formatting for dry-run diagnostics.
//!
//! **Design pattern:** Observer — stages emit trace lines; verbose mode subscribes and
//! renders them to stderr. Structured `tracing` logs mirror key nodes for audit tooling.

use macaca_proto::types::AgentExecutionEvent;
use tracing::{debug, info};

use super::config::PipelineDryRunConfig;

/// Emit a single trace line when verbose mode is enabled.
pub fn trace(cfg: PipelineDryRunConfig, msg: impl AsRef<str>) {
    let text = msg.as_ref();
    if cfg.verbose {
        eprintln!("{text}");
    }
    debug!(target: "pipeline_dry_run", "{text}");
}

/// Print a stage boundary header when verbose mode is enabled.
pub fn trace_stage(cfg: PipelineDryRunConfig, title: &str) {
    if cfg.verbose {
        eprintln!();
        eprintln!("┌── {title} ──");
    }
    info!(target: "pipeline_dry_run", stage = title, "stage_start");
}

/// Print a stage boundary footer when verbose mode is enabled.
pub fn trace_stage_end(cfg: PipelineDryRunConfig, ok: bool, detail: Option<&str>) {
    if cfg.verbose {
        let status = if ok { "OK" } else { "FAIL" };
        if let Some(d) = detail {
            eprintln!("└── {status} — {d}");
        } else {
            eprintln!("└── {status}");
        }
    }
    info!(
        target: "pipeline_dry_run",
        ok,
        detail = detail.unwrap_or(""),
        "stage_end"
    );
}

/// Format an [`AgentExecutionEvent`] for human-readable stderr output.
pub fn format_agent_event(ev: &AgentExecutionEvent) -> String {
    match ev {
        AgentExecutionEvent::Thinking { iteration, content } => {
            if let Some(c) = content {
                format!("iteration {iteration} | thinking | {c}")
            } else {
                format!("iteration {iteration} | thinking | (start LLM round)")
            }
        }
        AgentExecutionEvent::ToolCall {
            tool_name,
            tool_input,
            call_id,
        } => {
            let args = serde_json::to_string(tool_input).unwrap_or_else(|_| tool_input.to_string());
            let id = call_id.as_deref().unwrap_or("-");
            format!("tool_call | {tool_name} | id={id} | args={args}")
        }
        AgentExecutionEvent::ToolResult {
            tool_name,
            output,
            is_error,
        } => {
            let err = is_error.unwrap_or(false);
            let out = if output.len() > 240 {
                format!("{}…", &output[..240])
            } else {
                output.clone()
            };
            format!("tool_result | {tool_name} | error={err} | {out}")
        }
        AgentExecutionEvent::Assistant { content } => {
            format!(
                "assistant_text | {}",
                content.chars().take(120).collect::<String>()
            )
        }
        AgentExecutionEvent::DriverTrace { driver_name, .. } => {
            format!("driver_trace | {driver_name} | {ev:?}")
        }
        AgentExecutionEvent::Completed { success, error } => {
            format!("completed | success={success} | error={error:?}")
        }
    }
}
