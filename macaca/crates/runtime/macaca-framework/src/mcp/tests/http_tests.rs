//! HTTP MCP transport contract tests (`mcp-http` feature).

use std::collections::BTreeMap;

use super::super::core::McpClient;
use super::super::factory::client_from_transport;
use super::super::http::parse_http_mcp_response;
use super::super::types::{McpTimeouts, McpTransportConfig};

#[test]
fn test_parse_streamable_http_json_response() {
    let parsed = parse_http_mcp_response(
        "application/json",
        r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#,
    )
    .unwrap();
    assert!(parsed["result"]["tools"].as_array().unwrap().is_empty());
}

#[test]
fn test_parse_sse_json_rpc_data_frame() {
    let parsed = parse_http_mcp_response(
        "text/event-stream",
        "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}\n\n",
    )
    .unwrap();
    assert!(parsed["result"]["tools"].as_array().unwrap().is_empty());
}

async fn spawn_http_mcp_test_server(content_type: &'static str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        for _ in 0..3 {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buf = vec![0_u8; 8192];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);
                let body = if req.contains("\"method\":\"tools/list\"") {
                    r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"web_search","description":"Search","inputSchema":{"type":"object"}}]}}"#
                } else if req.contains("\"method\":\"initialize\"") {
                    r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{}}}"#
                } else {
                    r#"{"jsonrpc":"2.0","result":{}}"#
                };
                let response_body = if content_type == "text/event-stream" {
                    format!("event: message\ndata: {body}\n\n")
                } else {
                    body.to_string()
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    format!("http://{addr}/mcp")
}

#[tokio::test]
async fn test_streamable_http_client_lists_tools() {
    let url = spawn_http_mcp_test_server("application/json").await;
    let mut client = client_from_transport(
        McpTransportConfig::StreamableHttp {
            url,
            headers: BTreeMap::new(),
        },
        McpTimeouts::default(),
    )
    .unwrap();
    client.connect().await.unwrap();
    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools[0].name, "web_search");
}

#[tokio::test]
async fn test_sse_client_lists_tools_from_event_stream_response() {
    let url = spawn_http_mcp_test_server("text/event-stream").await;
    let mut client = client_from_transport(
        McpTransportConfig::Sse {
            url,
            headers: BTreeMap::new(),
        },
        McpTimeouts::default(),
    )
    .unwrap();
    client.connect().await.unwrap();
    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools[0].name, "web_search");
}
