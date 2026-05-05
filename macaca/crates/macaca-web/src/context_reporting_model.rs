use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{
    ContextAssembleInput, ContextBudget, ContextEngineSelection, ContextPreflightRecallConfig,
    ContextRuntimeFacade, ContextSourceKind, ContextSourceReport,
};
use macaca_framework::model::{ChatModel, ChatOptions, ChatResponse, ModelError};
use macaca_memory::{RecallQuery, TestMemoryManager};
use macaca_persist::{AppendEventCommand, EventLog, SessionLineageStore};
use macaca_proto::{config::ContextConfig, ApplicationId, LlmMessage, LlmOptions};
use tokio::time::timeout;

pub(crate) struct ContextReportingChatModel {
    inner: Arc<dyn ChatModel>,
    event_log: Arc<EventLog>,
    persist_backend: Arc<dyn macaca_persist::PersistBackend>,
    app_id: ApplicationId,
    session_id: Option<String>,
    agent_name: String,
    context_selection: ContextEngineSelection,
    context_budget: ContextBudget,
    recall_runtime: macaca_proto::config::ContextRecallRuntimeConfig,
    workspace_memory: Option<Arc<TestMemoryManager>>,
}

impl ContextReportingChatModel {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        inner: Arc<dyn ChatModel>,
        event_log: Arc<EventLog>,
        persist_backend: Arc<dyn macaca_persist::PersistBackend>,
        app_id: ApplicationId,
        session_id: Option<String>,
        agent_name: String,
        merged_context_config: ContextConfig,
        workspace_memory: Option<Arc<TestMemoryManager>>,
    ) -> Self {
        Self {
            inner,
            event_log,
            persist_backend,
            app_id,
            session_id,
            agent_name,
            context_selection: ContextEngineSelection {
                engine_id: merged_context_config.default_engine,
                fallback_engine_id: merged_context_config.fallback_engine,
            },
            context_budget: ContextBudget::new(
                merged_context_config.max_tokens,
                merged_context_config.reserve_output_tokens,
            ),
            recall_runtime: merged_context_config.recall.clone(),
            workspace_memory,
        }
    }

    fn preflight_config(&self) -> ContextPreflightRecallConfig {
        ContextPreflightRecallConfig {
            enabled: self.recall_runtime.preflight_recall_enabled,
            allowed_tool_names: self.recall_runtime.preflight_allowed_tools.clone(),
            timeout_ms: self.recall_runtime.preflight_timeout_ms,
            max_chars: self.recall_runtime.preflight_max_chars,
            max_tokens: self.recall_runtime.preflight_max_tokens,
            fatal_on_failure: self.recall_runtime.preflight_fatal_on_failure,
        }
    }

    async fn lineage_compactions(&self, session_id: &str) -> u32 {
        let store = SessionLineageStore::new(Arc::clone(&self.persist_backend));
        store
            .count_compaction_successors(session_id)
            .await
            .unwrap_or(0)
    }

    async fn apply_preflight_memory(
        &self,
        preflight_cfg: &ContextPreflightRecallConfig,
        assembled: &mut macaca_context::ContextAssembleResult,
        incoming_framework_messages: &[serde_json::Value],
    ) {
        if !preflight_cfg.enabled || !preflight_cfg.allows_tool("memory_search") {
            return;
        }
        let Some(memory) = self.workspace_memory.as_ref() else {
            assembled
                .report
                .decisions
                .push(macaca_context::ContextDecisionReport::info(
                    "preflight_memory_skipped",
                    "preflight recall allowed memory_search but no memory backend is configured",
                ));
            return;
        };
        let query = match last_user_text_from_framework(incoming_framework_messages) {
            Some(text) => text,
            None => return,
        };
        if query.trim().is_empty() {
            return;
        }
        let limit = self.recall_runtime.memory_search_default_limit.clamp(1, 16) as usize;
        let fut = memory.recall(RecallQuery::new(query, limit));
        let recall_res = match timeout(preflight_cfg.timeout(), fut).await {
            Ok(res) => res,
            Err(_) => {
                assembled
                    .report
                    .decisions
                    .push(macaca_context::ContextDecisionReport {
                        code: "preflight_recall_degraded".into(),
                        severity: macaca_context::ContextDecisionSeverity::Warning,
                        message: "preflight memory recall timed out".into(),
                    });
                return;
            }
        };
        match recall_res {
            Ok(bundle) => {
                if bundle.entries.is_empty() {
                    assembled
                        .report
                        .decisions
                        .push(macaca_context::ContextDecisionReport::info(
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
                        "[Preflight memory recall — reference only, verify before acting]\n{truncated}"
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
                assembled
                    .report
                    .decisions
                    .push(macaca_context::ContextDecisionReport::info(
                        "preflight_recall_applied",
                        format!(
                            "preflight memory_search injected {} entries",
                            bundle.entries.len()
                        ),
                    ));
            }
            Err(e) => {
                assembled
                    .report
                    .decisions
                    .push(if preflight_cfg.fatal_on_failure {
                        macaca_context::ContextDecisionReport {
                            code: "preflight_recall_fatal".into(),
                            severity: macaca_context::ContextDecisionSeverity::Error,
                            message: format!(
                                "preflight memory_search failed (fatal configured): {e}"
                            ),
                        }
                    } else {
                        macaca_context::ContextDecisionReport {
                            code: "preflight_recall_degraded".into(),
                            severity: macaca_context::ContextDecisionSeverity::Warning,
                            message: format!("preflight memory_search failed (non-fatal): {e}"),
                        }
                    });
            }
        }
    }

    async fn assemble_and_emit_report(
        &self,
        messages: &[serde_json::Value],
        options: &ChatOptions,
    ) -> Option<(Vec<serde_json::Value>, ChatOptions)> {
        let Some(session_id) = self.session_id.as_deref() else {
            return None;
        };
        let model = options.model.clone().unwrap_or_default();
        let input = ContextAssembleInput {
            app_id: Some(self.app_id),
            session_id: Some(session_id.to_string()),
            agent_name: self.agent_name.clone(),
            model: model.clone(),
            base_messages: framework_messages_to_llm(messages),
            options: framework_options_to_llm(options),
            budget: self.context_budget,
        };
        let lineage_count = self.lineage_compactions(session_id).await;
        let preflight_cfg = self.preflight_config();
        match ContextRuntimeFacade::builtins(self.context_selection.clone())
            .assemble(input)
            .await
        {
            Ok(mut assembled) => {
                assembled.report.lineage_compaction_count = lineage_count;
                self.apply_preflight_memory(&preflight_cfg, &mut assembled, messages)
                    .await;
                let output_messages = if assembled.report.engine_id == "legacy" {
                    messages.to_vec()
                } else {
                    llm_messages_to_framework(&assembled.messages)
                };
                let output_options = llm_options_to_framework(&assembled.options, options);
                let report = assembled.report.clone();
                let fallback_decisions = assembled
                    .report
                    .decisions
                    .iter()
                    .filter(|decision| decision.code == "context_engine_fallback")
                    .cloned()
                    .collect::<Vec<_>>();
                self.event_log
                    .append_command(
                        AppendEventCommand::new(
                            session_id,
                            "context_report",
                            &self.agent_name,
                            serde_json::json!({
                                "agent": self.agent_name,
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
                                "source_breakdown": report.sources.iter().map(|source| {
                                    serde_json::json!({
                                        "id": source.id,
                                        "kind": source.kind,
                                        "label": source.label,
                                        "estimated_tokens": source.estimated_tokens,
                                        "byte_size": source.byte_size,
                                        "included": source.included,
                                        "pruned_tokens": source.pruned_tokens,
                                        "render_mode": source.render_mode,
                                        "trust_level": source.trust_level,
                                        "source_ref": source.source_ref,
                                    })
                                }).collect::<Vec<_>>(),
                                "decisions": report.decisions.iter().map(|decision| {
                                    serde_json::json!({
                                        "code": decision.code,
                                        "severity": decision.severity,
                                        "message": decision.message,
                                    })
                                }).collect::<Vec<_>>(),
                                "warnings": report.decisions.iter()
                                    .filter(|decision| decision.severity != macaca_context::ContextDecisionSeverity::Info)
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
                        .with_app_id(self.app_id.to_string())
                        .with_agent_name(self.agent_name.clone()),
                    )
                    .await;
                for decision in fallback_decisions {
                    self.event_log
                        .append_command(
                            AppendEventCommand::new(
                                session_id,
                                "context_engine_fallback",
                                &self.agent_name,
                                serde_json::json!({
                                    "agent": self.agent_name,
                                    "engine_id": report.engine_id,
                                    "requested_engine_id": report.requested_engine_id,
                                    "request_id": report.request_id,
                                    "code": decision.code,
                                    "severity": decision.severity,
                                    "message": decision.message,
                                }),
                            )
                            .with_app_id(self.app_id.to_string())
                            .with_agent_name(self.agent_name.clone()),
                        )
                        .await;
                }
                Some((output_messages, output_options))
            }
            Err(error) => {
                tracing::warn!(
                    agent = %self.agent_name,
                    error = %error,
                    "failed to assemble legacy context report"
                );
                None
            }
        }
    }
}

fn truncate_chars(mut s: String, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s;
    }
    s = s.chars().take(max_chars).collect();
    s.push_str("\n...[preflight truncated]");
    s
}

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

fn last_user_text_from_framework(messages: &[serde_json::Value]) -> Option<String> {
    for message in messages.iter().rev() {
        if message.get("role").and_then(|v| v.as_str()) != Some("user") {
            continue;
        }
        return Some(message_text_content(
            message.get("content").unwrap_or(&serde_json::Value::Null),
        ));
    }
    None
}

#[async_trait]
impl ChatModel for ContextReportingChatModel {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        if let Some((assembled_messages, assembled_options)) =
            self.assemble_and_emit_report(&messages, options).await
        {
            self.inner
                .chat(assembled_messages, &assembled_options)
                .await
        } else {
            self.inner.chat(messages, options).await
        }
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

fn framework_options_to_llm(options: &ChatOptions) -> macaca_proto::LlmOptions {
    let mut llm_options = macaca_proto::LlmOptions {
        model: options.model.clone().unwrap_or_default(),
        temperature: options.temperature,
        max_tokens: options.max_tokens,
        ..Default::default()
    };
    if let Some(tools) = options.tools.as_ref() {
        llm_options.tools = Some(
            tools
                .iter()
                .map(|tool| macaca_proto::ToolDefinition {
                    name: tool["name"].as_str().unwrap_or("").to_string(),
                    description: tool["description"].as_str().unwrap_or("").to_string(),
                    parameters: tool["parameters"].clone(),
                })
                .collect(),
        );
    }
    llm_options
}

fn llm_options_to_framework(options: &LlmOptions, original: &ChatOptions) -> ChatOptions {
    ChatOptions {
        model: if options.model.is_empty() {
            original.model.clone()
        } else {
            Some(options.model.clone())
        },
        temperature: options.temperature.or(original.temperature),
        max_tokens: options.max_tokens.or(original.max_tokens),
        top_p: original.top_p,
        tools: original.tools.clone(),
        tool_choice: original.tool_choice.clone(),
    }
}

fn framework_messages_to_llm(messages: &[serde_json::Value]) -> Vec<macaca_proto::LlmMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = message.get("role").and_then(|value| value.as_str())?;
            let content =
                message_text_content(message.get("content").unwrap_or(&serde_json::Value::Null));
            match role {
                "system" => Some(macaca_proto::LlmMessage::system(content)),
                "user" => Some(macaca_proto::LlmMessage::user(content)),
                "assistant" => Some(assistant_message_from_json(message, content)),
                "tool" => {
                    let tool_call_id = message
                        .get("tool_call_id")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    Some(macaca_proto::LlmMessage::tool_result(tool_call_id, content))
                }
                _ => None,
            }
        })
        .collect()
}

fn assistant_message_from_json(message: &serde_json::Value, content: String) -> LlmMessage {
    let Some(tool_calls) = message.get("tool_calls").and_then(|value| value.as_array()) else {
        return LlmMessage::assistant(content);
    };
    let calls = tool_calls
        .iter()
        .filter_map(|call| {
            Some(macaca_proto::ToolCall {
                id: call.get("id")?.as_str().unwrap_or_default().to_string(),
                name: call
                    .get("function")
                    .and_then(|function| function.get("name"))
                    .and_then(|value| value.as_str())
                    .or_else(|| call.get("name").and_then(|value| value.as_str()))
                    .unwrap_or_default()
                    .to_string(),
                arguments: call
                    .get("function")
                    .and_then(|function| function.get("arguments"))
                    .map(parse_tool_arguments)
                    .or_else(|| call.get("input").cloned())
                    .unwrap_or_else(|| serde_json::json!({})),
            })
        })
        .collect::<Vec<_>>();
    if calls.is_empty() {
        LlmMessage::assistant(content)
    } else {
        LlmMessage::assistant_with_tool_calls(content, calls)
    }
}

fn parse_tool_arguments(value: &serde_json::Value) -> serde_json::Value {
    value
        .as_str()
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or_else(|| value.clone())
}

fn llm_messages_to_framework(messages: &[LlmMessage]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|message| match message.role {
            macaca_proto::LlmRole::System => serde_json::json!({
                "role": "system",
                "content": message.content,
            }),
            macaca_proto::LlmRole::User => serde_json::json!({
                "role": "user",
                "content": message.content,
            }),
            macaca_proto::LlmRole::Assistant => assistant_message_to_json(message),
            macaca_proto::LlmRole::Tool => serde_json::json!({
                "role": "tool",
                "tool_call_id": message.tool_call_id.clone().unwrap_or_default(),
                "content": message.content,
            }),
        })
        .collect()
}

fn assistant_message_to_json(message: &LlmMessage) -> serde_json::Value {
    if let Some(tool_calls) = message.tool_calls.as_ref() {
        let calls = tool_calls
            .iter()
            .map(|call| {
                serde_json::json!({
                    "id": call.id,
                    "type": "function",
                    "function": {
                        "name": call.name,
                        "arguments": call.arguments.to_string(),
                    }
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "role": "assistant",
            "content": if message.content.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(message.content.clone())
            },
            "tool_calls": calls,
        })
    } else {
        serde_json::json!({
            "role": "assistant",
            "content": message.content,
        })
    }
}

fn message_text_content(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
            .collect::<Vec<_>>()
            .join(""),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
