use std::collections::BTreeMap;

use super::super::communication_inbox::InboxFetchBodyCommand;
use super::super::communication_inbox_preflight::{InboxAdmissionEvidence, InboxDispatchPreflight};
use super::super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackPreflightStatus, DomainPackResourceReservation,
};

#[test]
fn inbox_admission_blocks_source_side_effects_for_every_rejected_evidence_state() {
    let gate = InboxDispatchPreflight::new(1_024, ["inbox.fetch_body"]);
    let command = InboxFetchBodyCommand {
        item_id: "item:one".into(),
        body_part: "part:body".into(),
        max_bytes: 512,
    };
    let preflight = valid_preflight();
    let evidence = valid_evidence();

    let mut dispatched = false;
    assert_eq!(
        gate.dispatch_body_fetch(&command, &preflight, &evidence, || {
            dispatched = true;
            "body-handle"
        }),
        Ok("body-handle")
    );
    assert!(dispatched);

    for rejected in rejected_evidence_cases(&evidence) {
        let mut rejected_dispatched = false;
        assert!(gate
            .dispatch_body_fetch(&command, &preflight, &rejected, || rejected_dispatched =
                true)
            .is_err());
        assert!(!rejected_dispatched);
    }
}

#[test]
fn inbox_preflight_returns_bounded_statuses_for_invalid_unavailable_and_unsupported_paths() {
    let gate = InboxDispatchPreflight::new(1_024, ["inbox.fetch_body"]);
    let command = InboxFetchBodyCommand {
        item_id: "item:one".into(),
        body_part: "part:body".into(),
        max_bytes: 2_048,
    };
    let invalid = gate
        .evaluate_body_fetch(&command, &valid_preflight(), &valid_evidence())
        .expect_err("oversized body fetch must not reach a source");
    assert_eq!(invalid.reason_code, "inbox_body_fetch_invalid");

    let unavailable = InboxAdmissionEvidence {
        provider_available: false,
        ..valid_evidence()
    };
    assert_eq!(
        gate.evaluate_body_fetch(
            &InboxFetchBodyCommand {
                max_bytes: 512,
                ..command
            },
            &valid_preflight(),
            &unavailable,
        )
        .expect_err("missing provider must be unavailable")
        .status,
        DomainPackPreflightStatus::Unavailable
    );
    let unsupported = InboxAdmissionEvidence {
        command_capability_available: false,
        ..valid_evidence()
    };
    assert_eq!(
        gate.evaluate_body_fetch(
            &InboxFetchBodyCommand {
                item_id: "item:two".into(),
                body_part: "part:body".into(),
                max_bytes: 512,
            },
            &valid_preflight(),
            &unsupported,
        )
        .expect_err("unsupported command must not dispatch")
        .status,
        DomainPackPreflightStatus::Unsupported
    );
}

fn valid_preflight() -> DomainPackCommandPreflight {
    DomainPackCommandPreflight {
        command_name: "inbox.fetch_body".into(),
        requested_scope: "inbox.read.body".into(),
        policy: DomainPackPolicyEvidence {
            decision_ref: "policy:allowed".into(),
            allowed: true,
            reason_code: "allowed".into(),
        },
        approval: Some(DomainPackApprovalEvidence {
            approval_ref: "approval:allowed".into(),
            approved: true,
            reason_code: "approved".into(),
        }),
        entitlement: DomainPackEntitlementEvidence {
            entitlement_ref: "entitlement:allowed".into(),
            provider_available: true,
            scope_granted: true,
            command_supported: true,
            host_capability_enabled: true,
            reason_code: "allowed".into(),
        },
        required_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
        reserved_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
    }
}

fn valid_evidence() -> InboxAdmissionEvidence {
    InboxAdmissionEvidence {
        source_owned_by_caller: true,
        credential_secret_reference_valid: true,
        webhook_secret_reference_valid: true,
        provider_available: true,
        command_capability_available: true,
        rate_limit_available: true,
        timeout_within_limit: true,
        page_size_within_limit: true,
        storage_budget_reserved: true,
        body_redaction_allowed: true,
        attachment_redaction_allowed: true,
    }
}

fn rejected_evidence_cases(value: &InboxAdmissionEvidence) -> Vec<InboxAdmissionEvidence> {
    vec![
        InboxAdmissionEvidence {
            source_owned_by_caller: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            credential_secret_reference_valid: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            webhook_secret_reference_valid: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            provider_available: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            command_capability_available: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            rate_limit_available: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            timeout_within_limit: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            page_size_within_limit: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            storage_budget_reserved: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            body_redaction_allowed: false,
            ..value.clone()
        },
        InboxAdmissionEvidence {
            attachment_redaction_allowed: false,
            ..value.clone()
        },
    ]
}
