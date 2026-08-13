use std::collections::{BTreeMap, BTreeSet};

use super::identity_organization_semantics::*;

fn allowed(command: &str) -> OrganizationMutationEvidenceV1 {
    OrganizationMutationEvidenceV1 {
        command: command.into(),
        scope_ref: "organization:one".into(),
        idempotency_key_hash: "hash:idem".into(),
        expected_version_hash: "hash:v1".into(),
        current_version_hash: "hash:v1".into(),
        directory_managed: false,
        final_privileged_subject: false,
        elevated_role: false,
        approval_ref: Some("approval:granted".into()),
        policy_allowed: true,
        entitlement_available: true,
        provider_supported: true,
        host_capability_enabled: true,
        reserved_units: BTreeMap::from([("provider_calls".into(), 1)]),
        required_units: BTreeMap::from([("provider_calls".into(), 1)]),
        replay_ref: "replay:one".into(),
    }
}

#[test]
fn organization_state_machines_preserve_provider_neutral_lifecycles() {
    assert!(OrganizationLifecycleSpec::organization_allows(
        OrganizationLifecycle::Active,
        OrganizationLifecycle::Archived
    ));
    assert!(!OrganizationLifecycleSpec::organization_allows(
        OrganizationLifecycle::Archived,
        OrganizationLifecycle::Active
    ));
    assert!(OrganizationLifecycleSpec::membership_allows(
        OrganizationMembershipLifecycle::Active,
        OrganizationMembershipLifecycle::Removed
    ));
    assert!(!OrganizationLifecycleSpec::membership_allows(
        OrganizationMembershipLifecycle::DirectoryManaged,
        OrganizationMembershipLifecycle::Removed
    ));
    assert!(OrganizationLifecycleSpec::invitation_allows(
        OrganizationInvitationLifecycle::Pending,
        OrganizationInvitationLifecycle::Revoked
    ));
    assert!(OrganizationLifecycleSpec::role_binding_allows(
        OrganizationRoleBindingLifecycle::Active,
        OrganizationRoleBindingLifecycle::Removed
    ));
}

#[test]
fn preflight_rejects_policy_availability_resource_and_version_before_dispatch() {
    for mut evidence in [
        allowed("organization.create"),
        allowed("organization.update"),
        allowed("organization.search"),
    ] {
        let mut dispatched = false;
        evidence.policy_allowed = false;
        assert_eq!(
            OrganizationMutationSpec::dispatch_after_validation(&evidence, || dispatched = true)
                .unwrap_err()
                .decision,
            OrganizationMutationDecision::Denied
        );
        assert!(!dispatched);
        evidence = allowed("organization.update");
        evidence.expected_version_hash = "hash:old".into();
        assert_eq!(
            OrganizationMutationSpec::evaluate(&evidence).decision,
            OrganizationMutationDecision::StaleVersion
        );
        evidence = allowed("organization.create");
        evidence.reserved_units.clear();
        assert_eq!(
            OrganizationMutationSpec::evaluate(&evidence).decision,
            OrganizationMutationDecision::QuotaExceeded
        );
    }
}

#[test]
fn approval_directory_and_final_privileged_protection_are_fail_closed() {
    let mut evidence = allowed("organization.create_invitation");
    evidence.approval_ref = None;
    assert_eq!(
        OrganizationMutationSpec::evaluate(&evidence).decision,
        OrganizationMutationDecision::ApprovalRequired
    );
    evidence = allowed("organization.request_membership_change");
    evidence.directory_managed = true;
    assert_eq!(
        OrganizationMutationSpec::evaluate(&evidence).decision,
        OrganizationMutationDecision::Conflict
    );
    evidence = allowed("organization.request_role_binding");
    evidence.final_privileged_subject = true;
    assert_eq!(
        OrganizationMutationSpec::evaluate(&evidence).decision,
        OrganizationMutationDecision::Conflict
    );
    evidence = allowed("organization.request_role_binding");
    evidence.elevated_role = true;
    evidence.approval_ref = None;
    assert_eq!(
        OrganizationMutationSpec::evaluate(&evidence).decision,
        OrganizationMutationDecision::ApprovalRequired
    );
}

#[test]
fn filtered_pages_and_role_history_do_not_leak_hidden_state() {
    let (page, next) = filtered_organization_page(&["visible:one", "visible:two"], None, 1);
    assert_eq!(page, vec!["visible:one"]);
    assert_eq!(next.as_deref(), Some("cursor:1"));
    let history = preserved_role_history(
        &BTreeSet::from(["role:prior".into()]),
        &BTreeSet::from(["role:current".into()]),
    );
    assert_eq!(history.len(), 2);
}
