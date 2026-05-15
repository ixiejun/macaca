//! Web-local adapters from serviceized tool descriptors to framework tools.
//!
//! The framework toolkit needs concrete `Tool` objects, while Route C S6 wants
//! Driver/Skill/MCP invocation to pass through service clients.  These adapters
//! apply the Adapter pattern at the Web shell boundary: descriptors remain
//! service-owned metadata, and invocation is forwarded to focused SDK clients
//! with explicit trace and application/session/agent scope.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_driver::DriverToolInvokeCommand;
use macaca_proto::{
    ApplicationId, CapabilityToolDescriptor, CapabilityToolInvocation,
    CapabilityToolInvocationScope, CapabilityToolOriginKind, MacacaError, MacacaResult,
    McpServiceLifecycleScope, McpToolInvokeCommand, TraceContext, MCP_DESCRIPTOR_BACKEND_TOOL_NAME,
    MCP_DESCRIPTOR_LIFECYCLE_SCOPE,
};
use macaca_sdk::{SystemDriverClient, SystemMcpClient, SystemSkillClient};
use macaca_skill::SkillToolInvokeCommand;
use macaca_tools::Tool;
use serde_json::Value;

/// Service-backed tool adapter for Driver Service descriptors.
pub struct ServiceDriverToolAdapter {
    descriptor: CapabilityToolDescriptor,
    client: Arc<dyn SystemDriverClient>,
    scope: CapabilityToolInvocationScope,
}

impl ServiceDriverToolAdapter {
    /// Create a driver adapter from sanitized descriptor metadata and scope.
    pub fn new(
        descriptor: CapabilityToolDescriptor,
        client: Arc<dyn SystemDriverClient>,
        app_id: ApplicationId,
        session_id: Option<String>,
        agent_name: impl Into<String>,
    ) -> Self {
        Self {
            descriptor,
            client,
            scope: CapabilityToolInvocationScope {
                application_id: app_id,
                session_id: session_id.unwrap_or_else(|| "no-session".into()),
                agent_name: agent_name.into(),
            },
        }
    }
}

#[async_trait]
impl Tool for ServiceDriverToolAdapter {
    fn name(&self) -> &str {
        &self.descriptor.tool_name
    }

    fn description(&self) -> &str {
        &self.descriptor.description
    }

    fn parameters_schema(&self) -> Value {
        self.descriptor.parameters_schema.clone()
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let trace = TraceContext::new(format!("driver-tool-{}", self.descriptor.tool_name));
        let invocation = CapabilityToolInvocation::new(
            trace,
            self.scope.clone(),
            self.descriptor.tool_name.clone(),
            input,
        )?;
        let result = self
            .client
            .invoke_tool(DriverToolInvokeCommand { invocation })
            .await?;
        Ok(result.output.unwrap_or(Value::Null))
    }
}

/// Service-backed tool adapter for Skill Service descriptors.
pub struct ServiceSkillToolAdapter {
    descriptor: CapabilityToolDescriptor,
    client: Arc<dyn SystemSkillClient>,
    scope: CapabilityToolInvocationScope,
}

impl ServiceSkillToolAdapter {
    /// Create a skill adapter from sanitized descriptor metadata and scope.
    pub fn new(
        descriptor: CapabilityToolDescriptor,
        client: Arc<dyn SystemSkillClient>,
        app_id: ApplicationId,
        session_id: Option<String>,
        agent_name: impl Into<String>,
    ) -> Self {
        Self {
            descriptor,
            client,
            scope: CapabilityToolInvocationScope {
                application_id: app_id,
                session_id: session_id.unwrap_or_else(|| "no-session".into()),
                agent_name: agent_name.into(),
            },
        }
    }
}

#[async_trait]
impl Tool for ServiceSkillToolAdapter {
    fn name(&self) -> &str {
        &self.descriptor.tool_name
    }

    fn description(&self) -> &str {
        &self.descriptor.description
    }

    fn parameters_schema(&self) -> Value {
        self.descriptor.parameters_schema.clone()
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let trace = TraceContext::new(format!("skill-tool-{}", self.descriptor.tool_name));
        let invocation = CapabilityToolInvocation::new(
            trace,
            self.scope.clone(),
            self.descriptor.tool_name.clone(),
            input,
        )?;
        let result = self
            .client
            .invoke_tool(SkillToolInvokeCommand { invocation })
            .await?;
        Ok(result.output.unwrap_or(Value::Null))
    }
}

