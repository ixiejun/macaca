//! Web adapter from service-owned descriptors to framework tools.
//!
//! The Web shell owns only the projection from a descriptor into a framework
//! `Tool`.  Production invocation crosses the SDK `SystemToolClient` facade and
//! then `service.tool/tool.invoke`; Driver, Skill, and MCP services retain
//! concrete lifecycle and execution ownership behind that route.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationId, CapabilityToolDescriptor, CapabilityToolInvocationScope,
    CapabilityToolOriginKind, IndustrialToolDescriptor, MacacaError, MacacaResult, ToolFamilyRef,
    ToolInvokeCommand, TraceContext,
};
use macaca_sdk::SystemToolClient;
use macaca_tools::Tool;
use serde_json::{json, Value};

/// Framework `Tool` implementation that invokes through `service.tool`.
pub struct ToolServiceInvocationAdapter {
    descriptor: IndustrialToolDescriptor,
    client: Arc<dyn SystemToolClient>,
    scope: CapabilityToolInvocationScope,
}

impl ToolServiceInvocationAdapter {
    /// Create an adapter from a sanitized descriptor and explicit runtime scope.
    ///
    /// The descriptor is included in the command so `service.tool` can route by
    /// stable owner metadata even when the planning cache is empty or stale.
    pub fn new(
        descriptor: IndustrialToolDescriptor,
        client: Arc<dyn SystemToolClient>,
        app_id: ApplicationId,
        session_id: Option<String>,
        agent_name: impl Into<String>,
    ) -> Self {
        let agent_name = agent_name.into();
        let scope = CapabilityToolInvocationScope {
            application_id: app_id,
            session_id: session_id.unwrap_or_else(|| "no-session".into()),
            agent_name,
        };
        Self {
            descriptor,
            client,
            scope,
        }
    }
}

#[async_trait]
impl Tool for ToolServiceInvocationAdapter {
    fn name(&self) -> &str {
        &self.descriptor.visible_name
    }

    fn description(&self) -> &str {
        &self.descriptor.base_descriptor.description
    }

    fn parameters_schema(&self) -> Value {
        self.descriptor.base_descriptor.parameters_schema.clone()
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let trace = TraceContext::new(format!("tool-service-{}", self.descriptor.visible_name));
        let command = ToolInvokeCommand {
            trace,
            scope: self.scope.clone(),
            tool_id: self.descriptor.stable_tool_id.clone(),
            descriptor: Some(self.descriptor.clone()),
            input,
            policy_ref: None,
            approval_ref: None,
            metadata: Default::default(),
        };
        let result = self.client.invoke(command).await?;
        match result.status.as_str() {
            "ok" | "approval_required" => Ok(result.inline_output.unwrap_or_else(|| {
                json!({
                    "status": result.status,
                    "artifact_refs": result.artifact_refs.into_iter().map(|item| item.0).collect::<Vec<_>>(),
                })
            })),
            _ => Err(MacacaError::Config(result.error_summary.unwrap_or_else(|| {
                format!("tool service invocation failed with status '{}'", result.status)
            }))),
        }
    }
}

/// Convert one service-owned descriptor into a production framework tool.
pub fn service_tool_from_descriptor(
    descriptor: CapabilityToolDescriptor,
    state: &crate::state::AppState,
    app_id: ApplicationId,
    session_id: Option<String>,
    agent_name: &str,
) -> Option<Box<dyn Tool>> {
    let industrial = industrial_from_capability_descriptor(descriptor).ok()?;
    Some(Box::new(ToolServiceInvocationAdapter::new(
        industrial,
        Arc::clone(&state.tool_client),
        app_id,
        session_id,
        agent_name.to_string(),
    )))
}

fn industrial_from_capability_descriptor(
    descriptor: CapabilityToolDescriptor,
) -> MacacaResult<IndustrialToolDescriptor> {
    let family = descriptor
        .metadata
        .get("tool.family")
        .cloned()
        .unwrap_or_else(|| default_family(&descriptor.origin_kind).to_string());
    let title = descriptor
        .display_name
        .clone()
        .unwrap_or_else(|| descriptor.tool_name.clone());
    let stable_tool_id = descriptor
        .metadata
        .get("tool.stable_id")
        .cloned()
        .unwrap_or_else(|| {
            format!(
                "tool.{}.{}.{}",
                descriptor.service_id, descriptor.provider_id, descriptor.tool_name
            )
        });
    IndustrialToolDescriptor::new(
        stable_tool_id,
        descriptor.tool_name.clone(),
        title,
        ToolFamilyRef::new(family)?,
        descriptor,
    )
}

fn default_family(origin_kind: &CapabilityToolOriginKind) -> &'static str {
    match origin_kind {
        CapabilityToolOriginKind::Driver => "driver",
        CapabilityToolOriginKind::Skill => "skill",
        CapabilityToolOriginKind::Mcp => "mcp",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{MCP_DESCRIPTOR_BACKEND_TOOL_NAME, MCP_DESCRIPTOR_LIFECYCLE_SCOPE};

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
    fn adapter_preserves_descriptor_routing_without_name_parsing() {
        let descriptor = industrial_from_capability_descriptor(mcp_descriptor()).unwrap();

        assert_eq!(descriptor.visible_name, "mcp_lookup");
        assert_eq!(
            descriptor.executor_route.service_id,
            macaca_proto::MCP_SERVICE_ID
        );
        assert_eq!(
            descriptor
                .base_descriptor
                .metadata
                .get(MCP_DESCRIPTOR_BACKEND_TOOL_NAME)
                .unwrap(),
            "lookup"
        );
    }
}
