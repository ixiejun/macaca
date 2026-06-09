// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;

fn command() -> McpRegistryCommand {
    McpRegistryCommand::new(RuntimeContext::new("trace-mcp-contract"))
}

fn descriptor(client_id: &str) -> McpClientDescriptor {
    let mut options = McpHttpOptions::default();
    options.headers.insert("x-scope".into(), "test".into());
    options.query_params.insert("tenant".into(), "t".into());
    let mut descriptor = McpClientDescriptor::new(
        client_id,
        McpClientTransport::StreamableHttp {
            url: "https://mcp.example.invalid".into(),
            options,
        },
        McpProtocolVersion::new("2025-06-18"),
    );
    descriptor.tool_filter.exclude.insert("disabled".into());
    descriptor.elicitation = McpElicitationConfig {
        enabled: true,
        prompt_channel: Some("generic".into()),
    };
    descriptor
}

#[test]
fn mcp_tool_filter_applies_include_and_exclude() {
    let mut filter = McpToolFilter::default();
    filter.include.insert("allowed".into());
    filter.exclude.insert("blocked".into());

    assert!(filter.allows("allowed"));
    assert!(!filter.allows("other"));
    assert!(!filter.allows("blocked"));
}

#[test]
fn descriptor_preserves_transport_headers_query_protocol_and_elicitation() {
    let descriptor = descriptor("client-a");
    let json = serde_json::to_value(&descriptor).unwrap();

    assert_eq!(json["protocol"]["version"], "2025-06-18");
    assert_eq!(json["elicitation"]["enabled"], true);
    assert_eq!(json["transport"]["options"]["headers"]["x-scope"], "test");
    assert_eq!(json["transport"]["options"]["query_params"]["tenant"], "t");
}

#[tokio::test]
async fn mock_mcp_registry_lists_and_removes_clients() {
    let registry = InMemoryMcpClientRegistry::new();

    registry
        .register(command(), descriptor("client-a"))
        .await
        .unwrap();
    assert_eq!(registry.list(command()).await.unwrap().len(), 1);

    registry.remove(command(), "client-a").await.unwrap();
    assert!(registry.list(command()).await.unwrap().is_empty());
}

#[tokio::test]
async fn unavailable_mcp_registry_fails_explicitly() {
    let registry = UnavailableMcpClientRegistry::new("mcp service not installed");
    let error = registry.list(command()).await.unwrap_err();

    assert!(matches!(error, McpRegistryError::Unavailable { .. }));
}