/// Service-backed tool adapter for MCP Service descriptors.
///
/// The adapter deliberately stores only the sanitized descriptor, focused SDK
/// client, and explicit invocation scope.  It never constructs an MCP protocol
/// client or infers routing from the visible tool name, which keeps Web as a
/// shell adapter and leaves MCP lifecycle semantics inside `service.mcp`.
pub struct ServiceMcpToolAdapter {
    descriptor: CapabilityToolDescriptor,
    client: Arc<dyn SystemMcpClient>,
    scope: CapabilityToolInvocationScope,
}

impl ServiceMcpToolAdapter {
    /// Create an MCP adapter from descriptor metadata produced by MCP Service.
    pub fn new(
        descriptor: CapabilityToolDescriptor,
        client: Arc<dyn SystemMcpClient>,
        app_id: ApplicationId,
        session_id: Option<String>,
        agent_name: impl Into<String>,
    ) -> Self {
        Self {
            descriptor,
            client,
            scope: CapabilityToolInvocationScope {
                application_id: app_id,
                session_id: session_id.unwrap_or_else(|| "no-session".into()),
                agent_name: agent_name.into(),
            },
        }
    }

    fn backend_tool_name(&self) -> MacacaResult<String> {
        self.descriptor
            .metadata
            .get(MCP_DESCRIPTOR_BACKEND_TOOL_NAME)
            .cloned()
            .ok_or_else(|| {
                MacacaError::Config(format!(
                    "MCP descriptor '{}' is missing backend tool metadata",
                    self.descriptor.tool_name
                ))
            })
    }

    fn lifecycle(&self) -> MacacaResult<McpServiceLifecycleScope> {
        let Some(value) = self.descriptor.metadata.get(MCP_DESCRIPTOR_LIFECYCLE_SCOPE) else {
            return Ok(McpServiceLifecycleScope::AgentSession);
        };
        match value.as_str() {
            "global" => Ok(McpServiceLifecycleScope::Global),
            "app" => Ok(McpServiceLifecycleScope::App),
            "session" => Ok(McpServiceLifecycleScope::Session),
            "agent_session" => Ok(McpServiceLifecycleScope::AgentSession),
            "call" => Ok(McpServiceLifecycleScope::Call),
            other => Err(MacacaError::Config(format!(
                "unsupported MCP descriptor lifecycle '{other}'"
            ))),
        }
    }
}

#[async_trait]
impl Tool for ServiceMcpToolAdapter {
    fn name(&self) -> &str {
        &self.descriptor.tool_name
    }

    fn description(&self) -> &str {
        &self.descriptor.description
    }

    fn parameters_schema(&self) -> Value {
        self.descriptor.parameters_schema.clone()
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let trace = TraceContext::new(format!("mcp-tool-{}", self.descriptor.tool_name));
        let invocation = CapabilityToolInvocation::new(
            trace,
            self.scope.clone(),
            self.descriptor.tool_name.clone(),
            input,
        )?;
        let result = self
            .client
            .invoke_tool(McpToolInvokeCommand::routed(
                invocation,
                self.descriptor.provider_id.clone(),
                self.backend_tool_name()?,
                self.lifecycle()?,
            )?)
            .await?;
        if result.status == "ok" {
            Ok(result.output.unwrap_or(Value::Null))
        } else {
            Err(MacacaError::Config(
                result
                    .error_summary
                    .unwrap_or_else(|| "MCP tool invocation failed".into()),
            ))
        }
    }
}

