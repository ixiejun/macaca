//! Shared test fixtures for MCP runtime contract tests.
//!
//! Provides a Strategy-injectable [`TestMcpClient`] and helper builders so
//! invocation/catalog tests can simulate protocol outcomes without real MCP I/O.

use std::collections::BTreeMap;
use std::future::pending;
use std::sync::Arc;

use macaca_framework::mcp::{
    McpCallResult, McpClient, McpError, McpResourceDef, McpResourceRead, McpResourceTemplateDef,
    McpSessionMode, McpTimeouts, McpToolDef, McpTransportConfig,
};
use macaca_framework::message::{ContentBlock, TextBlock};

use crate::mcp_runtime::{McpDefinitionSource, McpLifecycleScope, McpServerDefinition};

use super::super::descriptors::descriptor_from_tool;
use super::super::manager::McpRuntimeManager;

/// Builds a minimal stdio [`McpServerDefinition`] for contract tests.
///
/// `command` doubles as the sole required binary so missing-binary paths can be
/// exercised by choosing a non-existent executable name.
pub(crate) fn stdio_definition(id: &str, command: &str) -> McpServerDefinition {
    McpServerDefinition {
        id: id.to_string(),
        transport: McpTransportConfig::Stdio {
            command: command.to_string(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
        },
        lifecycle: McpLifecycleScope::AgentSession,
        session_mode: McpSessionMode::Stateful,
        tool_prefix: None,
        required_bins: vec![command.to_string()],
        enabled: true,
        source: McpDefinitionSource::Mapping,
        concurrency_isolation: None,
    }
}

/// Seeds the descriptor index with a single routable tool for invoke-path tests.
pub(crate) async fn seed_descriptor_route(
    manager: &McpRuntimeManager,
    definition: &McpServerDefinition,
    backend_tool_name: &str,
    visible_tool_name: &str,
) {
    let tool = McpToolDef {
        name: backend_tool_name.into(),
        description: "Seeded test tool".into(),
        input_schema: serde_json::json!({"type": "object"}),
    };
    let descriptor = descriptor_from_tool(definition, tool, visible_tool_name.into()).unwrap();
    let outcomes = manager
        .descriptor_index
        .upsert_descriptors(&[descriptor])
        .await;
    assert!(outcomes[0].is_ok());
}

/// Scripted protocol client outcomes for Template Method invoke tests.
#[derive(Clone)]
pub(crate) enum TestMcpClientBehavior {
    Success,
    ToolError,
    ConnectFailure,
    CallFailure,
    CallTimeout,
}

/// In-memory [`McpClient`] that returns deterministic fixture responses.
pub(crate) struct TestMcpClient {
    behavior: TestMcpClientBehavior,
    connected: bool,
}

#[async_trait::async_trait]
impl McpClient for TestMcpClient {
    async fn connect(&mut self) -> Result<(), McpError> {
        if matches!(self.behavior, TestMcpClientBehavior::ConnectFailure) {
            return Err(McpError::Connection("fixture connect failed".into()));
        }
        self.connected = true;
        Ok(())
    }

    async fn list_tools(&mut self) -> Result<Vec<McpToolDef>, McpError> {
        Ok(vec![McpToolDef {
            name: "lookup".into(),
            description: "Lookup values".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }])
    }

    async fn call_tool(
        &mut self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<McpCallResult, McpError> {
        match self.behavior {
            TestMcpClientBehavior::Success => Ok(McpCallResult {
                content: vec![ContentBlock::Text(TextBlock {
                    text: format!("called {name} with {}", args["query"]),
                })],
                is_error: false,
                metadata: Some(serde_json::json!({"fixture": true})),
            }),
            TestMcpClientBehavior::ToolError => Ok(McpCallResult {
                content: vec![ContentBlock::Text(TextBlock {
                    text: "fixture tool rejected input".into(),
                })],
                is_error: true,
                metadata: None,
            }),
            TestMcpClientBehavior::ConnectFailure => {
                unreachable!("connect failure should short-circuit before call_tool")
            }
            TestMcpClientBehavior::CallFailure => {
                Err(McpError::Execution("fixture call failed".into()))
            }
            TestMcpClientBehavior::CallTimeout => pending().await,
        }
    }

    async fn list_resources(&mut self) -> Result<Vec<McpResourceDef>, McpError> {
        Ok(vec![McpResourceDef {
            uri: "fixture://resource-a".into(),
            name: Some("Resource A".into()),
            mime_type: Some("text/plain".into()),
        }])
    }

    async fn list_resource_templates(&mut self) -> Result<Vec<McpResourceTemplateDef>, McpError> {
        Ok(vec![McpResourceTemplateDef {
            uri_template: "fixture://{id}".into(),
            name: Some("Fixture Template".into()),
            mime_type: Some("text/plain".into()),
        }])
    }

    async fn read_resource(&mut self, uri: &str) -> Result<McpResourceRead, McpError> {
        Ok(McpResourceRead {
            uri: uri.into(),
            mime_type: Some("text/plain".into()),
            text: Some("fixture resource body".into()),
            blob: None,
        })
    }

    async fn close(&mut self) -> Result<(), McpError> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Returns a manager wired with a fixture client factory for invoke/resource tests.
pub(crate) fn manager_with_fixture_client(
    behavior: TestMcpClientBehavior,
    timeouts: McpTimeouts,
) -> Arc<McpRuntimeManager> {
    Arc::new(McpRuntimeManager::new_with_client_factory_for_tests(
        Arc::new(move |_definition, _timeouts| {
            Ok(Box::new(TestMcpClient {
                behavior: behavior.clone(),
                connected: false,
            }))
        }),
        timeouts,
    ))
}
