//! Shared helpers for legacy HTTP [`ChatModel`] adapters.
//!
//! Option merging, numeric serialization, and tool-choice wire encoding are
//! centralized here so protocol-specific adapters stay focused on request/response
//! shape translation (Adapter pattern).

use serde_json::{json, Value};

use crate::model::{ChatOptions, ToolChoice};

/// Merge `options` on top of `defaults`, preferring values in `options`.
pub(crate) fn merge_options<'a>(options: &'a ChatOptions, defaults: &'a ChatOptions) -> ChatOptions {
    ChatOptions {
        model: options.model.clone().or_else(|| defaults.model.clone()),
        temperature: options.temperature.or(defaults.temperature),
        max_tokens: options.max_tokens.or(defaults.max_tokens),
        top_p: options.top_p.or(defaults.top_p),
        tools: options.tools.clone().or_else(|| defaults.tools.clone()),
        tool_choice: options
            .tool_choice
            .clone()
            .or_else(|| defaults.tool_choice.clone()),
    }
}

/// Convert f32 to a clean f64 by going through string representation.
/// This avoids f32 precision artifacts like 0.7_f32 becoming 0.699999988079071.
pub(crate) fn f32_to_clean_f64(v: f32) -> f64 {
    // Format with enough precision for f32, then parse as f64
    let s = format!("{}", v);
    s.parse::<f64>().unwrap_or(v as f64)
}

pub(crate) fn tool_choice_to_openai_value(tc: &ToolChoice) -> Value {
    match tc {
        ToolChoice::Auto => json!("auto"),
        ToolChoice::None => json!("none"),
        ToolChoice::Required => json!("required"),
        ToolChoice::Specific(name) => json!({"type": "function", "function": {"name": name}}),
    }
}

pub(crate) fn tool_choice_to_anthropic_value(tc: &ToolChoice) -> Value {
    match tc {
        ToolChoice::Auto => json!({"type": "auto"}),
        ToolChoice::None => json!({"type": "none"}),
        ToolChoice::Required => json!({"type": "any"}),
        ToolChoice::Specific(name) => json!({"type": "tool", "name": name}),
    }
}
