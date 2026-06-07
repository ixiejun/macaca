use std::sync::Arc;

use macaca_context::{
    ActiveRecallDiagnostics, ContextDecisionReport, ContextDecisionSeverity,
    ContextPreflightRecallConfig, ContextSourceKind, ContextSourceReport,
};
use macaca_sdk::memory::{MemoryPolicyHints, MemoryPrefetchCommand, MemoryScope};
use macaca_proto::{LlmMessage, MemoryEntry, TraceContext};
use tokio::time::timeout;

use crate::context_message_codec::last_user_text_from_framework;

fn memory_trace(session_id: Option<&str>, agent_name: Option<&str>) -> TraceContext {
    let mut trace = TraceContext::new(uuid::Uuid::new_v4().to_string());
    trace.session_id = session_id.map(str::to_owned);
    trace.agent = agent_name.map(str::to_owned);
    trace
}

fn legacy_memory_source_report(
    entry: &MemoryEntry,
    label: &str,
    estimated_tokens: u32,
    byte_size: usize,
    render_mode: &str,
) -> ContextSourceReport {
    ContextSourceReport::included(
        entry.id.0.to_string(),
        ContextSourceKind::Memory,
        label,
        estimated_tokens,
        byte_size,
    )
    .with_rendering(
        render_mode,
        "untrusted",
        Some(format!("memory:{}", entry.id.0)),
        0,
    )
    .with_recall_metadata(
        "workspace-memory",
        entry.id.0.to_string(),
        85,
        "workspace",
        true,
    )
}

