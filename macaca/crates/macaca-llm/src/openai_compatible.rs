//! Generic OpenAI-compatible LLM provider.
//!
//! Works with any API that implements the OpenAI chat/completions format:
//! vLLM, Ollama, LM Studio, DeepSeek, Together AI, Groq, etc.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use macaca_proto::{
    error::{MacacaError, MacacaResult},
    types::{LlmMessage, LlmOptions, LlmResponse, LlmRole, TokenUsage, ToolCall},
};

use crate::provider::LlmProvider;
use crate::tool_wire::tool_arguments_for_chat_api;

/// A generic provider for any OpenAI-compatible API endpoint.
pub struct OpenAiCompatibleProvider {
    provider_name: String,
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let base_url = base_url.into().trim().trim_end_matches('/').to_string();
        Self {
            provider_name: name.into(),
            api_key: api_key.into(),
            base_url,
            client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn from_env(
        name: impl Into<String>,
        base_url: impl Into<String>,
        env_var_name: &str,
    ) -> MacacaResult<Self> {
        let api_key = if env_var_name.is_empty() {
            String::new()
        } else {
            std::env::var(env_var_name)
                .map_err(|_| MacacaError::Config(format!("{env_var_name} not set")))?
        };
        Ok(Self::new(name, base_url, api_key))
    }
}

// ── Wire types ───────────────────────────────────────────────────────────────

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
    #[serde(default)]
    usage: Option<UsageInfo>,
    model: Option<String>,
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
struct UsageInfo {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
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
impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.provider_name
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
        tracing::info!(provider = %self.provider_name, model = %options.model, url = %url, "OpenAI-compatible chat request");

        let mut req = self.client.post(&url).json(&body);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let raw = req
            .send()
            .await
            .map_err(|e| MacacaError::Llm(format!("{} request failed: {e}", self.provider_name)))?;

        if !raw.status().is_success() {
            let status = raw.status();
            let text = raw.text().await.unwrap_or_default();
            return Err(MacacaError::Llm(format!(
                "{} API error {status}: {text}",
                self.provider_name
            )));
        }

        let raw_text = raw.text().await.map_err(|e| {
            MacacaError::Llm(format!("{} response read failed: {e}", self.provider_name))
        })?;
        let resp: ChatResponse = serde_json::from_str(&raw_text).map_err(|e| {
            tracing::error!(provider = %self.provider_name, body = %&raw_text[..raw_text.len().min(500)], "Failed to parse LLM response");
            MacacaError::Llm(format!("{} response parse failed: {e}", self.provider_name))
        })?;

        let choice = resp.choices.into_iter().next().ok_or_else(|| {
            MacacaError::Llm(format!("{} returned no choices", self.provider_name))
        })?;

        let content = choice.message.content.unwrap_or_default();
        let finish_reason = choice.finish_reason.unwrap_or_else(|| "stop".into());
        let model = resp.model.unwrap_or_else(|| options.model.clone());
        let tool_calls = choice.message.tool_calls.map(parse_tool_calls);

        let usage = resp.usage.unwrap_or(UsageInfo {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });

        Ok(LlmResponse {
            content,
            model,
            usage: TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
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
    fn provider_name_is_custom() {
        let p = OpenAiCompatibleProvider::new("ollama", "http://localhost:11434/v1", "");
        assert_eq!(p.name(), "ollama");
    }

    #[test]
    fn empty_api_key_is_allowed() {
        let p = OpenAiCompatibleProvider::new("local", "http://localhost:8000/v1", "");
        assert!(p.api_key.is_empty());
    }

    #[test]
    fn from_env_empty_key_env_succeeds() {
        let p = OpenAiCompatibleProvider::from_env("local", "http://localhost:8000/v1", "");
        assert!(p.is_ok());
    }

    #[test]
    fn from_env_missing_key_fails() {
        std::env::remove_var("NONEXISTENT_KEY_FOR_TEST");
        let p = OpenAiCompatibleProvider::from_env(
            "test",
            "http://localhost:8000/v1",
            "NONEXISTENT_KEY_FOR_TEST",
        );
        assert!(p.is_err());
    }

    #[test]
    fn role_mapping() {
        assert_eq!(role_str(LlmRole::System), "system");
        assert_eq!(role_str(LlmRole::User), "user");
        assert_eq!(role_str(LlmRole::Assistant), "assistant");
        assert_eq!(role_str(LlmRole::Tool), "tool");
    }
}
