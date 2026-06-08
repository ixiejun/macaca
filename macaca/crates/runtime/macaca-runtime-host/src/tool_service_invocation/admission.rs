//! Admission decorator chain for `service.tool` invocations.
//!
//! Policy, entitlement, approval, and unavailable-route checks run as a small
//! Chain of Responsibility before any provider dispatch or side effect. Each step
//! returns a typed decision so the router never infers admission from free-form
//! strings.

use macaca_proto::{
    IndustrialToolDescriptor, ToolCommandResult, ToolExecutorRouteKind, ToolInvocationRef,
    ToolInvokeCommand, ToolResultClass,
};

use crate::tool_service_result::command_result;

/// Typed admission outcome returned by the admission decorator chain.
///
/// Keeping this as an enum prevents policy, entitlement, approval, and
/// unavailable-provider outcomes from being inferred from free-form strings in
/// the dispatch code. Metadata remains an input compatibility surface, but the
/// router consumes typed decisions before any side effect.
pub(crate) enum ToolAdmissionDecision {
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
pub(crate) struct ToolInvocationAdmissionChain;

impl ToolInvocationAdmissionChain {
    /// Evaluate all admission decorators and return the first blocking decision.
    pub(crate) fn evaluate(
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

/// Deny when caller metadata marks an explicit policy decision or family block list.
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

/// Deny when entitlement metadata or descriptor availability requires a grant that is missing.
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

/// Return an approval-request result when approval metadata or profile requires a ref.
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
