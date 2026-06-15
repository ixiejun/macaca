//! Tool response text extraction and trace output truncation.

use macaca_host_composition::framework::tool::ToolResponse;

pub(crate) const TOOL_TRACE_OUTPUT_MAX_BYTES: usize = 2000;

pub(crate) fn truncate_tool_output(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let mut end = max_bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}...[truncated, {} bytes]", &text[..end], text.len())
}

pub(crate) fn tool_response_text(response: &ToolResponse) -> String {
    response
        .content
        .iter()
        .filter_map(|block| match block {
            macaca_host_composition::framework::message::ContentBlock::Text(text) => {
                Some(text.text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

pub(crate) fn tool_trace_output(response: &ToolResponse) -> String {
    truncate_tool_output(&tool_response_text(response), TOOL_TRACE_OUTPUT_MAX_BYTES)
}

pub(crate) fn tool_call_event(
    name: &str,
    args: &serde_json::Value,
) -> macaca_proto::AgentExecutionEvent {
    macaca_proto::AgentExecutionEvent::ToolCall {
        tool_name: name.to_string(),
        tool_input: args.clone(),
        call_id: None,
    }
}

pub(crate) fn tool_result_event(name: &str, output: String) -> macaca_proto::AgentExecutionEvent {
    macaca_proto::AgentExecutionEvent::ToolResult {
        tool_name: name.to_string(),
        output,
        is_error: None,
    }
}
