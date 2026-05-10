use super::config::{
    MemoryProviderConfig, MemoryProviderEndpointConfig, MemoryProviderMcpServerConfig,
    MemoryProviderResilienceConfig, MemoryProviderTransportKind,
};
use super::mcp::{mcp_provider_diagnostics, McpMemoryProvider};
use super::remote::{MemoryProviderRemoteEnvelope, MemoryProviderRemoteResult};
use super::resilience::redact_text;
use crate::{MemoryFacade, MemoryScope, MemorySearchRequest, MemoryWriteRequest};
use async_trait::async_trait;
use macaca_proto::{ApplicationId, MacacaResult, MemoryId, MemoryLayer};
use std::sync::{Arc, Mutex};

#[test]
fn remote_protocol_serializes_scope_and_timeout() {
    let envelope = MemoryProviderRemoteEnvelope {
        scope: serde_json::json!({"application_id":"app","visibility":"AgentPrivate"}),
        trace_id: Some("trace-1".into()),
        timeout_ms: 1234,
        payload: serde_json::json!({"query":"hello"}),
    };
    let encoded = serde_json::to_value(&envelope).unwrap();
    assert_eq!(encoded["timeout_ms"], 1234);
    assert_eq!(encoded["trace_id"], "trace-1");
    assert!(encoded.get("scope").is_some());
}

#[test]
fn remote_response_schema_deserializes_payload() {
    let payload = serde_json::json!({
        "healthy": true,
        "provider_id": "remote-1",
        "message": "ok",
        "payload": [{"id":"1","content":"hello"}]
    });
    let parsed: MemoryProviderRemoteResult = serde_json::from_value(payload).unwrap();
    assert!(parsed.healthy);
    assert_eq!(parsed.provider_id.as_deref(), Some("remote-1"));
    assert!(parsed.payload.is_some());
}

#[test]
fn redact_text_removes_secret_markers() {
    let redacted = redact_text(
        "token=secret-value endpoint=https://example.com",
        &["secret-value".into(), "https://example.com".into()],
    );
    assert!(!redacted.contains("secret-value"));
    assert!(!redacted.contains("https://example.com"));
}

#[test]
fn mcp_diagnostics_redact_command_details() {
    let config = MemoryProviderConfig {
        id: "mcp-provider".into(),
        display_name: Some("MCP Provider".into()),
        transport: MemoryProviderTransportKind::Mcp,
        endpoint: Some(MemoryProviderEndpointConfig {
            url: "playwright-mcp".into(),
            api_key_env: None,
            timeout_ms: 1_000,
        }),
        resilience: MemoryProviderResilienceConfig::default(),
        tools: Vec::new(),
        components: Default::default(),
    };
    let server = MemoryProviderMcpServerConfig {
        command: "playwright-mcp".into(),
        args: vec!["--token".into(), "secret-value".into()],
        env: std::collections::HashMap::from([("API_KEY".into(), "secret-value".into())]),
        timeout_ms: 1_000,
        trust_external: false,
    };
    let provider = McpMemoryProvider::new(&config, server);
    let diagnostics = mcp_provider_diagnostics(&provider);
    assert!(!diagnostics.contains("secret-value"));
}

fn mcp_config_with_tools() -> MemoryProviderConfig {
    MemoryProviderConfig {
        id: "mcp-provider".into(),
        display_name: Some("MCP Provider".into()),
        transport: MemoryProviderTransportKind::Mcp,
        endpoint: Some(MemoryProviderEndpointConfig {
            url: "memory-mcp".into(),
            api_key_env: None,
            timeout_ms: 1_000,
        }),
        resilience: MemoryProviderResilienceConfig::default(),
        tools: vec![
            super::config::MemoryProviderToolConfig {
                name: "memory_write".into(),
                namespaced: false,
            },
            super::config::MemoryProviderToolConfig {
                name: "memory_search".into(),
                namespaced: false,
            },
            super::config::MemoryProviderToolConfig {
                name: "memory_get".into(),
                namespaced: false,
            },
            super::config::MemoryProviderToolConfig {
                name: "memory_delete".into(),
                namespaced: false,
            },
        ],
        components: Default::default(),
    }
}

fn mcp_server_config() -> MemoryProviderMcpServerConfig {
    MemoryProviderMcpServerConfig {
        command: "memory-mcp".into(),
        args: Vec::new(),
        env: std::collections::HashMap::new(),
        timeout_ms: 1_000,
        trust_external: false,
    }
}

#[test]
fn mcp_provider_without_live_client_reports_unavailable_capabilities() {
    let provider = McpMemoryProvider::new(&mcp_config_with_tools(), mcp_server_config());
    let status = provider.status();

    assert!(!status.healthy);
    assert!(!status.capabilities.store);
    assert!(!status.capabilities.search);
    assert_eq!(status.message.as_deref(), Some("mcp_transport_unwired"));
}

#[derive(Default)]
struct FakeMcpClient {
    calls: Mutex<Vec<String>>,
}

#[async_trait]
impl super::mcp::MemoryMcpClient for FakeMcpClient {
    async fn call_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> MacacaResult<serde_json::Value> {
        self.calls.lock().unwrap().push(tool_name.to_string());
        match tool_name {
            "memory_write" => Ok(serde_json::json!({"id": MemoryId::new().0.to_string()})),
            "memory_search" => Ok(serde_json::json!({
                "entries": [{
                    "id": MemoryId::new(),
                    "layer": MemoryLayer::Session,
                    "content": input["query"].as_str().unwrap_or_default(),
                    "metadata": {},
                    "agent_id": null,
                    "created_at": chrono::Utc::now(),
                    "expires_at": null
                }]
            })),
            "memory_get" => Ok(serde_json::json!({"found": false})),
            "memory_delete" => Ok(serde_json::json!({"deleted": true})),
            other => Err(macaca_proto::MacacaError::Memory(format!(
                "unexpected tool {other}"
            ))),
        }
    }
}

#[tokio::test]
async fn mcp_provider_with_live_client_maps_store_and_search_tools() {
    let client = Arc::new(FakeMcpClient::default());
    let provider = McpMemoryProvider::new(&mcp_config_with_tools(), mcp_server_config())
        .with_client(client.clone());
    let scope = MemoryScope::session_shared(ApplicationId::new(), "session-1");

    let status = provider.status();
    assert!(status.healthy);
    assert!(status.capabilities.store);
    assert!(status.capabilities.search);

    let _ = provider
        .remember(MemoryWriteRequest::new(scope.clone(), "hello"))
        .await
        .unwrap();
    let rows = provider
        .search(MemorySearchRequest::new(scope, "hello", 10))
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].content, "hello");
    assert_eq!(
        client.calls.lock().unwrap().as_slice(),
        ["memory_write", "memory_search"]
    );
}
