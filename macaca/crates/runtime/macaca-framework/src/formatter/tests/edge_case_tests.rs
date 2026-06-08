//! Cross-formatter edge-case contract tests.

use serde_json::json;

use crate::formatter::{AnthropicFormatter, DashScopeFormatter, Formatter, OpenAiFormatter};
use crate::message::{
    ContentBlock, ImageBlock, Msg, MsgContent, Role, TextBlock, ThinkingBlock, ToolResultBlock,
    ToolUseBlock,
};

// -----------------------------------------------------------------------
// OpenAiFormatter
// -----------------------------------------------------------------------

#[test]
fn test_format_mixed_content_blocks() {
    // A message with Text + ToolUse + Image blocks.
    let blocks = vec![
        ContentBlock::Text(TextBlock {
            text: "Here is the result.".into(),
        }),
        ContentBlock::ToolUse(ToolUseBlock {
            id: "tu_mix".into(),
            name: "search".into(),
            input: json!({"q": "test"}),
            raw_input: None,
        }),
        ContentBlock::Image(ImageBlock {
            url: Some("https://img.example.com/pic.png".into()),
            data: None,
            mime_type: None,
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));

    // OpenAI: tool_calls present, text present, image not in tool_calls path
    let openai = OpenAiFormatter;
    let openai_result = openai.format(&[msg.clone()]);
    assert_eq!(openai_result[0]["role"], "assistant");
    let tool_calls = openai_result[0]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["function"]["name"], "search");

    // Anthropic: all blocks appear in content array
    let anthropic = AnthropicFormatter;
    let anthropic_result = anthropic.format(&[msg.clone()]);
    let content = anthropic_result[0]["content"].as_array().unwrap();
    let types: Vec<&str> = content
        .iter()
        .map(|c| c["type"].as_str().unwrap())
        .collect();
    assert!(types.contains(&"text"));
    assert!(types.contains(&"tool_use"));
    assert!(types.contains(&"image"));

    // DashScope: same as OpenAI
    let dashscope = DashScopeFormatter;
    let ds_result = dashscope.format(&[msg]);
    assert_eq!(ds_result[0]["role"], "assistant");
    assert!(ds_result[0]["tool_calls"].as_array().unwrap().len() == 1);
}

// -----------------------------------------------------------------------
// Additional robustness tests
// -----------------------------------------------------------------------
