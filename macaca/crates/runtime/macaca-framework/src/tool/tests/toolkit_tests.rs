//! Registry, group policy, and preset-arg contract tests for [`Toolkit`].

use crate::message::ContentBlock;
use crate::tool::{ToolError, Toolkit};
use serde_json::Value;

use super::fixtures::{AddTool, EchoTool, NamedEchoTool};

#[tokio::test]
async fn test_register_and_call() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(AddTool), None);

    let resp = kit
        .call_tool("add", serde_json::json!({"a": 3.0, "b": 4.0}))
        .await
        .unwrap();

    assert_eq!(resp.content.len(), 1);
    if let ContentBlock::Text(tb) = &resp.content[0] {
        assert_eq!(tb.text, "7");
    } else {
        panic!("expected TextBlock");
    }
}

#[tokio::test]
async fn test_tool_not_found() {
    let kit = Toolkit::new();
    let err = kit
        .call_tool("nonexistent", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotFound(_)));
}

#[tokio::test]
async fn test_preset_args() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(EchoTool), None);

    kit.tools.get_mut("echo").unwrap().preset_args =
        serde_json::json!({"preset_key": "preset_val"});

    let resp = kit
        .call_tool("echo", serde_json::json!({"caller_key": "caller_val"}))
        .await
        .unwrap();

    if let ContentBlock::Text(tb) = &resp.content[0] {
        let v: Value = serde_json::from_str(&tb.text).unwrap();
        assert_eq!(v["preset_key"], "preset_val");
        assert_eq!(v["caller_key"], "caller_val");
    } else {
        panic!("expected TextBlock");
    }
}

#[tokio::test]
async fn test_group_active() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(EchoTool), Some("optional"));

    kit.call_tool("echo", serde_json::json!({})).await.unwrap();

    kit.set_group_active("optional", false);

    let err = kit
        .call_tool("echo", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(_)));
}

#[tokio::test]
async fn test_get_definitions() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(EchoTool), Some("group_a"));
    kit.register(Box::new(AddTool), Some("group_b"));

    let defs = kit.get_definitions();
    assert_eq!(defs.len(), 2);

    kit.set_group_active("group_a", false);
    let defs = kit.get_definitions();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0]["name"], "add");
}

#[tokio::test]
async fn test_unregister() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(AddTool), None);
    assert_eq!(kit.tool_count(), 1);

    kit.unregister("add");
    assert_eq!(kit.tool_count(), 0);

    let err = kit
        .call_tool("add", serde_json::json!({"a": 1.0, "b": 2.0}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotFound(_)));
}

#[tokio::test]
async fn test_basic_group_always_active() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(AddTool), None);

    kit.set_group_active("basic", false);

    kit.call_tool("add", serde_json::json!({"a": 10.0, "b": 5.0}))
        .await
        .unwrap();

    let defs = kit.get_definitions();
    assert_eq!(defs.len(), 1);
}

#[tokio::test]
async fn test_register_duplicate_tool_name() {
    let mut kit = Toolkit::new();
    kit.register(
        Box::new(NamedEchoTool {
            tool_name: "dup".into(),
        }),
        None,
    );
    kit.register(
        Box::new(NamedEchoTool {
            tool_name: "dup".into(),
        }),
        None,
    );
    assert_eq!(kit.tool_count(), 1);
    kit.call_tool("dup", serde_json::json!({})).await.unwrap();
}

#[tokio::test]
async fn test_call_nonexistent_tool() {
    let kit = Toolkit::new();
    let err = kit
        .call_tool("does_not_exist", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotFound(_)));
    assert!(err.to_string().contains("does_not_exist"));
}

#[tokio::test]
async fn test_disabled_group_tool_rejected() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(EchoTool), Some("optional"));

    kit.set_group_active("optional", false);

    let err = kit
        .call_tool("echo", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(_)));
    assert!(err.to_string().contains("inactive group"));
}

#[tokio::test]
async fn test_basic_group_cannot_disable() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(AddTool), None);

    kit.set_group_active("basic", false);

    let group = kit.groups.get("basic").unwrap();
    assert!(group.active);

    kit.call_tool("add", serde_json::json!({"a": 1.0, "b": 1.0}))
        .await
        .unwrap();

    assert_eq!(kit.get_definitions().len(), 1);
}

#[tokio::test]
async fn test_empty_toolkit_definitions() {
    let kit = Toolkit::new();
    let defs = kit.get_definitions();
    assert!(defs.is_empty());
}

#[tokio::test]
async fn test_preset_kwargs_merge() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(EchoTool), None);

    kit.tools.get_mut("echo").unwrap().preset_args =
        serde_json::json!({"preset_a": "from_preset", "shared": "preset_wins_not"});

    let resp = kit
        .call_tool(
            "echo",
            serde_json::json!({"shared": "caller_wins", "caller_b": 42}),
        )
        .await
        .unwrap();

    if let ContentBlock::Text(tb) = &resp.content[0] {
        let v: Value = serde_json::from_str(&tb.text).unwrap();
        assert_eq!(v["preset_a"], "from_preset");
        assert_eq!(v["shared"], "caller_wins");
        assert_eq!(v["caller_b"], 42);
    } else {
        panic!("expected TextBlock");
    }
}
