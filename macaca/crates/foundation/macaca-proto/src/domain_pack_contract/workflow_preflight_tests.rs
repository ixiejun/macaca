use std::collections::BTreeMap;

use super::super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackCommandPreflightSpec,
    DomainPackEntitlementEvidence, DomainPackPolicyEvidence, DomainPackResourceReservation,
};
use super::super::workflow_approval::workflow_approval_pack_definition;
use super::super::workflow_delegation::workflow_delegation_pack_definition;
use super::super::workflow_recovery::workflow_recovery_pack_definition;
use super::super::workflow_review::workflow_review_pack_definition;
use super::super::workflow_schedule::workflow_schedule_pack_definition;
use super::*;

#[test]
fn workflow_preflight_checks_policy_entitlement_and_resources_before_dispatch() {
    for (definition, command, scope) in workflow_preflight_cases() {
        let spec = DomainPackCommandPreflightSpec::from_definition(&definition, [command]);
        let allowed = allowed_preflight(command, scope);
        let mut dispatched = false;
        assert_eq!(
            spec.dispatch_after_preflight(&allowed, || {
                dispatched = true;
                "provider-result"
            }),
            Ok("provider-result")
        );
        assert!(
            dispatched,
            "accepted command must reach the provider closure"
        );

        for rejected in [
            with_policy_denied(&allowed),
            with_provider_unavailable(&allowed),
            with_entitlement_denied(&allowed),
            with_host_capability_disabled(&allowed),
            with_insufficient_reservation(&allowed),
            with_approval_denied(&allowed),
        ] {
            let mut rejected_dispatched = false;
            assert!(spec
                .dispatch_after_preflight(&rejected, || rejected_dispatched = true)
                .is_err());
            assert!(
                !rejected_dispatched,
                "rejected preflight must not call a concrete provider"
            );
        }
    }
}

#[test]
fn workflow_preflight_reports_missing_scope_and_unsupported_command_without_dispatch() {
    for (definition, command, scope) in workflow_preflight_cases() {
        let spec = DomainPackCommandPreflightSpec::from_definition(&definition, [command]);
        for rejected in [
            DomainPackCommandPreflight {
                requested_scope: "workflow.scope.not_declared".into(),
                ..allowed_preflight(command, scope)
            },
            DomainPackCommandPreflight {
                command_name: "workflow.command.not_supported".into(),
                ..allowed_preflight(command, scope)
            },
        ] {
            let mut dispatched = false;
            assert!(spec
                .dispatch_after_preflight(&rejected, || dispatched = true)
                .is_err());
            assert!(!dispatched);
        }
    }
}

fn workflow_preflight_cases() -> Vec<(DomainPackDefinition, &'static str, &'static str)> {
    vec![
        (
            workflow_approval_pack_definition(),
            "approval.request_approval",
            "workflow.approval.request",
        ),
        (
            workflow_delegation_pack_definition(),
            "delegation.delegate",
            "workflow.delegation.create",
        ),
        (
            workflow_recovery_pack_definition(),
            "recovery.classify_failure",
            "workflow.recovery.read",
        ),
        (
            workflow_review_pack_definition(),
            "review.request_review",
            "workflow.review.request",
        ),
        (
            workflow_schedule_pack_definition(),
            "workflow_schedule.create",
            "workflow.schedule.write",
        ),
    ]
}

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

fn with_policy_denied(value: &DomainPackCommandPreflight) -> DomainPackCommandPreflight {
    let mut result = value.clone();
    result.policy.allowed = false;
    result
}

fn with_provider_unavailable(value: &DomainPackCommandPreflight) -> DomainPackCommandPreflight {
    let mut result = value.clone();
    result.entitlement.provider_available = false;
    result
}

fn with_entitlement_denied(value: &DomainPackCommandPreflight) -> DomainPackCommandPreflight {
    let mut result = value.clone();
    result.entitlement.scope_granted = false;
    result
}

fn with_host_capability_disabled(value: &DomainPackCommandPreflight) -> DomainPackCommandPreflight {
    let mut result = value.clone();
    result.entitlement.host_capability_enabled = false;
    result
}

fn with_insufficient_reservation(value: &DomainPackCommandPreflight) -> DomainPackCommandPreflight {
    let mut result = value.clone();
    result.reserved_resources.units.clear();
    result
}

fn with_approval_denied(value: &DomainPackCommandPreflight) -> DomainPackCommandPreflight {
    let mut result = value.clone();
    result.approval = None;
    result
}
