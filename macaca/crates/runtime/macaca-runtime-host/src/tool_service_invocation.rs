//! Invocation router for the `service.tool` Tool Capability Plane.
//!
//! `service.tool` coordinates policy, trace, resource, entitlement, timeout,
//! result-budget, and audit decorators, then dispatches to the owning service.
//! It does not own Driver, Skill, or MCP runtime lifecycles.  Routing is based
//! on descriptor metadata selected during planning, not on application names,
//! provider product names, or visible-name parsing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use macaca_driver::{DriverToolInvokeCommand, DRIVER_TOOL_INVOKE_COMMAND};
use macaca_proto::{
    CapabilityToolInvocation, CapabilityToolInvocationResult, CapabilityToolOriginKind,
    IndustrialToolDescriptor, KernelServiceId, MacacaError, MacacaResult, McpServiceLifecycleScope,
    McpToolInvokeCommand, ServiceBusSource, ServiceCommand, ServiceCommandName, ToolCommandResult,
    ToolExecutorRouteKind, ToolInvocationRef, ToolInvokeCommand, ToolResultClass, TraceContext,
    MCP_DESCRIPTOR_BACKEND_TOOL_NAME, MCP_DESCRIPTOR_LIFECYCLE_SCOPE, MCP_TOOL_INVOKE_COMMAND,
};
use macaca_skill::{SkillToolInvokeCommand, SKILL_TOOL_INVOKE_COMMAND};
use serde_json::Value;

use crate::tool_service_provider_state::stable_json_hash;
use crate::tool_service_provider_state::ToolServiceProviderState;
use crate::tool_service_result::{bounded_summary, command_result, normalize_invocation_result};
use crate::{
    industrial_tool_managed_gateway_service, industrial_tool_runtime_environment_service,
    ToolManagedGatewayInvocation, ToolManagedGatewayService, ToolRuntimeEnvironmentInvocation,
    ToolRuntimeEnvironmentService,
};
use crate::{ServiceRuntime, ServiceRuntimeError};

static INVOCATION_SEQ: AtomicU64 = AtomicU64::new(1);

/// Service-owned invocation coordinator.
pub struct ToolInvocationService {
    runtime: Arc<ServiceRuntime>,
    state: Arc<ToolServiceProviderState>,
    source: ServiceBusSource,
    environment_service: ToolRuntimeEnvironmentService,
    gateway_service: ToolManagedGatewayService,
}

impl ToolInvocationService {
    pub fn new(runtime: Arc<ServiceRuntime>, state: Arc<ToolServiceProviderState>) -> Self {
        Self {
            runtime,
            state,
            source: ServiceBusSource::new("service.tool"),
            environment_service: industrial_tool_runtime_environment_service(),
            gateway_service: industrial_tool_managed_gateway_service(),
        }
    }

