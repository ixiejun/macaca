use macaca_framework::model::ChatOptions;
use macaca_proto::{LlmMessage, LlmOptions};

/// Convert framework chat options into the LLM-neutral context-engine DTO.
///
/// The context engine should not depend on framework-specific JSON shapes. This
/// adapter is the Anti-Corruption Layer between the web/framework boundary and
/// the portable `macaca_proto` prompt contract.
pub(crate) fn framework_options_to_llm(options: &ChatOptions) -> macaca_proto::LlmOptions {
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

/// Convert assembled LLM options back into framework chat options.
///
/// Only context-engine-owned fields are overridden. Framework-only settings
/// such as tool choice are preserved so this wrapper remains transparent.
pub(crate) fn llm_options_to_framework(
    options: &LlmOptions,
    original: &ChatOptions,
) -> ChatOptions {
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

/// Convert framework JSON messages into `LlmMessage` values for context assembly.
///
/// Unknown roles are ignored because the context engine can only reason about
/// the stable proto roles. The original message list remains available to the
/// underlying model when no context transformation is applied.
pub(crate) fn framework_messages_to_llm(
    messages: &[serde_json::Value],
) -> Vec<macaca_proto::LlmMessage> {
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

/// Convert assembled `LlmMessage` values back into framework JSON format.
pub(crate) fn llm_messages_to_framework(messages: &[LlmMessage]) -> Vec<serde_json::Value> {
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

/// Extract the latest user-visible text from framework message JSON.
///
/// Active recall uses the latest user utterance as a recall query. This helper
/// intentionally lives with the codec because the query extraction depends on
/// the framework message representation, not on the memory subsystem.
pub(crate) fn last_user_text_from_framework(messages: &[serde_json::Value]) -> Option<String> {
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

/// Best-effort extraction of textual content from framework message payloads.
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

/// Rebuild an assistant message, preserving tool-call structure when present.
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

/// Parse tool-call arguments whether they arrive as a JSON string or raw JSON.
fn parse_tool_arguments(value: &serde_json::Value) -> serde_json::Value {
    value
        .as_str()
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or_else(|| value.clone())
}

/// Convert one assistant message back into framework JSON, preserving tool calls.
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
