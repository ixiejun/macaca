//! Contract tests for shared `model_impls` helpers (option merge, tool-choice encoding).

use serde_json::json;

use super::super::helpers::{
    merge_options, tool_choice_to_anthropic_value, tool_choice_to_openai_value,
};
use crate::model::{ChatOptions, ToolChoice};

#[test]
fn test_merge_options_prefers_options() {
    let defaults = ChatOptions {
        temperature: Some(0.5),
        max_tokens: Some(2048),
        ..Default::default()
    };
    let options = ChatOptions {
        temperature: Some(1.0),
        ..Default::default()
    };

    let merged = merge_options(&options, &defaults);
    assert_eq!(merged.temperature, Some(1.0));
    assert_eq!(merged.max_tokens, Some(2048));
}

#[test]
fn test_merge_options_uses_defaults_when_none() {
    let defaults = ChatOptions {
        temperature: Some(0.5),
        max_tokens: Some(2048),
        top_p: Some(0.9),
        ..Default::default()
    };
    let options = ChatOptions::default();

    let merged = merge_options(&options, &defaults);
    assert_eq!(merged.temperature, Some(0.5));
    assert_eq!(merged.max_tokens, Some(2048));
    assert_eq!(merged.top_p, Some(0.9));
}

#[test]
fn test_tool_choice_openai_values() {
    assert_eq!(
        tool_choice_to_openai_value(&ToolChoice::Auto),
        json!("auto")
    );
    assert_eq!(
        tool_choice_to_openai_value(&ToolChoice::None),
        json!("none")
    );
    assert_eq!(
        tool_choice_to_openai_value(&ToolChoice::Required),
        json!("required")
    );
    assert_eq!(
        tool_choice_to_openai_value(&ToolChoice::Specific("foo".into())),
        json!({"type": "function", "function": {"name": "foo"}})
    );
}

#[test]
fn test_tool_choice_anthropic_values() {
    assert_eq!(
        tool_choice_to_anthropic_value(&ToolChoice::Auto),
        json!({"type": "auto"})
    );
    assert_eq!(
        tool_choice_to_anthropic_value(&ToolChoice::None),
        json!({"type": "none"})
    );
    assert_eq!(
        tool_choice_to_anthropic_value(&ToolChoice::Required),
        json!({"type": "any"})
    );
    assert_eq!(
        tool_choice_to_anthropic_value(&ToolChoice::Specific("bar".into())),
        json!({"type": "tool", "name": "bar"})
    );
}
