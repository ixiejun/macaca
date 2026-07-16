use std::collections::BTreeMap;

use super::super::communication_email::{EmailDraftRef, EmailSendCommand};
use super::super::communication_email_preflight::{EmailAdmissionEvidence, EmailDispatchPreflight};
use super::super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackResourceReservation,
};

#[test]
fn email_admission_rejects_delivery_before_provider_dispatch() {
    let gate = EmailDispatchPreflight::new(["email.send"]);
    let command = valid_command();
    let preflight = valid_preflight();
    let evidence = valid_evidence();
    let mut dispatched = false;
    assert_eq!(
        gate.dispatch_send(&command, &preflight, &evidence, || {
            dispatched = true;
            "delivery-handle"
        }),
        Ok("delivery-handle")
    );
    assert!(dispatched);

    for rejected in rejection_cases(&evidence) {
        let mut rejected_dispatched = false;
        assert!(gate
            .dispatch_send(&command, &preflight, &rejected, || rejected_dispatched =
                true)
            .is_err());
        assert!(!rejected_dispatched);
    }
}

#[test]
fn email_send_requires_a_canonical_approved_and_idempotent_request() {
    let gate = EmailDispatchPreflight::new(["email.send"]);
    let invalid = EmailSendCommand {
        approval_ref: None,
        ..valid_command()
    };
    let mut dispatched = false;
    let result = gate.dispatch_send(&invalid, &valid_preflight(), &valid_evidence(), || {
        dispatched = true
    });
    assert_eq!(
        result
            .expect_err("missing approval must reject send")
            .reason_code,
        "email_send_invalid"
    );
    assert!(!dispatched);
}

#[test]
fn email_sensitive_operations_require_approval_before_dispatch() {
    let commands = [
        ("email.send", "email.send"),
        ("email.schedule_send", "email.send"),
        ("email.sync_mailbox", "email.mailbox.sync"),
        ("email.fetch_attachment", "email.attachment"),
        ("email.apply_labels", "email.mailbox.mutate"),
    ];
    let gate = EmailDispatchPreflight::new(commands.iter().map(|(command, _)| *command));

    for (command_name, scope) in commands {
        let mut preflight = valid_preflight();
        preflight.command_name = command_name.into();
        preflight.requested_scope = scope.into();
        preflight.approval = None;
        let mut dispatched = false;
        assert!(gate
            .dispatch_send(&valid_command(), &preflight, &valid_evidence(), || {
                dispatched = true
            })
            .is_err());
        assert!(!dispatched, "{command_name} must require approval");
    }
}

fn valid_command() -> EmailSendCommand {
    EmailSendCommand {
        message: None,
        draft: Some(EmailDraftRef {
            draft_id: "draft:one".into(),
            revision: "v1".into(),
        }),
        approval_ref: Some("approval:send".into()),
        idempotency_key: "idempotency:send".into(),
    }
}

fn valid_preflight() -> DomainPackCommandPreflight {
    DomainPackCommandPreflight {
        command_name: "email.send".into(),
        requested_scope: "email.send".into(),
        policy: DomainPackPolicyEvidence {
            decision_ref: "policy:allowed".into(),
            allowed: true,
            reason_code: "allowed".into(),
        },
        approval: Some(DomainPackApprovalEvidence {
            approval_ref: "approval:send".into(),
            approved: true,
            reason_code: "approved".into(),
        }),
        entitlement: DomainPackEntitlementEvidence {
            entitlement_ref: "entitlement:email".into(),
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

fn valid_evidence() -> EmailAdmissionEvidence {
    EmailAdmissionEvidence {
        sender_verified: true,
        recipient_valid: true,
        recipient_consent_granted: true,
        external_recipient_approved: true,
        message_within_limit: true,
        attachments_within_limit: true,
        rate_limit_available: true,
        webhook_signature_valid: true,
        idempotency_available: true,
        provider_capability_available: true,
    }
}

fn rejection_cases(value: &EmailAdmissionEvidence) -> Vec<EmailAdmissionEvidence> {
    vec![
        EmailAdmissionEvidence {
            sender_verified: false,
            ..value.clone()
        },
        EmailAdmissionEvidence {
            recipient_valid: false,
            ..value.clone()
        },
        EmailAdmissionEvidence {
            recipient_consent_granted: false,
            ..value.clone()
        },
        EmailAdmissionEvidence {
            external_recipient_approved: false,
            ..value.clone()
        },
        EmailAdmissionEvidence {
            message_within_limit: false,
            ..value.clone()
        },
        EmailAdmissionEvidence {
            attachments_within_limit: false,
            ..value.clone()
        },
        EmailAdmissionEvidence {
            rate_limit_available: false,
            ..value.clone()
        },
        EmailAdmissionEvidence {
            webhook_signature_valid: false,
            ..value.clone()
        },
        EmailAdmissionEvidence {
            idempotency_available: false,
            ..value.clone()
        },
        EmailAdmissionEvidence {
            provider_capability_available: false,
            ..value.clone()
        },
    ]
}
