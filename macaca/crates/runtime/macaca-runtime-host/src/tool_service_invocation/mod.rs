//! Invocation router for the `service.tool` Tool Capability Plane (Facade module tree).
//!
//! `service.tool` coordinates policy, trace, resource, entitlement, timeout,
//! result-budget, and audit decorators, then dispatches to the owning service.
//! It does not own Driver, Skill, or MCP runtime lifecycles. Routing is based
//! on descriptor metadata selected during planning, not on application names,
//! provider product names, or visible-name parsing.
//!
//! Module layout:
//! - `admission.rs` — Chain of Responsibility admission decorators
//! - `dispatch.rs` — Strategy router for typed executor routes
//! - `helpers.rs` — reply normalization, payload adapters, invocation refs

mod admission;
mod dispatch;
mod helpers;

use std::sync::Arc;

use macaca_proto::{
    IndustrialToolDescriptor, MacacaError, MacacaResult, ServiceBusSource, ToolCommandResult,
    ToolInvocationRef, ToolInvokeCommand, ToolResultClass, TraceContext,
};

use crate::tool_service_provider_state::stable_json_hash;
use crate::tool_service_provider_state::ToolServiceProviderState;
use crate::tool_service_result::{command_result, normalize_invocation_result};
use crate::{
    industrial_tool_managed_gateway_service, industrial_tool_runtime_environment_service,
    ServiceRuntime, ToolManagedGatewayService, ToolRuntimeEnvironmentService,
};

use admission::{ToolAdmissionDecision, ToolInvocationAdmissionChain};
use helpers::{inline_budget_bytes, next_invocation_ref};

/// Service-owned invocation coordinator.
pub struct ToolInvocationService {
    pub(super) runtime: Arc<ServiceRuntime>,
    pub(super) state: Arc<ToolServiceProviderState>,
    pub(super) source: ServiceBusSource,
    pub(super) environment_service: ToolRuntimeEnvironmentService,
    pub(super) gateway_service: ToolManagedGatewayService,
}

impl ToolInvocationService {
    /// Wire the invocation router to the shared service runtime and tool provider state.
    pub fn new(runtime: Arc<ServiceRuntime>, state: Arc<ToolServiceProviderState>) -> Self {
        tracing::info!("tool invocation service constructed for service.tool dispatch");
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

    /// Return the last recorded result for an invocation reference.
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

    /// Record a cancellation outcome for an invocation reference.
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

    /// Alias for status lookup used by `tool.result_get` commands.
    pub fn result_get(
        &self,
        trace: TraceContext,
        invocation_ref: ToolInvocationRef,
    ) -> ToolCommandResult {
        self.status(trace, invocation_ref)
    }

    /// Open a stored artifact by reference for bounded replay.
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

    /// Resolve descriptor routing metadata from the command envelope or provider state.
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
}
