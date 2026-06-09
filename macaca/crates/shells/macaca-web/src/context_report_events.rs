use macaca_sdk::context::{ContextDecisionSeverity, ContextReport, ContextSourceReport};
use macaca_sdk::runtime_host::persist::{AppendEventCommand, EventLog};
use macaca_proto::ApplicationId;

fn source_report_value(source: &ContextSourceReport) -> serde_json::Value {
    serde_json::to_value(source).unwrap_or_else(|_| serde_json::Value::Null)
}

/// Persist the sanitized context report and fallback events for one model call.
///
/// The helper keeps event serialization isolated from context assembly.  It
/// writes only bounded report metadata, source summaries, and decisions; prompt
/// bodies and raw memory payloads remain outside the event log by default.
pub(crate) async fn persist_context_report_events(
    event_log: &EventLog,
    session_id: &str,
    agent_name: &str,
    app_id: ApplicationId,
    model: &str,
    report: &ContextReport,
) {
    let fallback_decisions = report
        .decisions
        .iter()
        .filter(|decision| decision.code == "context_engine_fallback")
        .cloned()
        .collect::<Vec<_>>();
    event_log
        .append_command(
            AppendEventCommand::new(
                session_id,
                "context_report",
                agent_name,
                serde_json::json!({
                    "agent": agent_name,
                    "engine_id": report.engine_id,
                    "requested_engine_id": report.requested_engine_id,
                    "engine_fallback_applied": report.engine_fallback_applied,
                    "lineage_compaction_count": report.lineage_compaction_count,
                    "request_id": report.request_id,
                    "model": model,
                    "created_at": report.created_at,
                    "estimated_total_tokens": report.estimated_total_tokens,
                    "token_budget": report.token_budget,
                    "stable_prompt_tokens": report.stable_prompt_tokens,
                    "dynamic_prompt_tokens": report.dynamic_prompt_tokens,
                    "history_tokens": report.history_tokens,
                    "tool_schema_tokens": report.tool_schema_tokens,
                    "skill_tokens": report.skill_tokens,
                    "memory_tokens": report.memory_tokens,
                    "trace_tokens": report.trace_tokens,
                    "pruned_tokens": report.pruned_tokens,
                    "stable_prompt_hash": report.stable_prompt_hash,
                    "prompt_hash": report.prompt_hash,
                    "source_count": report.sources.len(),
                    "decision_count": report.decisions.len(),
                    "provider_runtime": report.provider_runtime.as_ref().map(|runtime| {
                        serde_json::to_value(runtime).unwrap_or_else(|_| serde_json::Value::Null)
                    }),
                    "active_recall": report.active_recall.iter().map(|recall| {
                        serde_json::json!({
                            "provider_id": recall.provider_id,
                            "total_candidates": recall.total_candidates,
                            "selected_candidates": recall.selected_candidates,
                            "latency_ms": recall.latency_ms,
                            "source_count": recall.source_breakdown.len(),
                            "source_breakdown": recall.source_breakdown.iter().map(source_report_value).collect::<Vec<_>>(),
                            "decisions": recall.decisions.iter().map(|decision| {
                                serde_json::json!({
                                    "code": decision.code,
                                    "severity": decision.severity,
                                    "message": decision.message,
                                })
                            }).collect::<Vec<_>>(),
                        })
                    }).collect::<Vec<_>>(),
                    "knowledge_digest": report.knowledge_digest.iter().map(|digest| {
                        serde_json::json!({
                            "provider_id": digest.provider_id,
                            "total_candidates": digest.total_candidates,
                            "selected_candidates": digest.selected_candidates,
                            "source_count": digest.source_breakdown.len(),
                            "source_breakdown": digest.source_breakdown.iter().map(source_report_value).collect::<Vec<_>>(),
                        })
                    }).collect::<Vec<_>>(),
                    "source_breakdown": report.sources.iter().map(source_report_value).collect::<Vec<_>>(),
                    "decisions": report.decisions.iter().map(|decision| {
                        serde_json::json!({
                            "code": decision.code,
                            "severity": decision.severity,
                            "message": decision.message,
                        })
                    }).collect::<Vec<_>>(),
                    "warnings": report.decisions.iter()
                        .filter(|decision| decision.severity != ContextDecisionSeverity::Info)
                        .map(|decision| {
                            serde_json::json!({
                                "code": decision.code,
                                "severity": decision.severity,
                                "message": decision.message,
                            })
                        })
                        .collect::<Vec<_>>(),
                }),
            )
            .with_app_id(app_id.to_string())
            .with_agent_name(agent_name.to_string()),
        )
        .await;
    for decision in fallback_decisions {
        event_log
            .append_command(
                AppendEventCommand::new(
                    session_id,
                    "context_engine_fallback",
                    agent_name,
                    serde_json::json!({
                        "agent": agent_name,
                        "engine_id": report.engine_id,
                        "requested_engine_id": report.requested_engine_id,
                        "request_id": report.request_id,
                        "code": decision.code,
                        "severity": decision.severity,
                        "message": decision.message,
                    }),
                )
                .with_app_id(app_id.to_string())
                .with_agent_name(agent_name.to_string()),
            )
            .await;
    }
}
