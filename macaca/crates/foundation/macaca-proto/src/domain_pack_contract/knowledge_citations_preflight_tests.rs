use std::sync::atomic::{AtomicBool, Ordering};

use super::knowledge_citations::{CitationItem, CitationSourceAnchor};
use super::knowledge_citations_preflight::{CitationAdmissionEvidence, CitationDispatchPreflight};
use super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackPreflightStatus, DomainPackResourceReservation,
};

fn evidence() -> CitationAdmissionEvidence {
    CitationAdmissionEvidence {
        source_accessible: true,
        source_anchor_valid: true,
        identifier_scheme_supported: true,
        resolver_policy_allowed: true,
        style_supported: true,
        import_export_within_limit: true,
        quote_redacted: true,
        output_bounded: true,
        rate_limit_available: true,
        timeout_available: true,
        resource_budget_available: true,
        provider_capability_available: true,
        payload_redacted: true,
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

fn item() -> CitationItem {
    CitationItem {
        citation_id: "citation-ref".into(),
        title_ref: "title-ref".into(),
        source_anchor: Some(CitationSourceAnchor {
            source_ref: "source-ref".into(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn citation_preflight_blocks_denied_validation_and_quota_before_dispatch() {
    let gate = CitationDispatchPreflight::new(["citations.create_citation"]);
    let invoked = AtomicBool::new(false);
    let mut denied = evidence();
    denied.source_accessible = false;
    let error = gate
        .dispatch_after_preflight(
            Some(&item()),
            &preflight("citations.create_citation", "citation.create"),
            &denied,
            || invoked.store(true, Ordering::SeqCst),
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Denied);
    assert!(!invoked.load(Ordering::SeqCst));
    let mut invalid = item();
    invalid.title_ref = "raw_document=private".into();
    assert!(gate
        .evaluate(
            Some(&invalid),
            &preflight("citations.create_citation", "citation.create"),
            &evidence()
        )
        .is_err());
    let mut quota = evidence();
    quota.rate_limit_available = false;
    let error = gate
        .evaluate(
            Some(&item()),
            &preflight("citations.create_citation", "citation.create"),
            &quota,
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::QuotaExceeded);
}

#[test]
fn citation_preflight_blocks_unsupported_and_unavailable_before_dispatch() {
    let gate = CitationDispatchPreflight::new(std::iter::empty::<String>());
    let mut unsupported = evidence();
    unsupported.style_supported = false;
    let error = gate
        .evaluate(
            None,
            &preflight("citations.format_bibliography", "citation.format"),
            &unsupported,
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Unsupported);
    let mut request = preflight("citations.resolve_identifier", "citation.resolve");
    request.entitlement.provider_available = false;
    let invoked = AtomicBool::new(false);
    let error = gate
        .dispatch_after_preflight(None, &request, &evidence(), || {
            invoked.store(true, Ordering::SeqCst)
        })
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Unavailable);
    assert!(!invoked.load(Ordering::SeqCst));
}
