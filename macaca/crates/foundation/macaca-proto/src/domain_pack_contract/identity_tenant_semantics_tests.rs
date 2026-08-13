use std::collections::BTreeMap;

use super::identity_tenant_semantics::*;

fn allowed(command: &str) -> TenantMutationEvidenceV1 {
    TenantMutationEvidenceV1 {
        command: command.into(),
        tenant_scope_ref: "tenant:one".into(),
        idempotency_key_hash: "hash:idem".into(),
        expected_version_hash: "hash:v1".into(),
        current_version_hash: "hash:v1".into(),
        policy_allowed: true,
        entitlement_available: true,
        provider_supported: true,
        host_capability_enabled: true,
        approval_ref: Some("approval:granted".into()),
        sensitive_config_reference: false,
        secret_reference_safe: true,
        residency_change: false,
        reserved_units: BTreeMap::from([("provider_calls".into(), 1)]),
        required_units: BTreeMap::from([("provider_calls".into(), 1)]),
        replay_ref: "replay:one".into(),
    }
}

#[test]
fn tenant_policy_and_quota_state_machines_are_provider_neutral() {
    assert!(TenantLifecycleSpec::tenant_allows(
        TenantLifecycle::Active,
        TenantLifecycle::Suspended
    ));
    assert!(!TenantLifecycleSpec::tenant_allows(
        TenantLifecycle::Deleted,
        TenantLifecycle::Active
    ));
    assert!(TenantLifecycleSpec::policy_attachment_allows(
        TenantPolicyAttachmentState::Attached,
        TenantPolicyAttachmentState::Detached
    ));
    assert!(TenantLifecycleSpec::quota_allows(
        TenantQuotaReservationState::Reserved,
        TenantQuotaReservationState::Released
    ));
}

#[test]
fn tenant_preflight_rejects_before_provider_dispatch() {
    let mut evidence = allowed("tenant.update");
    evidence.policy_allowed = false;
    let mut dispatched = false;
    assert_eq!(
        TenantMutationSpec::dispatch_after_validation(&evidence, || dispatched = true)
            .unwrap_err()
            .decision,
        TenantMutationDecision::Denied
    );
    assert!(!dispatched);
    evidence = allowed("tenant.update");
    evidence.expected_version_hash = "hash:old".into();
    assert_eq!(
        TenantMutationSpec::evaluate(&evidence).decision,
        TenantMutationDecision::StaleVersion
    );
    evidence = allowed("tenant.create");
    evidence.reserved_units.clear();
    assert_eq!(
        TenantMutationSpec::evaluate(&evidence).decision,
        TenantMutationDecision::QuotaExceeded
    );
    evidence = allowed("tenant.get");
    evidence.entitlement_available = false;
    assert_eq!(
        TenantMutationSpec::evaluate(&evidence).decision,
        TenantMutationDecision::Unavailable
    );
}

#[test]
fn tenant_approval_and_secret_reference_safety_are_fail_closed() {
    let mut evidence = allowed("tenant.create");
    evidence.approval_ref = None;
    assert_eq!(
        TenantMutationSpec::evaluate(&evidence).decision,
        TenantMutationDecision::ApprovalRequired
    );
    evidence = allowed("tenant.update_config_reference");
    evidence.sensitive_config_reference = true;
    evidence.secret_reference_safe = false;
    assert_eq!(
        TenantMutationSpec::evaluate(&evidence).decision,
        TenantMutationDecision::SecretReferenceDenied
    );
    evidence = allowed("tenant.inspect_quota");
    evidence.provider_supported = false;
    assert_eq!(
        TenantMutationSpec::evaluate(&evidence).decision,
        TenantMutationDecision::Unsupported
    );
}

#[test]
fn tenant_listing_is_authorized_before_pagination() {
    let (page, next) = filtered_tenant_page(&["tenant:visible-one", "tenant:visible-two"], None, 1);
    assert_eq!(page, vec!["tenant:visible-one"]);
    assert_eq!(next.as_deref(), Some("cursor:1"));
}
