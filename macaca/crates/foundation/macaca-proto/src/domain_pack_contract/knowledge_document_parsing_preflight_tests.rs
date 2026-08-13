use std::sync::atomic::{AtomicBool, Ordering};

use super::knowledge_document_parsing::DocumentSource;
use super::knowledge_document_parsing_preflight::{
    DocumentParsingAdmissionEvidence, DocumentParsingDispatchPreflight,
};
use super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackPreflightStatus, DomainPackResourceReservation,
};

fn evidence() -> DocumentParsingAdmissionEvidence {
    DocumentParsingAdmissionEvidence {
        document_owned: true,
        document_handle_valid: true,
        media_type_allowed: true,
        malware_scan_clean: true,
        encryption_policy_allowed: true,
        size_page_output_available: true,
        ocr_policy_allowed: true,
        embedded_resource_policy_allowed: true,
        provider_capability_available: true,
        timeout_available: true,
        rate_limit_available: true,
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

fn source() -> DocumentSource {
    DocumentSource {
        document_ref: "document-ref".into(),
        media_type: "application/pdf".into(),
        size_bytes: 1024,
        malware_scan_state: "clean".into(),
    }
}

#[test]
fn document_parsing_preflight_blocks_denied_validation_and_quota_before_dispatch() {
    let gate = DocumentParsingDispatchPreflight::new(["document_parsing.start_parse_job"]);
    let invoked = AtomicBool::new(false);
    let mut denied = evidence();
    denied.malware_scan_clean = false;
    let error = gate
        .dispatch_after_preflight(
            Some(&source()),
            &preflight("document_parsing.parse_document", "document.parse"),
            &denied,
            || invoked.store(true, Ordering::SeqCst),
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Denied);
    assert!(!invoked.load(Ordering::SeqCst));
    let mut invalid = source();
    invalid.document_ref = "raw_document=private".into();
    assert!(gate
        .evaluate(
            Some(&invalid),
            &preflight("document_parsing.parse_document", "document.parse"),
            &evidence()
        )
        .is_err());
    let mut quota = evidence();
    quota.rate_limit_available = false;
    let error = gate
        .evaluate(
            Some(&source()),
            &preflight("document_parsing.parse_document", "document.parse"),
            &quota,
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::QuotaExceeded);
}

#[test]
fn document_parsing_preflight_blocks_unsupported_and_unavailable_before_dispatch() {
    let gate = DocumentParsingDispatchPreflight::new(std::iter::empty::<String>());
    let mut unsupported = evidence();
    unsupported.provider_capability_available = false;
    let error = gate
        .evaluate(
            Some(&source()),
            &preflight("document_parsing.parse_document", "document.parse"),
            &unsupported,
        )
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Unsupported);
    let mut request = preflight("document_parsing.parse_document", "document.parse");
    request.entitlement.provider_available = false;
    let invoked = AtomicBool::new(false);
    let error = gate
        .dispatch_after_preflight(Some(&source()), &request, &evidence(), || {
            invoked.store(true, Ordering::SeqCst)
        })
        .unwrap_err();
    assert_eq!(error.status, DomainPackPreflightStatus::Unavailable);
    assert!(!invoked.load(Ordering::SeqCst));
}
