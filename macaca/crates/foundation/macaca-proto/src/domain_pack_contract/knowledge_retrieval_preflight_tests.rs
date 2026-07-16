use std::sync::atomic::{AtomicBool, Ordering};

use super::knowledge_retrieval::{RetrievalFusionStrategy, RetrievalQuery};
use super::knowledge_retrieval_preflight::{
    RetrievalAdmissionEvidence, RetrievalDispatchPreflight,
};
use super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackPreflightStatus, DomainPackResourceReservation,
};

fn evidence() -> RetrievalAdmissionEvidence {
    RetrievalAdmissionEvidence {
        collection_accessible: true,
        secret_reference_valid: true,
        namespace_isolated: true,
        vector_space_compatible: true,
        embedding_model_compatible: true,
        acl_allows: true,
        filters_valid: true,
        top_k_available: true,
        threshold_and_range_valid: true,
        context_window_available: true,
        query_complexity_allowed: true,
        timeout_available: true,
        provider_capability_available: true,
        rate_limit_available: true,
        refresh_quota_available: true,
        resource_budget_available: true,
        payload_redacted: true,
        output_bounded: true,
    }
}

fn preflight(command: &str, scope: &str) -> DomainPackCommandPreflight {
    DomainPackCommandPreflight {
        command_name: command.into(),
        requested_scope: scope.into(),
        policy: DomainPackPolicyEvidence {
            decision_ref: "policy-ref".into(),
            allowed: true,
            reason_code: "allowed".into(),
        },
        approval: Some(DomainPackApprovalEvidence {
            approval_ref: "approval-ref".into(),
            approved: true,
            reason_code: "approved".into(),
        }),
        entitlement: DomainPackEntitlementEvidence {
            entitlement_ref: "entitlement-ref".into(),
            provider_available: true,
            scope_granted: true,
            command_supported: true,
            host_capability_enabled: true,
            reason_code: "granted".into(),
        },
        required_resources: DomainPackResourceReservation::default(),
        reserved_resources: DomainPackResourceReservation::default(),
    }
}

fn query() -> RetrievalQuery {
    RetrievalQuery {
        query_ref: "query-ref".into(),
        vector_space_id: "space-ref".into(),
        filters: Vec::new(),
        fusion: RetrievalFusionStrategy {
            mode: "dense".into(),
            weights: Default::default(),
        },
        top_k: 10,
    }
}

#[test]
fn retrieval_preflight_blocks_denied_invalid_and_quota_paths_before_dispatch() {
    let gate = RetrievalDispatchPreflight::new(["retrieval.upsert_records"]);
    let invoked = AtomicBool::new(false);
    let mut denied = evidence();
    denied.acl_allows = false;
    let error = gate
        .dispatch_after_preflight(
            Some(&query()),
            &preflight("retrieval.retrieve", "retrieval.query"),
            &denied,
            || invoked.store(true, Ordering::SeqCst),
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Denied);
    assert!(!invoked.load(Ordering::SeqCst));
    let mut quota = evidence();
    quota.top_k_available = false;
    let error = gate
        .evaluate(
            Some(&query()),
            &preflight("retrieval.retrieve", "retrieval.query"),
            &quota,
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::QuotaExceeded);
    let mut invalid = query();
    invalid.query_ref = "credential=raw".into();
    assert!(gate
        .evaluate(
            Some(&invalid),
            &preflight("retrieval.retrieve", "retrieval.query"),
            &evidence()
        )
        .is_err());
}

#[test]
fn retrieval_preflight_rejects_unavailable_and_unsupported_before_dispatch() {
    let gate = RetrievalDispatchPreflight::new(std::iter::empty::<String>());
    let mut unavailable = evidence();
    unavailable.provider_capability_available = false;
    let error = gate
        .evaluate(
            Some(&query()),
            &preflight("retrieval.retrieve", "retrieval.query"),
            &unavailable,
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Unsupported);
    let error = gate
        .evaluate(
            Some(&query()),
            &preflight("retrieval.retrieve", "retrieval.read"),
            &evidence(),
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Denied);
}

#[test]
fn retrieval_preflight_reports_provider_unavailable_before_dispatch() {
    let gate = RetrievalDispatchPreflight::new(std::iter::empty::<String>());
    let mut unavailable = evidence();
    unavailable.provider_capability_available = true;
    let mut request = preflight("retrieval.retrieve", "retrieval.query");
    request.entitlement.provider_available = false;
    let invoked = AtomicBool::new(false);

    let error = gate
        .dispatch_after_preflight(Some(&query()), &request, &unavailable, || {
            invoked.store(true, Ordering::SeqCst)
        })
        .unwrap_err();

    assert_eq!(error.status, DomainPackPreflightStatus::Unavailable);
    assert!(!invoked.load(Ordering::SeqCst));
}
