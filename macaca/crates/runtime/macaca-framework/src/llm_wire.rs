//! Bidirectional mapping between OpenAI-style chat JSON and `macaca_proto::LlmMessage`.
//!
//! ### Why this module exists (not only `adapter`)
//! - The `adapter` module is behind the `macaca-compat` feature; the ReAct path needs this bridge
//!   unconditionally so it can call `macaca_context::ContextFacade` with the same carrier types as
//!   the LLM stack.
//! - Avoid duplicating half of the JSON parsing logic inside `react_agent`.
//!
//! ### Contract
//! - **Approximate inverse**: `messages_from_json_values` and `llm_messages_to_json_values` are
//!   mutual inverses for the supported subset (whitespace / field-order differences are not
//!   guaranteed to round-trip bitwise).

use macaca_proto::{LlmMessage, LlmOptions, LlmRole, ToolCall, ToolDefinition};
use serde_json::{json, Value};

use crate::model::ChatOptions;

/// Maps framework [`ChatOptions`] to OS-level `LlmOptions` for [`ContextFacade`] and providers.
pub fn chat_options_to_llm_options(options: &ChatOptions) -> LlmOptions {
    let mut llm = LlmOptions {
        model: options.model.clone().unwrap_or_default(),
        temperature: options.temperature,
        max_tokens: options.max_tokens,
        ..Default::default()
    };
    if let Some(ref tools) = options.tools {
        llm.tools = Some(
            tools
                .iter()
                .map(|t| ToolDefinition {
                    name: t["name"].as_str().unwrap_or("").to_string(),
                    description: t["description"].as_str().unwrap_or("").to_string(),
                    parameters: t["parameters"].clone(),
                })
                .collect(),
        );
    }
    llm
}

/// OpenAI `messages` JSON array → protocol `LlmMessage` list.
///
/// Same logic historically lived in `adapter::messages_from_json`; exposed here as the stable API.
pub fn messages_from_json_values(messages: &[Value]) -> Vec<LlmMessage> {
    messages
        .iter()
        .filter_map(|msg| {
            let role_str = msg["role"].as_str()?;
            let content = msg["content"].as_str().unwrap_or("").to_string();
            let reasoning_content = msg["reasoning_content"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_string);

            match role_str {
                "system" => Some(LlmMessage::system(content)),
                "user" => Some(LlmMessage::user(content)),
                "assistant" => {
                    if let Some(tool_calls) = msg["tool_calls"].as_array() {
                        let tcs: Vec<ToolCall> = tool_calls
                            .iter()
                            .filter_map(|tc| {
                                let name = tc["function"]["name"]
                                    .as_str()
                                    .or_else(|| tc["name"].as_str())?
                                    .to_string();
                                let args = if let Some(fa) = tc["function"].get("arguments") {
                                    if let Some(s) = fa.as_str() {
                                        serde_json::from_str(s).unwrap_or(serde_json::json!({}))
                                    } else {
                                        fa.clone()
                                    }
                                } else {
                                    tc.get("input").cloned().unwrap_or(serde_json::json!({}))
                                };
                                Some(ToolCall {
                                    id: tc["id"].as_str().unwrap_or("").to_string(),
                                    name,
                                    arguments: args,
                                })
                            })
                            .collect();
                        if tcs.is_empty() {
                            let mut message = LlmMessage::assistant(content);
                            message.reasoning_content = reasoning_content;
                            Some(message)
                        } else {
                            let mut message = LlmMessage::assistant_with_tool_calls(content, tcs);
                            message.reasoning_content = reasoning_content;
                            Some(message)
                        }
                    } else {
                        let mut message = LlmMessage::assistant(content);
                        message.reasoning_content = reasoning_content;
                        Some(message)
                    }
                }
                "tool" => {
                    let tool_call_id = msg["tool_call_id"].as_str().unwrap_or("");
                    Some(LlmMessage::tool_result(tool_call_id, content))
                }
                _ => None,
            }
        })
        .collect()
}

/// Protocol messages → OpenAI chat JSON for `ChatModel::chat` inputs.
pub fn llm_messages_to_json_values(messages: &[LlmMessage]) -> Vec<Value> {
    messages.iter().map(single_llm_to_json).collect()
}

fn single_llm_to_json(m: &LlmMessage) -> Value {
    match m.role {
        LlmRole::System => json!({"role":"system","content": m.content}),
        LlmRole::User => json!({"role":"user","content": m.content}),
        LlmRole::Assistant => {
            let mut v = serde_json::Map::new();
            v.insert("role".into(), json!("assistant"));
            v.insert("content".into(), json!(m.content));
            if let Some(ref rc) = m.reasoning_content {
                v.insert("reasoning_content".into(), json!(rc));
            }
            if let Some(ref tcs) = m.tool_calls {
                let arr: Vec<Value> = tcs
                    .iter()
                    .map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments.to_string()
                            }
                        })
                    })
                    .collect();
                v.insert("tool_calls".into(), Value::Array(arr));
            }
            Value::Object(v)
        }
        LlmRole::Tool => json!({
            "role": "tool",
            "tool_call_id": m.tool_call_id.clone().unwrap_or_default(),
            "content": m.content,
        }),
    }
}