    /// Invoke one tool through the owning service after local decorators pass.
    pub async fn invoke(&self, command: ToolInvokeCommand) -> MacacaResult<ToolCommandResult> {
        let descriptor = self.resolve_descriptor(&command)?;
        let invocation_ref = next_invocation_ref(&command.trace);
        let input_hash = stable_json_hash(&command.input);
        tracing::info!(
            trace_id = %command.trace.trace_id,
            invocation_ref = %invocation_ref.0,
            tool_id = %command.tool_id,
            owner_service = %descriptor.executor_route.service_id,
            family = %descriptor.family.as_str(),
            "tool service invocation accepted"
        );
        self.state.record_invocation_event(
            &command.trace,
            &command.tool_id,
            "tool.invocation.started",
            "accepted",
        );
        self.state.record_invocation_event(
            &command.trace,
            &command.tool_id,
            "tool.resource_lease.acquired",
            "acquired",
        );

        let admission =
            ToolInvocationAdmissionChain::evaluate(&command, &descriptor, &invocation_ref);
        if let ToolAdmissionDecision::Denied(result)
        | ToolAdmissionDecision::ApprovalRequired(result)
        | ToolAdmissionDecision::Unavailable(result) = admission
        {
            tracing::warn!(
                trace_id = %command.trace.trace_id,
                invocation_ref = %invocation_ref.0,
                tool_id = %command.tool_id,
                route_kind = ?descriptor.executor_route.route_kind,
                status = %result.status,
                "tool service admission stopped invocation before provider dispatch"
            );
            self.state.record_invocation_result(result.clone());
            self.state.record_invocation_audit(
                &result,
                &command.tool_id,
                &descriptor.executor_route.provider_id,
                &descriptor.executor_route.service_id,
                input_hash,
            );
            return Ok(result);
        }
        if command
            .metadata
            .get("timeout.force")
            .is_some_and(|value| value == "true")
        {
            let result = command_result(
                command.trace,
                "timeout",
                ToolResultClass::Failure,
                None,
                Vec::new(),
                Some(invocation_ref),
                Some("tool invocation timed out before dispatch".into()),
            );
            self.state.record_invocation_result(result.clone());
            self.state.record_invocation_audit(
                &result,
                &command.tool_id,
                &descriptor.executor_route.provider_id,
                &descriptor.executor_route.service_id,
                input_hash,
            );
            return Ok(result);
        }

        tracing::info!(
            trace_id = %command.trace.trace_id,
            invocation_ref = %invocation_ref.0,
            owner_service = %descriptor.executor_route.service_id,
            "tool service decorators admitted invocation; dispatching to owner"
        );
        let owner_result = self.dispatch_to_owner(&command, &descriptor).await?;
        let normalized = normalize_invocation_result(
            invocation_ref,
            owner_result,
            inline_budget_bytes(&command.metadata),
        );
        if let Some((artifact_ref, payload)) = normalized.artifact_payload {
            tracing::info!(
                trace_id = %normalized.command_result.trace.trace_id,
                artifact_ref = %artifact_ref.0,
                "tool service stored oversized result as artifact ref"
            );
            self.state.record_artifact(&artifact_ref, payload);
        }
        self.state
            .record_invocation_result(normalized.command_result.clone());
        self.state.record_invocation_audit(
            &normalized.command_result,
            &command.tool_id,
            &descriptor.executor_route.provider_id,
            &descriptor.executor_route.service_id,
            input_hash,
        );
        tracing::info!(
            trace_id = %normalized.command_result.trace.trace_id,
            status = %normalized.command_result.status,
            result_class = ?normalized.command_result.result_class,
            "tool service invocation completed"
        );
        Ok(normalized.command_result)
    }

    pub fn status(
        &self,
        trace: TraceContext,
        invocation_ref: ToolInvocationRef,
    ) -> ToolCommandResult {
        self.state
            .invocation_result(&invocation_ref)
            .unwrap_or_else(|| {
                command_result(
                    trace,
                    "unknown",
                    ToolResultClass::Failure,
                    None,
                    Vec::new(),
                    Some(invocation_ref),
                    Some("tool invocation ref is unknown".into()),
                )
            })
    }

    pub fn cancel(
        &self,
        trace: TraceContext,
        invocation_ref: ToolInvocationRef,
    ) -> ToolCommandResult {
        let result = command_result(
            trace,
            "cancelled",
            ToolResultClass::Failure,
            None,
            Vec::new(),
            Some(invocation_ref),
            Some("tool invocation cancellation recorded".into()),
        );
        self.state.record_invocation_result(result.clone());
        result
    }

    pub fn result_get(
        &self,
        trace: TraceContext,
        invocation_ref: ToolInvocationRef,
    ) -> ToolCommandResult {
        self.status(trace, invocation_ref)
    }

    pub fn artifact_open(
        &self,
        trace: TraceContext,
        artifact_ref: macaca_proto::ToolArtifactRef,
    ) -> ToolCommandResult {
        match self.state.artifact(&artifact_ref) {
            Some(payload) => command_result(
                trace,
                "ok",
                ToolResultClass::StructuredJson,
                Some(payload),
                vec![artifact_ref],
                None,
                None,
            ),
            None => command_result(
                trace,
                "unknown",
                ToolResultClass::Failure,
                None,
                vec![artifact_ref],
                None,
                Some("tool artifact ref is unknown".into()),
            ),
        }
    }

    fn resolve_descriptor(
        &self,
        command: &ToolInvokeCommand,
    ) -> MacacaResult<IndustrialToolDescriptor> {
        command
            .descriptor
            .clone()
            .or_else(|| self.state.descriptor_by_tool_id(&command.tool_id))
            .ok_or_else(|| {
                MacacaError::Config(format!(
                    "tool '{}' is not present in descriptor routing metadata",
                    command.tool_id
                ))
            })
    }

