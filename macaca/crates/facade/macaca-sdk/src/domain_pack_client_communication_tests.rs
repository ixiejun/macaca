use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    COMMUNICATION_CALENDAR_PACK_ID, COMMUNICATION_CALENDAR_SERVICE_ID, COMMUNICATION_EMAIL_PACK_ID,
    COMMUNICATION_EMAIL_SERVICE_ID, COMMUNICATION_INBOX_PACK_ID, COMMUNICATION_INBOX_SERVICE_ID,
    COMMUNICATION_MESSAGING_PACK_ID, COMMUNICATION_MESSAGING_SERVICE_ID,
    COMMUNICATION_NOTIFICATION_PACK_ID, COMMUNICATION_NOTIFICATION_SERVICE_ID,
};

use super::*;

// These tests keep communication pack discovery in a sibling module so the core
// SDK client tests stay below the repository's source-size ceiling.

#[tokio::test]
async fn catalog_client_discovers_communication_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            COMMUNICATION_EMAIL_PACK_ID,
            COMMUNICATION_EMAIL_SERVICE_ID,
            "email.send",
            "email_provider_not_installed",
            "transactional-mail",
        ),
        (
            COMMUNICATION_MESSAGING_PACK_ID,
            COMMUNICATION_MESSAGING_SERVICE_ID,
            "messaging.send_message",
            "messaging_provider_not_installed",
            "conversation-bridge",
        ),
        (
            COMMUNICATION_NOTIFICATION_PACK_ID,
            COMMUNICATION_NOTIFICATION_SERVICE_ID,
            "notification.publish",
            "notification_provider_not_installed",
            "push-bridge",
        ),
        (
            COMMUNICATION_INBOX_PACK_ID,
            COMMUNICATION_INBOX_SERVICE_ID,
            "inbox.sync_sources",
            "inbox_provider_not_installed",
            "source-sync",
        ),
        (
            COMMUNICATION_CALENDAR_PACK_ID,
            COMMUNICATION_CALENDAR_SERVICE_ID,
            "calendar.create_event",
            "calendar_provider_not_installed",
            "calendar-sync",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid communication id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("communication descriptor exists");

        assert!(!pack.is_callable());
        assert_eq!(
            pack.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(pack
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|commands| commands.contains(command)));
        assert!(pack
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(pack
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/communication"));
    }
}

#[tokio::test]
async fn inbox_sdk_discovery_serializes_only_descriptor_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(COMMUNICATION_INBOX_PACK_ID)
                .expect("inbox pack id must be valid"),
        )
        .await
        .unwrap();

    // Discovery serializes immutable metadata, never connector request content.
    let diagnostic = serde_json::to_string(&inspect).unwrap();
    for marker in [
        "credential=inbox-secret",
        "oauth-access-token",
        "webhook-secret",
        "raw-provider-payload",
        "raw-full-body",
        "raw-attachment-bytes",
        "unbounded-content",
    ] {
        assert!(
            !diagnostic.contains(marker),
            "SDK diagnostic leaked {marker}"
        );
    }
    assert!(diagnostic.contains("inbox_provider_not_installed"));
    assert!(diagnostic.contains("redaction_policy"));
}
