use crate::provider::LlmProvider;
use async_trait::async_trait;
use macaca_proto::{
    error::{MacacaError, MacacaResult},
    types::{LlmMessage, LlmOptions, LlmResponse, LlmRole, TokenUsage, ToolCall},
};
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn from_env() -> MacacaResult<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| MacacaError::Config("ANTHROPIC_API_KEY not set".into()))?;
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

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

// ── Wire types ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicToolDef>>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

/// Anthropic content can be a plain string or an array of content blocks.
#[derive(Serialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize)]
struct AnthropicToolDef {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ResponseContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ResponseContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

fn convert_messages(messages: &[LlmMessage]) -> (Option<String>, Vec<AnthropicMessage>) {
    let system_content: Option<String> = messages
        .iter()
        .find(|m| m.role == LlmRole::System)
        .map(|m| m.content.clone());

    let mut result = Vec::new();

    for m in messages {
        match m.role {
            LlmRole::System => continue,
            LlmRole::User => {
                result.push(AnthropicMessage {
                    role: "user".into(),
                    content: AnthropicContent::Text(m.content.clone()),
                });
            }
            LlmRole::Assistant => {
                if let Some(ref calls) = m.tool_calls {
                    let mut blocks: Vec<AnthropicContentBlock> = Vec::new();
                    if !m.content.is_empty() {
                        blocks.push(AnthropicContentBlock::Text {
                            text: m.content.clone(),
                        });
                    }
                    for tc in calls {
                        blocks.push(AnthropicContentBlock::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: tc.arguments.clone(),
                        });
                    }
                    result.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: AnthropicContent::Blocks(blocks),
                    });
                } else {
                    result.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: AnthropicContent::Text(m.content.clone()),
                    });
                }
            }
            LlmRole::Tool => {
                let tool_use_id = m.tool_call_id.clone().unwrap_or_default();
                result.push(AnthropicMessage {
                    role: "user".into(),
                    content: AnthropicContent::Blocks(vec![AnthropicContentBlock::ToolResult {
                        tool_use_id,
                        content: m.content.clone(),
                    }]),
                });
            }
        }
    }

    (system_content, result)
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let (system, anthropic_messages) = convert_messages(&messages);
        let stop_strs: Vec<String> = options.stop_sequences.clone();
        let max_tokens = options.max_tokens.unwrap_or(4096);

        let tools = options.tools.as_ref().map(|defs| {
            defs.iter()
                .map(|d| AnthropicToolDef {
                    name: d.name.clone(),
                    description: d.description.clone(),
                    input_schema: d.parameters.clone(),
                })
                .collect()
        });

        let body = MessagesRequest {
            model: options.model.clone(),
            max_tokens,
            messages: anthropic_messages,
            system,
            temperature: options.temperature,
            stop_sequences: stop_strs,
            tools,
        };

        let url = format!("{}/messages", self.base_url);

        let raw = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await
            .map_err(|e| MacacaError::Llm(format!("Anthropic request failed: {e}")))?;

        if !raw.status().is_success() {
            let status = raw.status();
            let text = raw.text().await.unwrap_or_default();
            return Err(MacacaError::Llm(format!(
                "Anthropic API error {status}: {text}"
            )));
        }

        let resp: MessagesResponse = raw
            .json()
            .await
            .map_err(|e| MacacaError::Llm(format!("Anthropic response parse failed: {e}")))?;

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for block in resp.content {
            match block {
                ResponseContentBlock::Text { text } => text_parts.push(text),
                ResponseContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments: input,
                    });
                }
            }
        }

        let content = text_parts.join("");
        let finish_reason = resp.stop_reason.unwrap_or_else(|| "end_turn".into());
        let total = resp.usage.input_tokens + resp.usage.output_tokens;
        let tool_calls_opt = if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        };

        Ok(LlmResponse {
            content,
            model: resp.model,
            usage: TokenUsage {
                prompt_tokens: resp.usage.input_tokens,
                completion_tokens: resp.usage.output_tokens,
                total_tokens: total,
            },
            finish_reason,
            tool_calls: tool_calls_opt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_fails_without_key() {
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(AnthropicProvider::from_env().is_err());
    }

    #[test]
    fn provider_name() {
        let p = AnthropicProvider::new("key");
        assert_eq!(p.name(), "anthropic");
    }

    #[test]
    fn convert_messages_extracts_system() {
        let msgs = vec![
            LlmMessage::system("you are helpful"),
            LlmMessage::user("hello"),
        ];
        let (sys, converted) = convert_messages(&msgs);
        assert_eq!(sys.as_deref(), Some("you are helpful"));
        assert_eq!(converted.len(), 1); // only user message
    }
}
