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
    CapabilityToolInvocationScope, CapabilityToolOriginKind, MacacaResult, TraceContext,
};
use macaca_sdk::{SystemDriverClient, SystemSkillClient};
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
        CapabilityToolOriginKind::Mcp => None,
    }
}
