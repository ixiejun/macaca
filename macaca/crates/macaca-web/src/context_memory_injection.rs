use std::sync::Arc;

use macaca_context::{
    ActiveRecallDiagnostics, ContextDecisionReport, ContextDecisionSeverity,
    ContextPreflightRecallConfig, ContextSourceKind, ContextSourceReport,
};
use macaca_memory::{RecallQuery, RecallResult, TestMemoryManager};
use macaca_proto::LlmMessage;
use tokio::time::timeout;

use crate::context_message_codec::last_user_text_from_framework;

/// Run active recall as a dynamic, request-only context source (legacy injection path).
///
/// This is the web-layer Strategy adapter that connects the current workspace
/// memory manager to the context engine report contract. It is deliberately
/// provider-shaped: callers pass the memory backend and runtime config in, so a
/// future pluggable memory fabric can replace `TestMemoryManager` without
/// changing the chat orchestration flow.
///
/// ## Composer migration
/// Prefer [`macaca_context::MemoryActiveRecallContextProvider`] when
/// `active_vector_memory` is enabled: it feeds recall through the composer as
/// fenced `macaca_context::ContextCandidate` rows. Pass `composer_recall_active = true` from
/// [`crate::context_reporting_model::ContextReportingChatModel::composer_handles_active_vector_recall`]
/// so this function returns immediately (recall already ran via the provider), avoiding duplicate
/// retrieval and duplicate system-side text.
pub(crate) async fn apply_active_recall(
    recall_runtime: &macaca_proto::config::ContextRecallRuntimeConfig,
    workspace_memory: Option<&Arc<TestMemoryManager>>,
    preflight_cfg: &ContextPreflightRecallConfig,
    composer_recall_active: bool,
    assembled: &mut macaca_context::ContextAssembleResult,
    incoming_framework_messages: &[serde_json::Value],
) {
    if composer_recall_active {
        return;
    }
    if !preflight_cfg.enabled {
        return;
    }
    let Some(memory) = workspace_memory else {
        assembled.report.decisions.push(ContextDecisionReport::info(
            "active_recall_skipped",
            "active recall enabled but no workspace memory backend is configured",
        ));
        return;
    };
    let Some(query) = last_user_text_from_framework(incoming_framework_messages) else {
        return;
    };
    if query.trim().is_empty() {
        return;
    }

    let started = std::time::Instant::now();
    let limit = recall_runtime.memory_search_default_limit.clamp(1, 16) as usize;
    let recall = timeout(
        preflight_cfg.timeout(),
        memory.recall(RecallQuery::new(query, limit)),
    )
    .await;

    let bundle = match recall {
        Ok(Ok(bundle)) => bundle,
        Ok(Err(error)) => {
            assembled.report.decisions.push(ContextDecisionReport {
                code: "active_recall_degraded".into(),
                severity: ContextDecisionSeverity::Warning,
                message: format!("active memory recall failed: {error}"),
            });
            return;
        }
        Err(_) => {
            assembled.report.decisions.push(ContextDecisionReport {
                code: "active_recall_degraded".into(),
                severity: ContextDecisionSeverity::Warning,
                message: "active memory recall timed out".into(),
            });
            return;
        }
    };

    if bundle.entries.is_empty() {
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
    for entry in bundle.entries.iter().take(limit) {
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
        let candidate_tokens = macaca_context::estimate_text_tokens(&candidate);
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
        source_breakdown.push(
            ContextSourceReport::included(
                entry.id.0.to_string(),
                ContextSourceKind::Memory,
                "active memory recall",
                candidate_tokens,
                candidate.len(),
            )
            .with_rendering(
                "excerpt",
                "untrusted",
                Some(format!("memory:{}", entry.id.0)),
                0,
            ),
        );
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
            total_candidates: bundle.entries.len(),
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
}

/// Optionally inject bounded workspace-memory recall before the model call.
///
/// This preserves the existing explicit `memory_search` preflight behavior while
/// keeping it separate from active recall diagnostics. Both injectors use the
/// same insertion policy so memory remains below the leading system prompt and
/// above user history.
pub(crate) async fn apply_preflight_memory(
    recall_runtime: &macaca_proto::config::ContextRecallRuntimeConfig,
    workspace_memory: Option<&Arc<TestMemoryManager>>,
    preflight_cfg: &ContextPreflightRecallConfig,
    assembled: &mut macaca_context::ContextAssembleResult,
    incoming_framework_messages: &[serde_json::Value],
) {
    if !preflight_cfg.enabled || !preflight_cfg.allows_tool("memory_search") {
        return;
    }
    let Some(memory) = workspace_memory else {
        assembled.report.decisions.push(ContextDecisionReport::info(
            "preflight_memory_skipped",
            "preflight recall allowed memory_search but no memory backend is configured",
        ));
        return;
    };
    let Some(query) = last_user_text_from_framework(incoming_framework_messages) else {
        return;
    };
    if query.trim().is_empty() {
        return;
    }
    let limit = recall_runtime.memory_search_default_limit.clamp(1, 16) as usize;
    let recall_res = match timeout(
        preflight_cfg.timeout(),
        memory.recall(RecallQuery::new(query, limit)),
    )
    .await
    {
        Ok(res) => res,
        Err(_) => {
            assembled.report.decisions.push(ContextDecisionReport {
                code: "preflight_recall_degraded".into(),
                severity: ContextDecisionSeverity::Warning,
                message: "preflight memory recall timed out".into(),
            });
            return;
        }
    };
    match recall_res {
        Ok(bundle) => inject_preflight_bundle(preflight_cfg, assembled, bundle),
        Err(error) => {
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

/// Render and account the legacy preflight memory-search result.
fn inject_preflight_bundle(
    preflight_cfg: &ContextPreflightRecallConfig,
    assembled: &mut macaca_context::ContextAssembleResult,
    bundle: RecallResult,
) {
    if bundle.entries.is_empty() {
        assembled.report.decisions.push(ContextDecisionReport::info(
            "preflight_recall_empty",
            "preflight memory_search returned no rows",
        ));
        return;
    }
    let rendered = serde_json::to_string_pretty(&bundle.entries).unwrap_or_default();
    let truncated = truncate_chars(rendered, preflight_cfg.max_chars);
    let tok = macaca_context::estimate_text_tokens(&truncated);
    insert_after_leading_system(
        &mut assembled.messages,
        LlmMessage::system(format!(
            "[Preflight memory recall - reference only, verify before acting]\n{truncated}"
        )),
    );
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
        ),
    );
    assembled.report.memory_tokens = assembled.report.memory_tokens.saturating_add(tok);
    assembled.report.estimated_total_tokens =
        assembled.report.estimated_total_tokens.saturating_add(tok);
    assembled.report.decisions.push(ContextDecisionReport::info(
        "preflight_recall_applied",
        format!(
            "preflight memory_search injected {} entries",
            bundle.entries.len()
        ),
    ));
}

/// Insert a synthetic system snippet after the leading block of system messages.
///
/// Keeping recall below the leading system block prevents recalled memory from
/// silently overriding the core instruction hierarchy, while still making it
/// visible before normal conversation history.
fn insert_after_leading_system(messages: &mut Vec<LlmMessage>, snippet: LlmMessage) {
    let mut pos = 0usize;
    for message in messages.iter() {
        if message.role == macaca_proto::LlmRole::System {
            pos += 1;
        } else {
            break;
        }
    }
    messages.insert(pos, snippet);
}

/// Truncate a string by character count and append a diagnostic marker.
fn truncate_chars(mut s: String, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s;
    }
    s = s.chars().take(max_chars).collect();
    s.push_str("\n...[preflight truncated]");
    s
}