/// Convert one descriptor into the correct service-backed Web adapter.
pub fn service_tool_from_descriptor(
    descriptor: CapabilityToolDescriptor,
    state: &crate::state::AppState,
    app_id: ApplicationId,
    session_id: Option<String>,
    agent_name: &str,
) -> Option<Box<dyn Tool>> {
    match descriptor.origin_kind {
        CapabilityToolOriginKind::Driver => Some(Box::new(ServiceDriverToolAdapter::new(
            descriptor,
            Arc::clone(&state.driver_client),
            app_id,
            session_id,
            agent_name.to_string(),
        ))),
        CapabilityToolOriginKind::Skill => Some(Box::new(ServiceSkillToolAdapter::new(
            descriptor,
            Arc::clone(&state.skill_client),
            app_id,
            session_id,
            agent_name.to_string(),
        ))),
        CapabilityToolOriginKind::Mcp => Some(Box::new(ServiceMcpToolAdapter::new(
            descriptor,
            Arc::clone(&state.mcp_client),
            app_id,
            session_id,
            agent_name.to_string(),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_proto::{
        McpCleanupCommand, McpProbeCommand, McpRegisterCommand, McpRegisterResult,
        McpRuntimeStatusView, McpServiceSnapshot, McpServiceSnapshotCommand, McpStatusCommand,
        McpStatusResult, McpToolAttachCommand, McpToolAttachResult, McpToolCatalogCommand,
        McpToolCatalogResult, McpToolInvokeResult,
    };

    #[derive(Default)]
    struct UnusedMcpClient;

    #[async_trait]
    impl SystemMcpClient for UnusedMcpClient {
        async fn register(&self, _: McpRegisterCommand) -> MacacaResult<McpRegisterResult> {
            unreachable!("metadata parsing tests do not invoke the client")
        }

        async fn probe(&self, _: McpProbeCommand) -> MacacaResult<McpStatusResult> {
            Ok(McpStatusResult::new(Vec::<McpRuntimeStatusView>::new()))
        }

        async fn tool_catalog(
            &self,
            _: McpToolCatalogCommand,
        ) -> MacacaResult<McpToolCatalogResult> {
            Ok(McpToolCatalogResult {
                tools: Vec::new(),
                captured_at: chrono::Utc::now(),
            })
        }

        async fn attach_tools(&self, _: McpToolAttachCommand) -> MacacaResult<McpToolAttachResult> {
            Ok(McpToolAttachResult {
                statuses: Vec::new(),
                conflicts: Vec::new(),
                applied_prefixes: Vec::new(),
                captured_at: chrono::Utc::now(),
            })
        }

        async fn invoke_tool(&self, _: McpToolInvokeCommand) -> MacacaResult<McpToolInvokeResult> {
            unreachable!("metadata parsing tests do not invoke the client")
        }

        async fn status(&self, _: McpStatusCommand) -> MacacaResult<McpStatusResult> {
            Ok(McpStatusResult::new(Vec::<McpRuntimeStatusView>::new()))
        }

        async fn snapshot(&self, _: McpServiceSnapshotCommand) -> MacacaResult<McpServiceSnapshot> {
            Ok(McpServiceSnapshot::unavailable("unused test client"))
        }

        async fn cleanup(&self, _: McpCleanupCommand) -> MacacaResult<McpStatusResult> {
            Ok(McpStatusResult::new(Vec::<McpRuntimeStatusView>::new()))
        }
    }

    fn mcp_descriptor() -> CapabilityToolDescriptor {
        let mut descriptor = CapabilityToolDescriptor::new(
            macaca_proto::MCP_SERVICE_ID,
            "server-a",
            "mcp.tool.server-a.lookup",
            "mcp_lookup",
            "Lookup values",
            serde_json::json!({"type": "object"}),
            CapabilityToolOriginKind::Mcp,
        )
        .unwrap();
        descriptor
            .metadata
            .insert(MCP_DESCRIPTOR_BACKEND_TOOL_NAME.into(), "lookup".into());
        descriptor.metadata.insert(
            MCP_DESCRIPTOR_LIFECYCLE_SCOPE.into(),
            "agent_session".into(),
        );
        descriptor
    }

    #[test]
    fn mcp_adapter_reads_descriptor_routing_without_name_parsing() {
        let adapter = ServiceMcpToolAdapter::new(
            mcp_descriptor(),
            Arc::new(UnusedMcpClient),
            ApplicationId::new(),
            Some("session-a".into()),
            "agent-a",
        );

        assert_eq!(adapter.name(), "mcp_lookup");
        assert_eq!(adapter.backend_tool_name().unwrap(), "lookup");
        assert_eq!(
            adapter.lifecycle().unwrap(),
            McpServiceLifecycleScope::AgentSession
        );
    }
}
