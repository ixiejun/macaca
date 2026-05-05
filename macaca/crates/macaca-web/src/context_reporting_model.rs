use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{ContextAssembleInput, ContextBudget, ContextManagerFacade};
use macaca_framework::model::{ChatModel, ChatOptions, ChatResponse, ModelError};
use macaca_persist::{AppendEventCommand, EventLog};
use macaca_proto::ApplicationId;

pub(crate) struct ContextReportingChatModel {
    inner: Arc<dyn ChatModel>,
    event_log: Arc<EventLog>,
    app_id: ApplicationId,
    session_id: Option<String>,
    agent_name: String,
}

impl ContextReportingChatModel {
    pub(crate) fn new(
        inner: Arc<dyn ChatModel>,
        event_log: Arc<EventLog>,
        app_id: ApplicationId,
        session_id: Option<String>,
        agent_name: String,
    ) -> Self {
        Self {
            inner,
            event_log,
            app_id,
            session_id,
            agent_name,
        }
    }

    async fn emit_report(&self, messages: &[serde_json::Value], options: &ChatOptions) {
        let Some(session_id) = self.session_id.as_deref() else {
            return;
        };
        let model = options.model.clone().unwrap_or_default();
        let input = ContextAssembleInput {
            app_id: Some(self.app_id),
            session_id: Some(session_id.to_string()),
            agent_name: self.agent_name.clone(),
            model: model.clone(),
            base_messages: framework_messages_to_llm(messages),
            options: framework_options_to_llm(options),
            budget: ContextBudget::default(),
        };
        match ContextManagerFacade::legacy().assemble(input).await {
            Ok(result) => {
                self.event_log
                    .append_command(
                        AppendEventCommand::new(
                            session_id,
                            "context_report",
                            &self.agent_name,
                            serde_json::json!({
                                "agent": self.agent_name,
                                "engine_id": result.report.engine_id,
                                "request_id": result.report.request_id,
                                "model": model,
                                "created_at": result.report.created_at,
                                "estimated_total_tokens": result.report.estimated_total_tokens,
                                "token_budget": result.report.token_budget,
                                "stable_prompt_tokens": result.report.stable_prompt_tokens,
                                "dynamic_prompt_tokens": result.report.dynamic_prompt_tokens,
                                "history_tokens": result.report.history_tokens,
                                "tool_schema_tokens": result.report.tool_schema_tokens,
                                "skill_tokens": result.report.skill_tokens,
                                "memory_tokens": result.report.memory_tokens,
                                "trace_tokens": result.report.trace_tokens,
                                "pruned_tokens": result.report.pruned_tokens,
                                "stable_prompt_hash": result.report.stable_prompt_hash,
                                "prompt_hash": result.report.prompt_hash,
                                "source_count": result.report.sources.len(),
                                "decision_count": result.report.decisions.len(),
                                "source_breakdown": result.report.sources.iter().map(|source| {
                                    serde_json::json!({
                                        "id": source.id,
                                        "kind": source.kind,
                                        "label": source.label,
                                        "estimated_tokens": source.estimated_tokens,
                                        "byte_size": source.byte_size,
                                        "included": source.included,
                                    })
                                }).collect::<Vec<_>>(),
                                "decisions": result.report.decisions.iter().map(|decision| {
                                    serde_json::json!({
                                        "code": decision.code,
                                        "severity": decision.severity,
                                        "message": decision.message,
                                    })
                                }).collect::<Vec<_>>(),
                                "warnings": result.report.decisions.iter()
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
            }
            Err(error) => {
                tracing::warn!(
                    agent = %self.agent_name,
                    error = %error,
                    "failed to assemble legacy context report"
                );
            }
        }
    }
}

#[async_trait]
impl ChatModel for ContextReportingChatModel {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        self.emit_report(&messages, options).await;
        self.inner.chat(messages, options).await
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
                "assistant" => Some(macaca_proto::LlmMessage::assistant(content)),
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