/// Run active recall as a dynamic, request-only context source (legacy injection path).
///
/// This is the web-layer Strategy adapter that connects the Memory Service to
/// the context engine report contract. It is deliberately client-shaped:
/// callers pass the focused SDK facade and scope in, so concrete memory
/// providers remain swappable without changing chat orchestration.
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
    memory_client: &Arc<dyn macaca_sdk::SystemMemoryClient>,
    scope: MemoryScope,
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
        source_breakdown.push(legacy_memory_source_report(
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
}

/// Optionally inject bounded workspace-memory recall before the model call.
///
/// This preserves the existing explicit `memory_search` preflight behavior while
/// keeping it separate from active recall diagnostics. Both injectors use the
/// same insertion policy so memory remains below the leading system prompt and
/// above user history.
pub(crate) async fn apply_preflight_memory(
    recall_runtime: &macaca_proto::config::ContextRecallRuntimeConfig,
    memory_client: &Arc<dyn macaca_sdk::SystemMemoryClient>,
    scope: MemoryScope,
    preflight_cfg: &ContextPreflightRecallConfig,
    assembled: &mut macaca_context::ContextAssembleResult,
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
    assembled: &mut macaca_context::ContextAssembleResult,
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
    let tok = macaca_context::estimate_text_tokens(&truncated);
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_context::{ContextOptionsPatch, ContextReportBuilder};
    use macaca_sdk::memory::{
        MemoryForgetCommand, MemoryGetCommand, MemoryGetResult, MemoryRecallCommand,
        MemoryRecallResult, MemoryRememberCommand, MemoryRememberResult, MemoryServiceSnapshot,
        MemoryServiceSnapshotCommand, MemoryStatusCommand, MemoryStatusReport,
    };
    use macaca_proto::{ApplicationId, LlmOptions, LlmRole, MemoryEntry, MemoryId, MemoryLayer};

    struct StaticMemoryClient {
        entries: Vec<MemoryEntry>,
        fail_search: bool,
    }

    #[async_trait]
    impl macaca_sdk::SystemMemoryClient for StaticMemoryClient {
        async fn remember(
            &self,
            _command: MemoryRememberCommand,
        ) -> macaca_proto::MacacaResult<MemoryRememberResult> {
            Ok(MemoryRememberResult {
                id: MemoryId::new(),
                stored_at: Utc::now(),
            })
        }

        async fn recall(
            &self,
            _command: MemoryRecallCommand,
        ) -> macaca_proto::MacacaResult<MemoryRecallResult> {
            if self.fail_search {
                return Err(macaca_proto::MacacaError::Agent("search failed".into()));
            }
            Ok(MemoryRecallResult::new(self.entries.clone()))
        }

        async fn prefetch(
            &self,
            _command: MemoryPrefetchCommand,
        ) -> macaca_proto::MacacaResult<MemoryRecallResult> {
            self.recall(MemoryRecallCommand {
                scope: MemoryScope::project_shared(ApplicationId::new(), "workspace"),
                trace: TraceContext::new("test"),
                query: "test".into(),
                limit: self.entries.len().max(1),
                policy: MemoryPolicyHints::default(),
            })
            .await
        }

        async fn get(
            &self,
            command: MemoryGetCommand,
        ) -> macaca_proto::MacacaResult<MemoryGetResult> {
            Ok(MemoryGetResult::new(
                self.entries
                    .iter()
                    .find(|entry| entry.id == command.id)
                    .cloned(),
            ))
        }

        async fn forget(&self, _command: MemoryForgetCommand) -> macaca_proto::MacacaResult<()> {
            Ok(())
        }

        async fn status(
            &self,
            _command: MemoryStatusCommand,
        ) -> macaca_proto::MacacaResult<MemoryStatusReport> {
            Ok(MemoryStatusReport::healthy(
                "test-memory-client",
                macaca_sdk::memory::MemoryCapabilitySet::basic_store_search(),
            ))
        }

        async fn snapshot(
            &self,
            _command: MemoryServiceSnapshotCommand,
        ) -> macaca_proto::MacacaResult<MemoryServiceSnapshot> {
            Ok(MemoryServiceSnapshot::new(
                "test-memory-client",
                true,
                macaca_sdk::memory::MemoryCapabilitySet::basic_store_search(),
                None,
            ))
        }
    }

    fn entry(content: &str) -> MemoryEntry {
        MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::Vector,
            content: content.into(),
            metadata: serde_json::Value::Null,
            agent_id: None,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    fn assembled() -> macaca_context::ContextAssembleResult {
        macaca_context::ContextAssembleResult {
            messages: vec![
                macaca_proto::LlmMessage::system("sys"),
                macaca_proto::LlmMessage::user("find memory"),
            ],
            options: LlmOptions::default(),
            options_patch: ContextOptionsPatch::default(),
            report: ContextReportBuilder::new("windowed").build(),
        }
    }

    fn framework_messages() -> Vec<serde_json::Value> {
        vec![serde_json::json!({
            "role": "user",
            "content": [{ "type": "text", "text": "find memory" }]
        })]
    }

    fn preflight_cfg(enabled: bool) -> ContextPreflightRecallConfig {
        ContextPreflightRecallConfig {
            enabled,
            allowed_tool_names: vec!["memory_search".into()],
            timeout_ms: 50,
            max_chars: 10_000,
            max_tokens: 4_000,
            fatal_on_failure: false,
        }
    }

    #[tokio::test]
    async fn preflight_memory_is_invisible_by_default() {
        let memory_client: Arc<dyn macaca_sdk::SystemMemoryClient> = Arc::new(StaticMemoryClient {
            entries: vec![entry("remembered fact")],
            fail_search: false,
        });
        let mut result = assembled();
        let recall_runtime = macaca_proto::config::ContextRecallRuntimeConfig::default();

        apply_preflight_memory(
            &recall_runtime,
            &memory_client,
            MemoryScope::project_shared(ApplicationId::new(), "workspace"),
            &preflight_cfg(false),
            &mut result,
            &framework_messages(),
        )
        .await;

        assert!(result.report.sources.is_empty());
        assert_eq!(result.messages.len(), 2);
    }

    #[tokio::test]
    async fn preflight_memory_fails_open_with_warning() {
        let memory_client: Arc<dyn macaca_sdk::SystemMemoryClient> = Arc::new(StaticMemoryClient {
            entries: vec![entry("remembered fact")],
            fail_search: true,
        });
        let mut result = assembled();
        let recall_runtime = macaca_proto::config::ContextRecallRuntimeConfig::default();

        apply_preflight_memory(
            &recall_runtime,
            &memory_client,
            MemoryScope::project_shared(ApplicationId::new(), "workspace"),
            &preflight_cfg(true),
            &mut result,
            &framework_messages(),
        )
        .await;

        assert!(result.report.sources.is_empty());
        assert!(result
            .report
            .decisions
            .iter()
            .any(|d| d.code == "preflight_recall_degraded"));
    }

    #[tokio::test]
    async fn legacy_active_recall_reports_request_only_metadata() {
        let memory_client: Arc<dyn macaca_sdk::SystemMemoryClient> = Arc::new(StaticMemoryClient {
            entries: vec![entry("remembered fact")],
            fail_search: false,
        });
        let mut result = assembled();
        let recall_runtime = macaca_proto::config::ContextRecallRuntimeConfig::default();

        apply_active_recall(
            &recall_runtime,
            &memory_client,
            MemoryScope::project_shared(ApplicationId::new(), "workspace"),
            &preflight_cfg(true),
            false,
            &mut result,
            &framework_messages(),
        )
        .await;

        let row = result.report.active_recall[0]
            .source_breakdown
            .first()
            .unwrap();
        assert_eq!(
            row.provenance_provider_id.as_deref(),
            Some("workspace-memory")
        );
        assert_eq!(row.privacy_tier.as_deref(), Some("workspace"));
        assert_eq!(row.request_only, Some(true));
        assert_eq!(row.trust_level.as_deref(), Some("untrusted"));
        assert!(result
            .messages
            .iter()
            .any(|m| m.role == LlmRole::System && m.content.contains("reference only")));
    }
}
