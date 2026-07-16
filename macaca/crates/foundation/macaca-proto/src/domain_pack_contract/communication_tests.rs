use std::collections::BTreeSet;

use super::*;

// Communication pack tests intentionally validate descriptor/DTO contracts only.
// They do not open network transports, register webhook endpoints, send messages,
// inspect local devices, or construct provider adapters.

#[test]
fn communication_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            communication_email_pack_definition(),
            COMMUNICATION_EMAIL_PACK_ID,
            COMMUNICATION_EMAIL_SERVICE_ID,
            COMMUNICATION_EMAIL_COMMANDS,
            "email_provider_not_installed",
            "transactional-mail",
            "email.send",
        ),
        (
            communication_messaging_pack_definition(),
            COMMUNICATION_MESSAGING_PACK_ID,
            COMMUNICATION_MESSAGING_SERVICE_ID,
            COMMUNICATION_MESSAGING_COMMANDS,
            "messaging_provider_not_installed",
            "conversation-bridge",
            "messaging.send",
        ),
        (
            communication_notification_pack_definition(),
            COMMUNICATION_NOTIFICATION_PACK_ID,
            COMMUNICATION_NOTIFICATION_SERVICE_ID,
            COMMUNICATION_NOTIFICATION_COMMANDS,
            "notification_provider_not_installed",
            "push-bridge",
            "notification.publish",
        ),
        (
            communication_inbox_pack_definition(),
            COMMUNICATION_INBOX_PACK_ID,
            COMMUNICATION_INBOX_SERVICE_ID,
            COMMUNICATION_INBOX_COMMANDS,
            "inbox_provider_not_installed",
            "source-sync",
            "inbox.sync",
        ),
        (
            communication_calendar_pack_definition(),
            COMMUNICATION_CALENDAR_PACK_ID,
            COMMUNICATION_CALENDAR_SERVICE_ID,
            COMMUNICATION_CALENDAR_COMMANDS,
            "calendar_provider_not_installed",
            "calendar-sync",
            "calendar.write",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, scope) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.communication.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition.metadata.sdk.docs_url.contains("communication"));
        assert!(definition.metadata.permission_scopes.contains(scope));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("communication descriptor exposes command schemas");
        for command in commands {
            assert!(
                descriptor_commands.contains(*command),
                "missing command {command}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_communication_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let email = definitions
        .iter()
        .find(|definition| definition.pack_id == COMMUNICATION_EMAIL_PACK_ID)
        .expect("industrial catalog includes communication email");
    let calendar = definitions
        .iter()
        .find(|definition| definition.pack_id == COMMUNICATION_CALENDAR_PACK_ID)
        .expect("industrial catalog includes communication calendar");

    assert_eq!(
        email.metadata.diagnostics.unavailable_reason,
        "email_provider_not_installed"
    );
    assert!(email
        .metadata
        .service_command_schemas
        .get(COMMUNICATION_EMAIL_SERVICE_ID)
        .is_some_and(|commands| commands.contains("email.send")));
    assert_eq!(
        calendar
            .metadata
            .provider_descriptors
            .get("calendar-sync")
            .and_then(|descriptor| descriptor.metadata.get("sync_watch"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn communication_command_dtos_are_serde_compatible() {
    let sender = EmailSenderRef {
        sender_id: "sender".into(),
        address_hash: "addr-hash".into(),
        verified: true,
        provider_class: "mock".into(),
        secret_ref: Some("secret:email".into()),
    };
    let recipient = EmailRecipient {
        kind: EmailRecipientKind::To,
        address_hash: "recipient-hash".into(),
        display_name: None,
        consent: EmailConsentStatus::Granted,
        domain_policy: "allowed".into(),
    };
    let email = serde_json::to_value(EmailComposeCommand {
        sender,
        recipients: vec![recipient],
        subject_ref: "artifact:subject".into(),
        body_parts: vec![EmailBodyPart {
            kind: EmailBodyKind::Markdown,
            content_ref: "artifact:body".into(),
            language: Some("en".into()),
            redaction_policy: "body_redacted".into(),
        }],
        attachments: Vec::new(),
    })
    .unwrap();

    let conversation = MessagingConversationRef {
        conversation_id: "conversation".into(),
        provider_class: "mock".into(),
        kind: MessagingConversationKind::Channel,
        tenant_scope: "tenant".into(),
        visibility: "private".into(),
    };
    let messaging = serde_json::to_value(MessagingSendMessageCommand {
        sender: MessagingSenderRef {
            sender_id: "bot".into(),
            verified: true,
            provider_class: "mock".into(),
            secret_ref: Some("secret:messaging".into()),
        },
        conversation,
        content: MessagingContent {
            fallback_text_ref: "artifact:fallback".into(),
            content_ref: None,
            format: "markdown".into(),
            formatting_policy: "fallback_allowed".into(),
        },
        attachments: Vec::new(),
        approval_ref: Some("approval:external".into()),
        idempotency_key: "idem-msg".into(),
    })
    .unwrap();

    let notification = serde_json::to_value(NotificationPublishCommand {
        message: NotificationMessage {
            title_ref: "artifact:title".into(),
            body_ref: "artifact:body".into(),
            locale: Some("en-US".into()),
            sensitivity: "normal".into(),
            category_id: Some("status".into()),
            collapse_key: None,
        },
        target: NotificationTarget {
            target_id: "session".into(),
            target_kind: "in_app".into(),
            subscription: None,
            redaction_label: "target_hash_only".into(),
        },
        channel: NotificationDeliveryChannel::InApp,
        client_request_id: "idem-notification".into(),
    })
    .unwrap();

    let inbox = serde_json::to_value(InboxSyncSourcesCommand {
        source_ids: vec!["source".into()],
        page_size: 100,
    })
    .unwrap();

    let calendar = serde_json::to_value(CalendarCreateEventCommand {
        event: CalendarEvent {
            event_id: "event".into(),
            source_id: "calendar".into(),
            title_ref: "artifact:title".into(),
            description_ref: Some("artifact:description".into()),
            start_epoch_ms: 1_800_000_000_000,
            end_epoch_ms: 1_800_003_600_000,
            timezone_id: "UTC".into(),
            recurrence: None,
            attendees: Vec::new(),
        },
        idempotency_key: "idem-calendar".into(),
    })
    .unwrap();

    assert!([email, messaging, notification, inbox, calendar]
        .iter()
        .all(|value| value.is_object()));
}

#[test]
fn communication_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        communication_email_descriptor_hashes().into_hashes(),
        communication_messaging_descriptor_hashes().into_hashes(),
        communication_notification_descriptor_hashes().into_hashes(),
        communication_inbox_descriptor_hashes().into_hashes(),
        communication_calendar_descriptor_hashes().into_hashes(),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), 5);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }

    let request = InboxListItemsCommand {
        source_id: "source".into(),
        cursor: None,
        page_size: 100,
    };
    let mut changed = request.clone();
    changed.page_size = 200;
    assert_ne!(inbox_stable_hash(&request), inbox_stable_hash(&changed));
}

trait DescriptorHashSet {
    fn into_hashes(self) -> [String; 5];
}

impl DescriptorHashSet for EmailDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.snapshot_schema_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for MessagingDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.snapshot_schema_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for NotificationDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.snapshot_schema_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for InboxDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.snapshot_schema_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}

impl DescriptorHashSet for CalendarDescriptorHashes {
    fn into_hashes(self) -> [String; 5] {
        [
            self.command_schema_hash,
            self.result_schema_hash,
            self.snapshot_schema_hash,
            self.provider_capability_schema_hash,
            self.unavailable_schema_hash,
        ]
    }
}
