use std::sync::atomic::{AtomicBool, Ordering};

use super::knowledge_search::SearchQuery;
use super::knowledge_search_preflight::{SearchAdmissionEvidence, SearchDispatchPreflight};
use super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackPreflightStatus, DomainPackResourceReservation,
};

fn evidence() -> SearchAdmissionEvidence {
    SearchAdmissionEvidence {
        corpus_owned: true,
        acl_trimmed: true,
        source_attribution_available: true,
        query_complexity_allowed: true,
        page_and_cursor_valid: true,
        timeout_available: true,
        facet_cardinality_allowed: true,
        snippet_redacted: true,
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

fn query() -> SearchQuery {
    SearchQuery {
        query_ref: "query-ref".into(),
        ast_hash: "ast-hash".into(),
        page_size: 10,
        ..Default::default()
    }
}

#[test]
fn search_preflight_blocks_denied_validation_and_quota_before_dispatch() {
    let gate = SearchDispatchPreflight::new(["search.register_corpus"]);
    let invoked = AtomicBool::new(false);
    let mut denied = evidence();
    denied.acl_trimmed = false;
    let error = gate
        .dispatch_after_preflight(
            Some(&query()),
            &preflight("search.search", "knowledge.search.query"),
            &denied,
            || invoked.store(true, Ordering::SeqCst),
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Denied);
    assert!(!invoked.load(Ordering::SeqCst));
    let mut invalid = query();
    invalid.query_ref = "raw_query=token".into();
    assert!(gate
        .evaluate(
            Some(&invalid),
            &preflight("search.search", "knowledge.search.query"),
            &evidence()
        )
        .is_err());
    let mut quota = evidence();
    quota.rate_limit_available = false;
    let error = gate
        .evaluate(
            Some(&query()),
            &preflight("search.search", "knowledge.search.query"),
            &quota,
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::QuotaExceeded);
}

#[test]
fn search_preflight_blocks_unsupported_and_unavailable_before_dispatch() {
    let gate = SearchDispatchPreflight::new(std::iter::empty::<String>());
    let mut unsupported = evidence();
    unsupported.provider_capability_available = false;
    let error = gate
        .evaluate(
            Some(&query()),
            &preflight("search.search", "knowledge.search.query"),
            &unsupported,
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Unsupported);
    let mut request = preflight("search.search", "knowledge.search.query");
    request.entitlement.provider_available = false;
    let invoked = AtomicBool::new(false);
    let error = gate
        .dispatch_after_preflight(Some(&query()), &request, &evidence(), || {
            invoked.store(true, Ordering::SeqCst)
        })
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Unavailable);
    assert!(!invoked.load(Ordering::SeqCst));
}

#[test]
fn search_preflight_requires_refresh_quota_and_matching_scope() {
    let gate = SearchDispatchPreflight::new(std::iter::empty::<String>());
    let mut exhausted = evidence();
    exhausted.refresh_quota_available = false;
    let error = gate
        .evaluate(
            None,
            &preflight("search.refresh_index", "knowledge.search.index.refresh"),
            &exhausted,
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::QuotaExceeded);
    let error = gate
        .evaluate(
            Some(&query()),
            &preflight("search.search", "knowledge.search.stats"),
            &evidence(),
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Denied);
}
