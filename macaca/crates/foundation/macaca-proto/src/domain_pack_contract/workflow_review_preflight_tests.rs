use std::collections::BTreeMap;

use super::super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackCommandPreflightSpec,
    DomainPackEntitlementEvidence, DomainPackPolicyEvidence, DomainPackPreflightStatus,
    DomainPackResourceReservation,
};
use super::super::workflow_review::workflow_review_pack_definition;

/// External, durable, or terminal review actions require approval before dispatch.
/// The generic preflight Specification keeps this policy rule provider-neutral.
#[test]
fn sensitive_review_commands_require_approval_before_provider_dispatch() {
    let commands = [
        ("review.request_review", "workflow.review.request"),
        ("review.request_rereview", "workflow.review.write"),
        ("review.approve", "workflow.review.approve"),
        ("review.close_review", "workflow.review.approve"),
        ("review.dismiss", "workflow.review.dismiss"),
    ];
    let spec = DomainPackCommandPreflightSpec::from_definition(
        &workflow_review_pack_definition(),
        commands.iter().map(|(command, _)| *command),
    );
    for (command, scope) in commands {
        let mut preflight = allowed_preflight(command, scope);
        preflight.approval = None;
        let mut dispatched = false;
        let rejection = spec
            .dispatch_after_preflight(&preflight, || dispatched = true)
            .expect_err("sensitive review command must not dispatch without approval");
        assert_eq!(rejection.status, DomainPackPreflightStatus::Denied);
        assert_eq!(rejection.reason_code, "approval_required");
        assert!(!dispatched, "provider side effect must remain unreachable");
    }
}

/// Build complete sanitized host evidence for a permitted preflight fixture.
fn allowed_preflight(command: &str, scope: &str) -> DomainPackCommandPreflight {
    DomainPackCommandPreflight {
        command_name: command.into(),
        requested_scope: scope.into(),
        policy: DomainPackPolicyEvidence {
            decision_ref: "policy:allowed".into(),
            allowed: true,
            reason_code: "allowed".into(),
        },
        approval: Some(DomainPackApprovalEvidence {
            approval_ref: "approval:granted".into(),
            approved: true,
            reason_code: "approved".into(),
        }),
        entitlement: DomainPackEntitlementEvidence {
            entitlement_ref: "entitlement:granted".into(),
            provider_available: true,
            scope_granted: true,
            command_supported: true,
            host_capability_enabled: true,
            reason_code: "granted".into(),
        },
        required_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
        reserved_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
    }
}
