use std::collections::BTreeMap;

use super::super::communication_messaging::{
    MessagingAttachmentRef, MessagingContent, MessagingConversationKind, MessagingConversationRef,
    MessagingSendMessageCommand, MessagingSenderRef,
};
use super::super::communication_messaging_preflight::{
    MessagingAdmissionEvidence, MessagingDispatchPreflight,
};
use super::super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackResourceReservation,
};

#[test]
fn messaging_admission_rejects_all_delivery_gates_before_provider_dispatch() {
    let gate = MessagingDispatchPreflight::new(4, 1_024, ["messaging.send_message"]);
    let command = valid_command();
    let preflight = valid_preflight();
    let evidence = valid_evidence();
    let mut dispatched = false;
    assert_eq!(
        gate.dispatch_send(&command, &preflight, &evidence, || {
            dispatched = true;
            "message-handle"
        }),
        Ok("message-handle")
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
fn messaging_sensitive_operations_require_approval_before_dispatch() {
    let commands = [
        ("messaging.send_message", "messaging.send"),
        (
            "messaging.create_conversation",
            "messaging.conversation.manage",
        ),
        ("messaging.delete_message", "messaging.delete"),
        ("messaging.fetch_message", "messaging.attachment"),
        ("messaging.ingest_event", "messaging.event.ingest"),
    ];
    let gate =
        MessagingDispatchPreflight::new(4, 1_024, commands.iter().map(|(command, _)| *command));
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

fn valid_command() -> MessagingSendMessageCommand {
    MessagingSendMessageCommand {
        sender: MessagingSenderRef {
            sender_id: "sender:one".into(),
            verified: true,
            provider_class: "mock".into(),
            secret_ref: Some("secret:messaging".into()),
        },
        conversation: MessagingConversationRef {
            conversation_id: "conversation:one".into(),
            provider_class: "mock".into(),
            kind: MessagingConversationKind::Channel,
            tenant_scope: "tenant:one".into(),
            visibility: "shared".into(),
        },
        content: MessagingContent {
            fallback_text_ref: "artifact:fallback".into(),
            content_ref: Some("artifact:content".into()),
            format: "reference".into(),
            formatting_policy: "redacted".into(),
        },
        attachments: vec![MessagingAttachmentRef {
            attachment_id: "attachment:one".into(),
            content_ref: "artifact:attachment".into(),
            content_type: "text/plain".into(),
            size_bytes: 128,
        }],
        approval_ref: Some("approval:message".into()),
        idempotency_key: "idempotency:message".into(),
    }
}

fn valid_preflight() -> DomainPackCommandPreflight {
    DomainPackCommandPreflight {
        command_name: "messaging.send_message".into(),
        requested_scope: "messaging.send".into(),
        policy: DomainPackPolicyEvidence {
            decision_ref: "policy:allowed".into(),
            allowed: true,
            reason_code: "allowed".into(),
        },
        approval: Some(DomainPackApprovalEvidence {
            approval_ref: "approval:message".into(),
            approved: true,
            reason_code: "approved".into(),
        }),
        entitlement: DomainPackEntitlementEvidence {
            entitlement_ref: "entitlement:messaging".into(),
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

fn valid_evidence() -> MessagingAdmissionEvidence {
    MessagingAdmissionEvidence {
        sender_verified: true,
        participant_channel_allowed: true,
        recipient_consent_granted: true,
        external_recipient_approved: true,
        message_within_limit: true,
        format_supported: true,
        attachments_within_limit: true,
        rate_limit_available: true,
        event_signature_valid: true,
        idempotency_available: true,
        provider_capability_available: true,
    }
}

fn rejection_cases(value: &MessagingAdmissionEvidence) -> Vec<MessagingAdmissionEvidence> {
    vec![
        MessagingAdmissionEvidence {
            sender_verified: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            participant_channel_allowed: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            recipient_consent_granted: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            external_recipient_approved: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            message_within_limit: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            format_supported: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            attachments_within_limit: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            rate_limit_available: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            event_signature_valid: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            idempotency_available: false,
            ..value.clone()
        },
        MessagingAdmissionEvidence {
            provider_capability_available: false,
            ..value.clone()
        },
    ]
}
