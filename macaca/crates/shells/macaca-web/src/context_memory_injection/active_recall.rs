//! Active recall injection — dynamic, request-only memory context source.
//!
//! Runs a bounded Memory Service prefetch and injects excerpt rows into the
//! assembled prompt with full active-recall diagnostics on the context report.

use std::sync::Arc;

use macaca_host_composition::context::{
    ActiveRecallDiagnostics, ContextDecisionReport, ContextDecisionSeverity,
    ContextPreflightRecallConfig,
};
use macaca_host_composition::memory::{MemoryPolicyHints, MemoryPrefetchCommand, MemoryScope};
use macaca_proto::LlmMessage;
use tokio::time::timeout;

use crate::context_memory_injection::adapter::{
    insert_after_leading_system, memory_trace, request_memory_source_report,
};
use crate::context_message_codec::last_user_text_from_framework;

/// Run active recall as a dynamic, request-only context source.
///
/// Skips work when composer-stage recall already ran (`composer_recall_active`).
pub(crate) async fn apply_active_recall(
    recall_runtime: &macaca_proto::config::ContextRecallRuntimeConfig,
    memory_client: &Arc<dyn macaca_sdk::SystemMemoryClient>,
    scope: MemoryScope,
    preflight_cfg: &ContextPreflightRecallConfig,
    composer_recall_active: bool,
    assembled: &mut macaca_host_composition::context::ContextAssembleResult,
    incoming_framework_messages: &[serde_json::Value],
) {
    if composer_recall_active {
        return;
    }
    if !preflight_cfg.enabled {
        return;
    }
    let Some(query) = last_user_text_from_framework(incoming_framework_messages) else {
        return;
    };
    if query.trim().is_empty() {
        return;
    }

    let started = std::time::Instant::now();
    let limit = recall_runtime.memory_search_default_limit.clamp(1, 16) as usize;
    let trace = memory_trace(
        scope.identity.session_id.as_deref(),
        scope.identity.agent_name.as_deref(),
    );
    tracing::info!(
        trace_id = %trace.trace_id,
        command = "context.memory.active_recall",
        limit,
        "starting active memory recall prefetch"
    );
    let command = MemoryPrefetchCommand {
        scope,
        trace,
        query: query.to_owned(),
        limit,
        policy: MemoryPolicyHints::default(),
    };
    let recall = timeout(preflight_cfg.timeout(), memory_client.prefetch(command)).await;

    let result = match recall {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            tracing::warn!(
                command = "context.memory.active_recall",
                error = %error,
                "active memory recall failed"
            );
            assembled.report.decisions.push(ContextDecisionReport {
                code: "active_recall_degraded".into(),
                severity: ContextDecisionSeverity::Warning,
                message: format!("active memory recall failed: {error}"),
            });
            return;
        }
        Err(_) => {
            tracing::warn!(
                command = "context.memory.active_recall",
                "active memory recall timed out"
            );
            assembled.report.decisions.push(ContextDecisionReport {
                code: "active_recall_degraded".into(),
                severity: ContextDecisionSeverity::Warning,
                message: "active memory recall timed out".into(),
            });
            return;
        }
    };

    if result.entries.is_empty() {
        assembled.report.decisions.push(ContextDecisionReport::info(
            "active_recall_empty",
            "active memory recall returned no rows",
        ));
        return;
    }

    let mut rendered =
        String::from("[Active memory recall - reference only, verify before acting]\n");
    let mut source_breakdown = Vec::new();
    let mut selected = 0usize;
    let mut used_chars = 0usize;
    let mut used_tokens = 0u32;
    for entry in result.entries.iter().take(limit) {
        let candidate = format!(
            "[{}] memory_id={} agent_id={}\n{}",
            selected + 1,
            entry.id.0,
            entry
                .agent_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "none".into()),
            entry.content
        );
        if used_chars.saturating_add(candidate.len()) > preflight_cfg.max_chars {
            assembled.report.decisions.push(ContextDecisionReport::info(
                "active_recall_skipped",
                "active recall candidate skipped because char budget was exhausted",
            ));
            continue;
        }
        let candidate_tokens = macaca_host_composition::context::estimate_text_tokens(&candidate);
        if used_tokens.saturating_add(candidate_tokens) > preflight_cfg.max_tokens {
            assembled.report.decisions.push(ContextDecisionReport::info(
                "active_recall_skipped",
                "active recall candidate skipped because token budget was exhausted",
            ));
            continue;
        }
        selected += 1;
        used_chars += candidate.len();
        used_tokens = used_tokens.saturating_add(candidate_tokens);
        rendered.push_str(&candidate);
        rendered.push('\n');
        source_breakdown.push(request_memory_source_report(
            entry,
            "active memory recall",
            candidate_tokens,
            candidate.len(),
            "excerpt",
        ));
    }

    if selected == 0 {
        return;
    }

    insert_after_leading_system(&mut assembled.messages, LlmMessage::system(rendered));
    for source in source_breakdown.iter().cloned() {
        assembled.report.memory_tokens = assembled
            .report
            .memory_tokens
            .saturating_add(source.estimated_tokens);
        assembled.report.estimated_total_tokens = assembled
            .report
            .estimated_total_tokens
            .saturating_add(source.estimated_tokens);
        assembled.report.sources.push(source);
    }
    assembled
        .report
        .active_recall
        .push(ActiveRecallDiagnostics {
            provider_id: "workspace-memory".into(),
            total_candidates: result.total_candidates,
            selected_candidates: selected,
            latency_ms: started.elapsed().as_millis() as u64,
            source_breakdown,
            decisions: vec![ContextDecisionReport::info(
                "active_recall_applied",
                format!("active memory recall injected {selected} entries"),
            )],
        });
    assembled.report.decisions.push(ContextDecisionReport::info(
        "active_recall_applied",
        format!("active memory recall injected {selected} entries"),
    ));
    tracing::info!(
        command = "context.memory.active_recall",
        selected,
        latency_ms = started.elapsed().as_millis() as u64,
        "active memory recall injected into assembled context"
    );
}
