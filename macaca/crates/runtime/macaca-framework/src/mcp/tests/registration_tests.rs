//! Toolkit registration policy contract tests.

use std::collections::HashSet;
use std::sync::Arc;

use crate::message::{ContentBlock, TextBlock};
use crate::tool::{ToolError, Toolkit};

use super::mock::{LocalEchoTool, MockMcpClient};
use super::super::core::McpClient;
use super::super::error::McpError;
use super::super::registration::{register_mcp_tools, register_mcp_tools_with_options};
use super::super::types::{
    McpCallResult, McpToolNameConflictPolicy, McpToolRegistrationOptions,
};

#[tokio::test]
async fn test_register_mcp_tools() {
    let result_a = McpCallResult {
        content: vec![ContentBlock::Text(TextBlock {
            text: "a_result".to_string(),
        })],
        is_error: false,
        metadata: None,
    };
    let result_b = McpCallResult {
        content: vec![ContentBlock::Text(TextBlock {
            text: "b_result".to_string(),
        })],
        is_error: false,
        metadata: None,
    };

    let mut mock = MockMcpClient::new()
        .with_tool("mcp_a", "Tool A")
        .with_tool("mcp_b", "Tool B")
        .with_response("mcp_a", result_a)
        .with_response("mcp_b", result_b);
    mock.connect().await.unwrap();

    let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
        Arc::new(tokio::sync::RwLock::new(mock));

    let mut toolkit = Toolkit::new();
    register_mcp_tools(&mut toolkit, client, "mcp_group")
        .await
        .unwrap();

    assert_eq!(toolkit.tool_count(), 2);
    assert!(toolkit.get_tool("mcp_a").is_some());
    assert!(toolkit.get_tool("mcp_b").is_some());

    let resp = toolkit
        .call_tool("mcp_a", serde_json::json!({}))
        .await
        .unwrap();
    if let ContentBlock::Text(tb) = &resp.content[0] {
        assert_eq!(tb.text, "a_result");
    } else {
        panic!("expected TextBlock");
    }
}

#[tokio::test]
async fn test_call_nonexistent_mcp_tool() {
    let mut mock = MockMcpClient::new().with_tool("exists", "Exists");
    mock.connect().await.unwrap();

    let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
        Arc::new(tokio::sync::RwLock::new(mock));

    let mut toolkit = Toolkit::new();
    register_mcp_tools(&mut toolkit, client, "mcp")
        .await
        .unwrap();

    let err = toolkit
        .call_tool("does_not_exist", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotFound(_)));

    let err = toolkit
        .call_tool("exists", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::ExecutionFailed(_)));
}

#[tokio::test]
async fn test_register_mcp_tools_raises_on_collision() {
    let mut mock = MockMcpClient::new().with_tool("exists", "MCP Exists");
    mock.connect().await.unwrap();
    let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
        Arc::new(tokio::sync::RwLock::new(mock));

    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(LocalEchoTool::new("exists")), None);

    let err = register_mcp_tools_with_options(
        &mut toolkit,
        client,
        McpToolRegistrationOptions::new("mcp"),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, McpError::ToolNameCollision(_)));
}

#[tokio::test]
async fn test_register_mcp_tools_prefixes_collision() {
    let result = McpCallResult {
        content: vec![ContentBlock::Text(TextBlock {
            text: "mcp result".to_string(),
        })],
        is_error: false,
        metadata: None,
    };
    let mut mock = MockMcpClient::new()
        .with_tool("exists", "MCP Exists")
        .with_response("exists", result);
    mock.connect().await.unwrap();
    let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
        Arc::new(tokio::sync::RwLock::new(mock));

    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(LocalEchoTool::new("exists")), None);

    register_mcp_tools_with_options(
        &mut toolkit,
        client,
        McpToolRegistrationOptions {
            group_name: "mcp".to_string(),
            conflict_policy: McpToolNameConflictPolicy::Prefix("mcp_".to_string()),
            disabled_tools: HashSet::new(),
            on_close: None,
        },
    )
    .await
    .unwrap();

    assert!(toolkit.get_tool("exists").is_some());
    assert!(toolkit.get_tool("mcp_exists").is_some());
    let resp = toolkit
        .call_tool("mcp_exists", serde_json::json!({}))
        .await
        .unwrap();
    match &resp.content[0] {
        ContentBlock::Text(text) => assert_eq!(text.text, "mcp result"),
        _ => panic!("expected text"),
    }
}

#[tokio::test]
async fn test_register_mcp_tools_skips_disabled_tools() {
    let mut mock = MockMcpClient::new()
        .with_tool("allowed", "Allowed")
        .with_tool("blocked", "Blocked");
    mock.connect().await.unwrap();
    let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
        Arc::new(tokio::sync::RwLock::new(mock));

    let mut disabled_tools = HashSet::new();
    disabled_tools.insert("blocked".to_string());
    let mut toolkit = Toolkit::new();
    let registered = register_mcp_tools_with_options(
        &mut toolkit,
        client,
        McpToolRegistrationOptions {
            group_name: "mcp".to_string(),
            conflict_policy: McpToolNameConflictPolicy::Raise,
            disabled_tools,
            on_close: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(registered, vec!["allowed"]);
    assert!(toolkit.get_tool("allowed").is_some());
    assert!(toolkit.get_tool("blocked").is_none());
}