    async fn dispatch_to_owner(
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

    async fn dispatch_service_command(
        &self,
        command: &ToolInvokeCommand,
        service_id: impl AsRef<str>,
        command_name: &str,
        payload: serde_json::Value,
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

fn wrap_service_command_reply(
    service_id: String,
    command_name: &str,
    output: serde_json::Value,
    trace: TraceContext,
) -> CapabilityToolInvocationResult {
    // Workbench-family services return their own typed WorkbenchCommandResult
    // envelopes. service.tool normalizes those replies into the historical
    // CapabilityToolInvocationResult shape so result budgeting, artifact
    // storage, and audit recording stay uniform across old and new providers.
    let mut result = if let Some(reason) = workbench_unavailable_reason(&output) {
        CapabilityToolInvocationResult::failed(
            service_id,
            CapabilityToolOriginKind::Mcp,
            command_name,
            reason,
            trace,
        )
    } else {
        CapabilityToolInvocationResult::ok(
            service_id,
            CapabilityToolOriginKind::Mcp,
            command_name,
            output,
            trace,
        )
    };
    result.metadata.insert(
        "service_tool.normalized_reply".into(),
        "workbench_command".into(),
    );
    result
}

fn workbench_unavailable_reason(output: &serde_json::Value) -> Option<String> {
    output
        .get("status")
        .and_then(|status| status.get("Unavailable"))
        .and_then(|unavailable| unavailable.get("reason"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// Typed admission outcome returned by the admission decorator chain.
///
/// Keeping this as an enum prevents policy, entitlement, approval, and
/// unavailable-provider outcomes from being inferred from free-form strings in
/// the dispatch code. Metadata remains an input compatibility surface, but the
/// router consumes typed decisions before any side effect.
enum ToolAdmissionDecision {
    Admit,
    Denied(ToolCommandResult),
    ApprovalRequired(ToolCommandResult),
    Unavailable(ToolCommandResult),
}

/// Decorator-style admission chain for `tool.invoke`.
///
/// The chain is intentionally small and local in this slice, but each check is
/// its own step so future policy, resource, entitlement, and budget services
/// can replace the data source without changing route dispatch.
struct ToolInvocationAdmissionChain;

impl ToolInvocationAdmissionChain {
    fn evaluate(
        command: &ToolInvokeCommand,
        descriptor: &IndustrialToolDescriptor,
        invocation_ref: &ToolInvocationRef,
    ) -> ToolAdmissionDecision {
        if descriptor.executor_route.route_kind == ToolExecutorRouteKind::Unavailable {
            return ToolAdmissionDecision::Unavailable(command_result(
                command.trace.clone(),
                "unavailable",
                ToolResultClass::Failure,
                None,
                Vec::new(),
                Some(invocation_ref.clone()),
                Some("tool route has no registered provider".into()),
            ));
        }
        if let Some(result) = policy_denial(command, descriptor) {
            return ToolAdmissionDecision::Denied(result);
        }
        if let Some(result) = entitlement_missing(command, descriptor) {
            return ToolAdmissionDecision::Denied(result);
        }
        if let Some(result) = approval_required(command, descriptor, invocation_ref.clone()) {
            return ToolAdmissionDecision::ApprovalRequired(result);
        }
        ToolAdmissionDecision::Admit
    }
}

fn policy_denial(
    command: &ToolInvokeCommand,
    descriptor: &IndustrialToolDescriptor,
) -> Option<ToolCommandResult> {
    let denies_by_decision = command
        .metadata
        .get("policy.decision")
        .is_some_and(|value| value == "deny");
    let denies_by_family = command
        .metadata
        .get("policy.denied_families")
        .is_some_and(|value| {
            value
                .split(',')
                .any(|item| item.trim() == descriptor.family.as_str())
        });
    (denies_by_decision || denies_by_family).then(|| {
        command_result(
            command.trace.clone(),
            "denied",
            ToolResultClass::Failure,
            None,
            Vec::new(),
            None,
            Some("tool invocation denied by policy before side effects".into()),
        )
    })
}

fn entitlement_missing(
    command: &ToolInvokeCommand,
    descriptor: &IndustrialToolDescriptor,
) -> Option<ToolCommandResult> {
    let requires_entitlement = command.metadata.contains_key("entitlement.required")
        || descriptor.availability.iter().any(|expr| {
            matches!(
                expr,
                macaca_proto::AvailabilityExpression::Entitlement { .. }
            )
        });
    let has_entitlement = command
        .metadata
        .get("entitlement.state")
        .is_some_and(|value| value == "granted");
    (requires_entitlement && !has_entitlement).then(|| {
        command_result(
            command.trace.clone(),
            "entitlement_missing",
            ToolResultClass::Failure,
            None,
            Vec::new(),
            None,
            Some("tool invocation missing required entitlement before side effects".into()),
        )
    })
}

fn approval_required(
    command: &ToolInvokeCommand,
    descriptor: &IndustrialToolDescriptor,
    invocation_ref: ToolInvocationRef,
) -> Option<ToolCommandResult> {
    let requires_approval = command
        .metadata
        .get("approval.required")
        .is_some_and(|value| value == "true")
        || descriptor.approval_profile.is_some();
    (requires_approval && command.approval_ref.is_none()).then(|| {
        command_result(
            command.trace.clone(),
            "approval_required",
            ToolResultClass::ApprovalRequest,
            Some(serde_json::json!({
                "invocation_ref": invocation_ref.0,
                "tool_id": command.tool_id,
                "reason": "approval_required",
            })),
            Vec::new(),
            Some(invocation_ref),
            None,
        )
    })
}

fn inline_budget_bytes(metadata: &std::collections::BTreeMap<String, String>) -> usize {
    metadata
        .get("result.inline_budget_bytes")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(16 * 1024)
}

fn owning_service_payload(
    command_name: &str,
    command: &ToolInvokeCommand,
) -> MacacaResult<serde_json::Value> {
    // Some workbench services expose a family-specific `*.tool.invoke`
    // compatibility command that expects the full ToolInvokeCommand envelope.
    // Other service-owned tools route directly to existing typed service
    // commands, where service.tool must forward only the caller-provided input
    // DTO.  This small Adapter keeps the route data authoritative and avoids
    // service.tool learning application or provider-specific command shapes.
    if command_name.ends_with(".tool.invoke") || command_name == "tool.invoke" {
        serde_json::to_value(command).map_err(MacacaError::from)
    } else {
        Ok(command.input.clone())
    }
}

fn backend_tool_name(descriptor: &IndustrialToolDescriptor) -> MacacaResult<String> {
    descriptor
        .base_descriptor
        .metadata
        .get(MCP_DESCRIPTOR_BACKEND_TOOL_NAME)
        .cloned()
        .unwrap_or_else(|| descriptor.visible_name.clone())
        .trim()
        .to_string()
        .pipe_non_empty("mcp tool invocation requires backend tool name")
}

fn lifecycle(descriptor: &IndustrialToolDescriptor) -> MacacaResult<McpServiceLifecycleScope> {
    match descriptor
        .base_descriptor
        .metadata
        .get(MCP_DESCRIPTOR_LIFECYCLE_SCOPE)
        .map(String::as_str)
        .unwrap_or("agent_session")
    {
        "global" => Ok(McpServiceLifecycleScope::Global),
        "app" => Ok(McpServiceLifecycleScope::App),
        "session" => Ok(McpServiceLifecycleScope::Session),
        "agent_session" => Ok(McpServiceLifecycleScope::AgentSession),
        "call" => Ok(McpServiceLifecycleScope::Call),
        other => Err(MacacaError::Config(format!(
            "unsupported MCP descriptor lifecycle '{}'",
            bounded_summary(other.into())
        ))),
    }
}

fn next_invocation_ref(trace: &TraceContext) -> ToolInvocationRef {
    ToolInvocationRef(format!(
        "tool-invocation-{}-{}",
        trace.trace_id,
        INVOCATION_SEQ.fetch_add(1, Ordering::SeqCst)
    ))
}

fn runtime_error(error: ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}

trait NonEmptyString {
    fn pipe_non_empty(self, message: &'static str) -> MacacaResult<String>;
}

impl NonEmptyString for String {
    fn pipe_non_empty(self, message: &'static str) -> MacacaResult<String> {
        if self.is_empty() {
            Err(MacacaError::Config(message.into()))
        } else {
            Ok(self)
        }
    }
}
