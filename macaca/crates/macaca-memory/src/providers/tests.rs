use super::config::{
    MemoryProviderConfig, MemoryProviderEndpointConfig, MemoryProviderMcpServerConfig,
    MemoryProviderResilienceConfig, MemoryProviderTransportKind,
};
use super::mcp::{mcp_provider_diagnostics, McpMemoryProvider};
use super::remote::{MemoryProviderRemoteEnvelope, MemoryProviderRemoteResult};
use super::resilience::redact_text;

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
