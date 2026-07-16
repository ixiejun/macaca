use super::*;

#[test]
fn calendar_preflight_rejects_raw_calendar_payloads_and_conference_secrets() {
    let source = CalendarSource {
        source_id: "calendar-source".into(),
        display_name: "work-calendar".into(),
        owner_hash: "owner-hash".into(),
        timezone_id: "America/Los_Angeles".into(),
        provider_class: "calendar-sync".into(),
    };
    assert!(source.has_safe_identity());

    let event = CalendarEvent {
        event_id: "event-ref".into(),
        source_id: "calendar-source".into(),
        title_ref: "title-ref".into(),
        description_ref: Some("description-ref".into()),
        start_epoch_ms: 10,
        end_epoch_ms: 20,
        timezone_id: "America/Los_Angeles".into(),
        recurrence: Some(CalendarRecurrence {
            frequency: "weekly".into(),
            interval: 1,
            count: Some(4),
            until_epoch_ms: None,
            timezone_id: "America/Los_Angeles".into(),
            expansion_limit: 16,
        }),
        attendees: vec![CalendarAttendee {
            attendee_id: "attendee-ref".into(),
            role: "required".into(),
            response_state: "needs_action".into(),
            identity_scope: "tenant-ref".into(),
        }],
    };
    let create = CalendarCreateEventCommand {
        event,
        idempotency_key: "idem-calendar".into(),
    };
    assert!(create.has_admission_preconditions(8, 32));

    let raw_import = CalendarImportIcalendarCommand {
        source: source.clone(),
        content_ref: "BEGIN:VCALENDAR\nSECRET".into(),
        dry_run: true,
    };
    assert!(!raw_import.is_reference_only());

    let safe_conference = CalendarConference {
        conference_id: "conference-ref".into(),
        join_url_ref: Some("join-handle".into()),
        secret_ref: Some("secret:calendar-passcode".into()),
    };
    assert!(safe_conference.is_handle_only());

    let raw_conference = CalendarConference {
        secret_ref: Some("https://meet.example/raw-secret".into()),
        ..safe_conference
    };
    assert!(!raw_conference.is_handle_only());

    let watch = CalendarWatch {
        watch_id: "watch-ref".into(),
        source_id: "calendar-source".into(),
        callback_ref: "callback-route-ref".into(),
        expires_epoch_ms: Some(100),
    };
    assert!(watch.is_safe_reference());

    let export = CalendarExportIcalendarCommand {
        event_ids: vec!["event-ref".into()],
        redaction_profile: "calendar-redacted".into(),
    };
    assert!(export.is_bounded_export(10));
}

#[test]
fn email_preflight_requires_secret_refs_and_reference_only_content() {
    let sender = EmailSenderRef {
        sender_id: "sender-ref".into(),
        address_hash: "address-hash".into(),
        verified: true,
        provider_class: "mailbox-send".into(),
        secret_ref: Some("secret:mailbox".into()),
    };
    assert!(sender.has_safe_credentials());

    let compose = EmailComposeCommand {
        sender,
        recipients: vec![EmailRecipient {
            kind: EmailRecipientKind::To,
            address_hash: "recipient-hash".into(),
            display_name: None,
            consent: EmailConsentStatus::Granted,
            domain_policy: "allowed".into(),
        }],
        subject_ref: "subject-ref".into(),
        body_parts: vec![EmailBodyPart {
            kind: EmailBodyKind::Reference,
            content_ref: "body-ref".into(),
            language: Some("en".into()),
            redaction_policy: "email-redacted".into(),
        }],
        attachments: vec![EmailAttachmentRef {
            attachment_id: "attachment-ref".into(),
            content_ref: "attachment-content-ref".into(),
            content_type: "application/pdf".into(),
            size_bytes: 128,
            checksum: Some("sha256-redacted".into()),
            inline_content_id: None,
        }],
    };
    assert!(compose.has_admission_preconditions(5, 1024));

    let raw_sender = EmailSenderRef {
        secret_ref: Some("ya29.raw-oauth-token".into()),
        ..compose.sender.clone()
    };
    assert!(!raw_sender.has_safe_credentials());

    let raw_body = EmailBodyPart {
        kind: EmailBodyKind::Html,
        content_ref: "line1\nraw html body".into(),
        language: None,
        redaction_policy: "email-redacted".into(),
    };
    assert!(!raw_body.is_reference_only());

    let send = EmailSendCommand {
        message: None,
        draft: Some(EmailDraftRef {
            draft_id: "draft-ref".into(),
            revision: "rev-1".into(),
        }),
        approval_ref: Some("approval-ref".into()),
        idempotency_key: "idem-email".into(),
    };
    assert!(send.has_send_preconditions());
}

