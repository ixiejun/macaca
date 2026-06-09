//! Preflight `memory_search` injection — explicit tool-aligned recall path.
//!
//! Preserves the existing preflight behavior as a separate injector from active
//! recall, using the same message insertion policy.

use std::sync::Arc;

use macaca_sdk::context::{
    ContextDecisionReport, ContextDecisionSeverity, ContextPreflightRecallConfig,
    ContextSourceKind, ContextSourceReport,
};
use macaca_proto::{LlmMessage, MemoryEntry};
use macaca_sdk::memory::{MemoryPolicyHints, MemoryPrefetchCommand, MemoryScope};
use tokio::time::timeout;

use crate::context_memory_injection::adapter::{
    insert_after_leading_system, legacy_memory_source_report, memory_trace, truncate_chars,
};
use crate::context_message_codec::last_user_text_from_framework;

/// Optionally inject bounded workspace-memory recall before the model call.
pub(crate) async fn apply_preflight_memory(
    recall_runtime: &macaca_proto::config::ContextRecallRuntimeConfig,
    memory_client: &Arc<dyn macaca_sdk::SystemMemoryClient>,
    scope: MemoryScope,
    preflight_cfg: &ContextPreflightRecallConfig,
    assembled: &mut macaca_sdk::context::ContextAssembleResult,
    incoming_framework_messages: &[serde_json::Value],
) {
    if !preflight_cfg.enabled || !preflight_cfg.allows_tool("memory_search") {
        return;
    }
    let Some(query) = last_user_text_from_framework(incoming_framework_messages) else {
        return;
    };
    if query.trim().is_empty() {
        return;
    }
    let limit = recall_runtime.memory_search_default_limit.clamp(1, 16) as usize;
    let trace = memory_trace(
        scope.identity.session_id.as_deref(),
        scope.identity.agent_name.as_deref(),
    );
    tracing::info!(
        trace_id = %trace.trace_id,
        command = "context.memory.preflight_recall",
        limit,
        "starting preflight memory_search prefetch"
    );
    let command = MemoryPrefetchCommand {
        scope,
        trace,
        query: query.to_owned(),
        limit,
        policy: MemoryPolicyHints::default(),
    };
    let recall_res = match timeout(preflight_cfg.timeout(), memory_client.prefetch(command)).await {
        Ok(res) => res.map(|result| result.entries),
        Err(_) => {
            tracing::warn!(
                command = "context.memory.preflight_recall",
                "preflight memory recall timed out"
            );
            assembled.report.decisions.push(ContextDecisionReport {
                code: "preflight_recall_degraded".into(),
                severity: ContextDecisionSeverity::Warning,
                message: "preflight memory recall timed out".into(),
            });
            return;
        }
    };
    match recall_res {
        Ok(entries) => inject_preflight_entries(preflight_cfg, assembled, entries),
        Err(error) => {
            tracing::warn!(
                command = "context.memory.preflight_recall",
                error = %error,
                fatal = preflight_cfg.fatal_on_failure,
                "preflight memory_search failed"
            );
            assembled
                .report
                .decisions
                .push(if preflight_cfg.fatal_on_failure {
                    ContextDecisionReport {
                        code: "preflight_recall_fatal".into(),
                        severity: ContextDecisionSeverity::Error,
                        message: format!(
                            "preflight memory_search failed (fatal configured): {error}"
                        ),
                    }
                } else {
                    ContextDecisionReport {
                        code: "preflight_recall_degraded".into(),
                        severity: ContextDecisionSeverity::Warning,
                        message: format!("preflight memory_search failed (non-fatal): {error}"),
                    }
                });
        }
    }
}

/// Render and account the preflight memory-search result.
fn inject_preflight_entries(
    preflight_cfg: &ContextPreflightRecallConfig,
    assembled: &mut macaca_sdk::context::ContextAssembleResult,
    entries: Vec<MemoryEntry>,
) {
    if entries.is_empty() {
        assembled.report.decisions.push(ContextDecisionReport::info(
            "preflight_recall_empty",
            "preflight memory_search returned no rows",
        ));
        return;
    }
    let rendered = serde_json::to_string_pretty(&entries).unwrap_or_default();
    let truncated = truncate_chars(rendered, preflight_cfg.max_chars);
    let tok = macaca_sdk::context::estimate_text_tokens(&truncated);
    insert_after_leading_system(
        &mut assembled.messages,
        LlmMessage::system(format!(
            "[Preflight memory recall - reference only, verify before acting]\n{truncated}"
        )),
    );
    if let Some(first_entry) = entries.first() {
        assembled.report.sources.push(legacy_memory_source_report(
            first_entry,
            "Preflight workspace memory recall",
            tok,
            truncated.len(),
            "summary",
        ));
    } else {
        assembled.report.sources.push(
            ContextSourceReport::included(
                "preflight/memory_search",
                ContextSourceKind::Memory,
                "Preflight workspace memory recall",
                tok,
                truncated.len(),
            )
            .with_rendering(
                "summary",
                "untrusted",
                Some("preflight/memory_search".into()),
                0,
            )
            .with_recall_metadata(
                "workspace-memory",
                "preflight/memory_search",
                85,
                "workspace",
                true,
            ),
        );
    }
    assembled.report.memory_tokens = assembled.report.memory_tokens.saturating_add(tok);
    assembled.report.estimated_total_tokens =
        assembled.report.estimated_total_tokens.saturating_add(tok);
    assembled.report.decisions.push(ContextDecisionReport::info(
        "preflight_recall_applied",
        format!("preflight memory_search injected {} entries", entries.len()),
    ));
    tracing::info!(
        command = "context.memory.preflight_recall",
        entry_count = entries.len(),
        estimated_tokens = tok,
        "preflight memory_search injected into assembled context"
    );
}
