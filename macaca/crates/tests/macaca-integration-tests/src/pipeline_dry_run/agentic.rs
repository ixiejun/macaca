//! Traced agentic loop runner for pipeline dry-run stages.
//!
//! **Design pattern:** Observer — drains `AgentExecutionEvent` concurrently with
//! `AgenticLoop::execute_with_events` and forwards formatted lines to stderr when verbose.

use macaca_proto::error::MacacaResult;
use macaca_proto::types::{
    AgentExecutionEvent, AgentId, LlmMessage, LlmOptions, Permission,
};
use macaca_runtime::{AgenticLoop, LoopResult};
use tracing::debug;

use crate::scripted_llm::ScriptedLlm;

use super::config::PipelineDryRunConfig;
use super::store::LocalToolSet;
use super::trace::format_agent_event;

/// Run an agentic loop with optional verbose event tracing on stderr.
pub async fn run_agentic_traced(
    cfg: PipelineDryRunConfig,
    label: &str,
    loop_: &AgenticLoop,
    agent_id: &AgentId,
    llm: &ScriptedLlm,
    tools: &LocalToolSet,
    initial_messages: Vec<LlmMessage>,
    options: &LlmOptions,
    permission: &Permission,
) -> MacacaResult<LoopResult> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<AgentExecutionEvent>(64);

    let label = label.to_string();
    let drain = async move {
        while let Some(ev) = event_rx.recv().await {
            if cfg.verbose {
                eprintln!("  │ [{label}] {}", format_agent_event(&ev));
            }
            debug!(
                target: "pipeline_dry_run",
                stage = %label,
                event = %format_agent_event(&ev),
                "agentic_event"
            );
        }
    };

    let run_fut = loop_.execute_with_events(
        agent_id,
        llm,
        tools,
        initial_messages,
        options,
        permission,
        None,
        Some(event_tx),
    );

    let (run_res, ()) = tokio::join!(run_fut, drain);
    run_res
}