#[test]
fn inbox_preflight_rejects_raw_source_secrets_and_unbounded_fetches() {
    let source = InboxSource {
        source_id: "inbox-source".into(),
        source_kind: "mailbox".into(),
        provider_class: "inbox-source".into(),
        credential_secret_ref: Some("secret:inbox".into()),
        health: "available".into(),
    };
    assert!(source.has_safe_credentials());

    let raw_source = InboxSource {
        credential_secret_ref: Some("https://provider.example/raw-token".into()),
        ..source
    };
    assert!(!raw_source.has_safe_credentials());

    let item = InboxItem {
        item_id: "item-ref".into(),
        source_id: "inbox-source".into(),
        thread_id: Some("thread-ref".into()),
        sender_hash: "sender-hash".into(),
        subject_ref: Some("subject-ref".into()),
        preview_ref: Some("preview-ref".into()),
        read: false,
        label_ids: vec!["label-ref".into()],
    };
    assert!(item.is_safe_projection(4));

    let body_fetch = InboxFetchBodyCommand {
        item_id: "item-ref".into(),
        body_part: "body-ref".into(),
        max_bytes: 512,
    };
    assert!(body_fetch.has_bounded_fetch(1024));

    let attachment = InboxAttachmentHandle {
        item_id: "item-ref".into(),
        part_id: "part-ref".into(),
        filename_hash: "filename-hash".into(),
        mime_type: "image/png".into(),
        size_bytes: 512,
        content_ref: Some("attachment-ref".into()),
    };
    assert!(attachment.is_within_limit(1024));
}

#[test]
fn messaging_preflight_requires_secret_refs_approval_and_reference_content() {
    let conversation = MessagingConversationRef {
        conversation_id: "conversation-ref".into(),
        provider_class: "message-bus".into(),
        kind: MessagingConversationKind::Channel,
        tenant_scope: "tenant-ref".into(),
        visibility: "shared".into(),
    };
    assert!(conversation.is_safe_reference());

    let sender = MessagingSenderRef {
        sender_id: "sender-ref".into(),
        verified: true,
        provider_class: "message-bus".into(),
        secret_ref: Some("secret:messaging".into()),
    };
    assert!(sender.has_safe_credentials());

    let send = MessagingSendMessageCommand {
        sender,
        conversation,
        content: MessagingContent {
            fallback_text_ref: "fallback-ref".into(),
            content_ref: Some("content-ref".into()),
            format: "reference".into(),
            formatting_policy: "messaging-redacted".into(),
        },
        attachments: vec![MessagingAttachmentRef {
            attachment_id: "attachment-ref".into(),
            content_ref: "attachment-content-ref".into(),
            content_type: "image/png".into(),
            size_bytes: 256,
        }],
        approval_ref: Some("approval-ref".into()),
        idempotency_key: "idem-message".into(),
    };
    assert!(send.has_admission_preconditions(4, 1024));

    let raw_sender = MessagingSenderRef {
        secret_ref: Some("xoxb-raw-token".into()),
        ..send.sender.clone()
    };
    assert!(!raw_sender.has_safe_credentials());

    let event = MessagingIngestEventCommand {
        event: MessagingProviderEventRef {
            event_id_hash: "event-hash".into(),
            provider_class: "message-bus".into(),
            signature_status: "verified".into(),
        },
        state: MessagingDeliveryState::Delivered,
        idempotency_key: "idem-event".into(),
    };
    assert!(event.has_ingest_preconditions());
}

#[test]
fn notification_preflight_rejects_raw_push_secrets_and_payloads() {
    let target = NotificationTarget {
        target_id: "target-ref".into(),
        target_kind: "device".into(),
        subscription: Some(NotificationSubscriptionHandle {
            subscription_id: "subscription-ref".into(),
            target_class: "device".into(),
            secret_ref: Some("secret:push-subscription".into()),
            provider_class: "push-provider".into(),
        }),
        redaction_label: "notification-redacted".into(),
    };
    assert!(target.is_safe_reference());

    let publish = NotificationPublishCommand {
        message: NotificationMessage {
            title_ref: "title-ref".into(),
            body_ref: "body-ref".into(),
            locale: Some("en".into()),
            sensitivity: "private".into(),
            category_id: Some("category-ref".into()),
            collapse_key: Some("collapse-ref".into()),
        },
        target,
        channel: NotificationDeliveryChannel::Push,
        client_request_id: "request-ref".into(),
    };
    assert!(publish.has_admission_preconditions());

    let raw_subscription = NotificationSubscriptionHandle {
        subscription_id: "subscription-ref".into(),
        target_class: "device".into(),
        secret_ref: Some("https://push.example/raw-endpoint".into()),
        provider_class: "push-provider".into(),
    };
    assert!(!raw_subscription.is_safe_reference());

    let raw_message = NotificationMessage {
        title_ref: "title-ref".into(),
        body_ref: "line1\nraw body".into(),
        locale: None,
        sensitivity: "private".into(),
        category_id: None,
        collapse_key: None,
    };
    assert!(!raw_message.is_reference_only());

    let schedule = NotificationScheduleCommand {
        publish,
        schedule: NotificationSchedule {
            deliver_at_epoch_ms: Some(10),
            relative_delay_ms: None,
            timezone_id: Some("UTC".into()),
            expiry_epoch_ms: Some(100),
        },
    };
    assert!(schedule.has_schedule_preconditions());
}
