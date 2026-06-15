//! Owner-service dispatch strategies for typed tool executor routes.
//!
//! The router matches `ToolExecutorRouteKind` from planning descriptors and delegates
//! to Driver, Skill, MCP, owning-service commands, runtime environment, or managed
//! gateway adapters without branching on application or provider product names.

use macaca_driver::{DriverToolInvokeCommand, DRIVER_TOOL_INVOKE_COMMAND};
use macaca_proto::{
    CapabilityToolInvocation, CapabilityToolInvocationResult, IndustrialToolDescriptor,
    KernelServiceId, MacacaResult, McpToolInvokeCommand, ServiceCommand, ServiceCommandName,
    ToolExecutorRouteKind, ToolInvokeCommand, MCP_TOOL_INVOKE_COMMAND,
};
use macaca_skill::{SkillToolInvokeCommand, SKILL_TOOL_INVOKE_COMMAND};
use serde_json::Value;

use crate::tool_service_provider_state::stable_json_hash;
use crate::ServiceRuntime;
use crate::{
    ToolManagedGatewayInvocation, ToolManagedGatewayService, ToolRuntimeEnvironmentInvocation,
    ToolRuntimeEnvironmentService,
};

use super::helpers::{
    backend_tool_name, lifecycle, owning_service_payload, runtime_error, wrap_service_command_reply,
};
use super::ToolInvocationService;

impl ToolInvocationService {
    /// Route one admitted invocation to the owning service selected by descriptor metadata.
    pub(super) async fn dispatch_to_owner(
        &self,
        command: &ToolInvokeCommand,
        descriptor: &IndustrialToolDescriptor,
    ) -> MacacaResult<CapabilityToolInvocationResult> {
        let invocation = CapabilityToolInvocation::new(
            command.trace.clone(),
            command.scope.clone(),
            descriptor.visible_name.clone(),
            command.input.clone(),
        )?;
        match descriptor.executor_route.route_kind {
            ToolExecutorRouteKind::Driver => {
                self.dispatch_service_command(
                    command,
                    &descriptor.executor_route.service_id,
                    DRIVER_TOOL_INVOKE_COMMAND,
                    serde_json::to_value(DriverToolInvokeCommand { invocation })?,
                )
                .await
            }
            ToolExecutorRouteKind::Skill => {
                self.dispatch_service_command(
                    command,
                    &descriptor.executor_route.service_id,
                    SKILL_TOOL_INVOKE_COMMAND,
                    serde_json::to_value(SkillToolInvokeCommand { invocation })?,
                )
                .await
            }
            ToolExecutorRouteKind::Mcp => {
                self.dispatch_service_command(
                    command,
                    descriptor.executor_route.service_id.clone(),
                    MCP_TOOL_INVOKE_COMMAND,
                    serde_json::to_value(McpToolInvokeCommand::routed(
                        invocation,
                        descriptor.base_descriptor.provider_id.clone(),
                        backend_tool_name(descriptor)?,
                        lifecycle(descriptor)?,
                    )?)?,
                )
                .await
            }
            ToolExecutorRouteKind::OwningServiceCommand => {
                let command_name = descriptor
                    .executor_route
                    .command_name
                    .as_deref()
                    .unwrap_or("tool.invoke");
                let payload = owning_service_payload(command_name, command)?;
                self.dispatch_service_command(
                    command,
                    &descriptor.executor_route.service_id,
                    command_name,
                    payload,
                )
                .await
            }
            ToolExecutorRouteKind::RuntimeEnvironment => {
                self.dispatch_runtime_environment(command, descriptor, invocation)
                    .await
            }
            ToolExecutorRouteKind::ManagedGateway => {
                self.dispatch_managed_gateway(command, descriptor, invocation)
                    .await
            }
            ToolExecutorRouteKind::Plugin | ToolExecutorRouteKind::Unavailable => {
                Ok(CapabilityToolInvocationResult::failed(
                    descriptor.executor_route.service_id.clone(),
                    descriptor.base_descriptor.origin_kind.clone(),
                    descriptor.visible_name.clone(),
                    "tool route is unavailable until a provider plugin registers this capability",
                    command.trace.clone(),
                ))
            }
        }
    }

    /// Delegate to the in-process runtime-environment adapter service.
    async fn dispatch_runtime_environment(
        &self,
        command: &ToolInvokeCommand,
        descriptor: &IndustrialToolDescriptor,
        invocation: CapabilityToolInvocation,
    ) -> MacacaResult<CapabilityToolInvocationResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            tool_id = %command.tool_id,
            provider_id = %descriptor.executor_route.provider_id,
            family = %descriptor.family.as_str(),
            "tool service dispatching to runtime environment provider service"
        );
        self.environment_service
            .invoke(ToolRuntimeEnvironmentInvocation {
                invocation,
                environment_id: format!("environment.tool.family.{}", descriptor.family.as_str()),
                provider_id: descriptor.executor_route.provider_id.clone(),
                family: descriptor.family.as_str().into(),
                tool_id: command.tool_id.clone(),
                input_hash: stable_json_hash(&command.input),
            })
            .await
    }

    /// Delegate to the in-process managed-gateway adapter service.
    async fn dispatch_managed_gateway(
        &self,
        command: &ToolInvokeCommand,
        descriptor: &IndustrialToolDescriptor,
        invocation: CapabilityToolInvocation,
    ) -> MacacaResult<CapabilityToolInvocationResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            tool_id = %command.tool_id,
            provider_id = %descriptor.executor_route.provider_id,
            family = %descriptor.family.as_str(),
            "tool service dispatching to managed gateway provider service"
        );
        self.gateway_service
            .invoke(ToolManagedGatewayInvocation {
                invocation,
                gateway_id: format!("gateway.tool.family.{}", descriptor.family.as_str()),
                provider_id: descriptor.executor_route.provider_id.clone(),
                family: descriptor.family.clone(),
                tool_id: command.tool_id.clone(),
                input_hash: stable_json_hash(&command.input),
            })
            .await
    }

    /// Forward a typed service command through the kernel service runtime bus.
    async fn dispatch_service_command(
        &self,
        command: &ToolInvokeCommand,
        service_id: impl AsRef<str>,
        command_name: &str,
        payload: Value,
    ) -> MacacaResult<CapabilityToolInvocationResult> {
        let service_command = ServiceCommand::with_trace(
            ServiceCommandName::new(command_name),
            payload,
            command.trace.clone(),
        );
        let service_id = service_id.as_ref().to_string();
        let reply = self
            .runtime
            .call(
                &KernelServiceId::new(service_id.clone()),
                self.source.clone(),
                service_command,
            )
            .await
            .map_err(runtime_error)?;
        let output = reply.output.unwrap_or(Value::Null);
        match serde_json::from_value::<CapabilityToolInvocationResult>(output.clone()) {
            Ok(result) => Ok(result),
            Err(_) => Ok(wrap_service_command_reply(
                service_id,
                command_name,
                output,
                command.trace.clone(),
            )),
        }
    }
}
