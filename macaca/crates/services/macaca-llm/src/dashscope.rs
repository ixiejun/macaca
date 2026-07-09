//! DashScope (阿里百炼) LLM provider — supports Qwen series models.
//!
//! Uses the OpenAI-compatible chat/completions endpoint at DashScope.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use macaca_proto::{
    error::{MacacaError, MacacaResult},
    types::{LlmMessage, LlmOptions, LlmResponse, LlmRole, TokenUsage, ToolCall},
};

use crate::provider::LlmProvider;
use crate::tool_wire::tool_arguments_for_chat_api;

const DEFAULT_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

/// DashScope (阿里百炼) LLM provider.
///
/// Supports Qwen series models (qwen-turbo, qwen-plus, qwen-max, etc.)
/// via the OpenAI-compatible chat API endpoint.
pub struct DashScopeProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl DashScopeProvider {
    /// Creates a new provider, reading `DASHSCOPE_API_KEY` from the environment.
    pub fn from_env() -> MacacaResult<Self> {
        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .map_err(|_| MacacaError::Config("DASHSCOPE_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_owned(),
            client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap_or_default(),
        }
    }

    /// Override the base URL.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

// ── Wire types (OpenAI-compatible format) ────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OaiToolDef>>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize)]
struct OaiToolDef {
    #[serde(rename = "type")]
    tool_type: String,
    function: OaiFunction,
}

#[derive(Serialize)]
struct OaiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
struct OaiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OaiFunctionCall,
}

#[derive(Serialize, Deserialize, Clone)]
struct OaiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Usage,
    model: String,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

fn role_str(role: LlmRole) -> &'static str {
    match role {
        LlmRole::System => "system",
        LlmRole::User => "user",
        LlmRole::Assistant => "assistant",
        LlmRole::Tool => "tool",
    }
}

fn convert_message(m: &LlmMessage) -> ChatMessage {
    let mut msg = ChatMessage {
        role: role_str(m.role).to_owned(),
        content: if m.content.is_empty() {
            None
        } else {
            Some(m.content.clone())
        },
        tool_calls: None,
        tool_call_id: m.tool_call_id.clone(),
    };
    if let Some(ref calls) = m.tool_calls {
        msg.tool_calls = Some(
            calls
                .iter()
                .map(|tc| OaiToolCall {
                    id: tc.id.clone(),
                    call_type: "function".into(),
                    function: OaiFunctionCall {
                        name: tc.name.clone(),
                        arguments: tool_arguments_for_chat_api(&tc.arguments),
                    },
                })
                .collect(),
        );
    }
    msg
}

fn parse_tool_calls(oai_calls: Vec<OaiToolCall>) -> Vec<ToolCall> {
    oai_calls
        .into_iter()
        .map(|tc| ToolCall {
            id: tc.id,
            name: tc.function.name,
            arguments: serde_json::from_str(&tc.function.arguments)
                .unwrap_or(serde_json::Value::String(tc.function.arguments)),
        })
        .collect()
}

#[async_trait]
impl LlmProvider for DashScopeProvider {
    fn name(&self) -> &str {
        "dashscope"
    }

    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let chat_messages: Vec<ChatMessage> = messages.iter().map(convert_message).collect();
        let stop_strs: Vec<String> = options.stop_sequences.clone();

        let tools = options.tools.as_ref().map(|defs| {
            defs.iter()
                .map(|d| OaiToolDef {
                    tool_type: "function".into(),
                    function: OaiFunction {
                        name: d.name.clone(),
                        description: d.description.clone(),
                        parameters: d.parameters.clone(),
                    },
                })
                .collect()
        });

        let body = ChatRequest {
            model: options.model.clone(),
            messages: chat_messages,
            max_tokens: options.max_tokens,
            temperature: options.temperature,
            stop: stop_strs,
            tools,
        };

        let url = format!("{}/chat/completions", self.base_url);

        let raw = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| MacacaError::Llm(format!("DashScope request failed: {e}")))?;

        if !raw.status().is_success() {
            let status = raw.status();
            let text = crate::error_sanitize::sanitize_provider_error_body(
                &raw.text().await.unwrap_or_default(),
            );
            return Err(MacacaError::Llm(format!(
                "DashScope API error {status}: {text}"
            )));
        }

        let resp: ChatResponse = raw
            .json()
            .await
            .map_err(|e| MacacaError::Llm(format!("DashScope response parse failed: {e}")))?;

        let choice = resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| MacacaError::Llm("DashScope returned no choices".into()))?;

        let content = choice.message.content.unwrap_or_default();
        let finish_reason = choice.finish_reason.unwrap_or_else(|| "stop".into());
        let tool_calls = choice.message.tool_calls.map(parse_tool_calls);

        Ok(LlmResponse {
            content,
            reasoning_content: None,
            model: resp.model,
            usage: TokenUsage {
                prompt_tokens: resp.usage.prompt_tokens,
                completion_tokens: resp.usage.completion_tokens,
                total_tokens: resp.usage.total_tokens,
            },
            finish_reason,
            tool_calls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_fails_without_key() {
        std::env::remove_var("DASHSCOPE_API_KEY");
        assert!(DashScopeProvider::from_env().is_err());
    }

    #[test]
    fn provider_name() {
        let p = DashScopeProvider::new("key");
        assert_eq!(p.name(), "dashscope");
    }

    #[test]
    fn custom_base_url() {
        let p = DashScopeProvider::new("key").with_base_url("http://localhost:8000");
        assert_eq!(p.base_url, "http://localhost:8000");
    }

    #[test]
    fn role_mapping() {
        assert_eq!(role_str(LlmRole::System), "system");
        assert_eq!(role_str(LlmRole::User), "user");
        assert_eq!(role_str(LlmRole::Assistant), "assistant");
        assert_eq!(role_str(LlmRole::Tool), "tool");
    }
}
