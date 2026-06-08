//! Mock client lifecycle and tool-call contract tests.

use std::sync::Arc;

use crate::message::{ContentBlock, TextBlock};
use crate::tool::ToolHandler;

use super::mock::MockMcpClient;
use super::super::core::McpClient;
use super::super::error::McpError;
use super::super::types::McpCallResult;

#[tokio::test]
async fn test_mock_client_connect_and_list() {
    let mut client = MockMcpClient::new()
        .with_tool("tool_a", "Tool A description")
        .with_tool("tool_b", "Tool B description");

    assert!(!client.is_connected());

    client.connect().await.unwrap();
    assert!(client.is_connected());

    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "tool_a");
    assert_eq!(tools[1].name, "tool_b");
}

#[tokio::test]
async fn test_mock_client_call_tool() {
    let result = McpCallResult {
        content: vec![ContentBlock::Text(TextBlock {
            text: "hello from mcp".to_string(),
        })],
        is_error: false,
        metadata: None,
    };

    let mut client = MockMcpClient::new()
        .with_tool("greet", "Greets the user")
        .with_response("greet", result);

    client.connect().await.unwrap();

    let call_result = client
        .call_tool("greet", serde_json::json!({"name": "world"}))
        .await
        .unwrap();

    assert!(!call_result.is_error);
    assert_eq!(call_result.content.len(), 1);
    if let ContentBlock::Text(tb) = &call_result.content[0] {
        assert_eq!(tb.text, "hello from mcp");
    } else {
        panic!("expected TextBlock");
    }
}

#[tokio::test]
async fn test_mock_client_not_connected() {
    let mut client = MockMcpClient::new().with_tool("t", "desc");

    let err = client
        .call_tool("t", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, McpError::NotConnected));
}

#[tokio::test]
async fn test_mock_client_already_connected() {
    let mut client = MockMcpClient::new();
    client.connect().await.unwrap();

    let err = client.connect().await.unwrap_err();
    assert!(matches!(err, McpError::AlreadyConnected));
}

#[tokio::test]
async fn test_mock_client_close() {
    let mut client = MockMcpClient::new();
    client.connect().await.unwrap();
    assert!(client.is_connected());

    client.close().await.unwrap();
    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_mcp_tool_handler_as_tool() {
    use super::super::adapter::McpToolHandler;
    use super::super::types::McpToolDef;

    let result = McpCallResult {
        content: vec![ContentBlock::Text(TextBlock {
            text: "result data".to_string(),
        })],
        is_error: false,
        metadata: Some(serde_json::json!({"tokens": 42})),
    };

    let mut mock = MockMcpClient::new()
        .with_tool("my_tool", "Does something")
        .with_response("my_tool", result);
    mock.connect().await.unwrap();

    let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
        Arc::new(tokio::sync::RwLock::new(mock));

    let tool_def = McpToolDef {
        name: "my_tool".to_string(),
        description: "Does something".to_string(),
        input_schema: serde_json::json!({"type": "object"}),
    };

    let handler = McpToolHandler::new(client, tool_def);

    assert_eq!(handler.name(), "my_tool");
    assert_eq!(handler.description(), "Does something");
    assert_eq!(handler.schema(), serde_json::json!({"type": "object"}));

    let resp = handler.execute(serde_json::json!({})).await.unwrap();
    assert_eq!(resp.content.len(), 1);
    if let ContentBlock::Text(tb) = &resp.content[0] {
        assert_eq!(tb.text, "result data");
    } else {
        panic!("expected TextBlock");
    }
    assert_eq!(resp.metadata, Some(serde_json::json!({"tokens": 42})));
}
