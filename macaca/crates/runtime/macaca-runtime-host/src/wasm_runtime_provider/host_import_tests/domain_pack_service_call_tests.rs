//! Contract tests proving generic `service.call` host imports route through ServiceRuntime.

use std::sync::Arc;

use macaca_proto::ApplicationHostCommandStatus;

use super::super::{
    DefaultInProcessWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmHostImportBridge,
    WasmHostImportBridgeConfig,
};
use crate::{
    calendar_service_provider::CalendarSystemServiceProvider,
    citation_service_provider::CitationSystemServiceProvider,
    document_parsing_service_provider::DocumentParsingSystemServiceProvider,
    email_service_provider::EmailSystemServiceProvider,
    graph_service_provider::GraphSystemServiceProvider,
    inbox_service_provider::InboxSystemServiceProvider,
    messaging_service_provider::MessagingSystemServiceProvider,
    notification_service_provider::NotificationSystemServiceProvider,
    retrieval_service_provider::RetrievalSystemServiceProvider,
    search_service_provider::SearchSystemServiceProvider,
    summarization_service_provider::SummarizationSystemServiceProvider,
    workflow_task_service_provider::WorkflowTaskLifecycleSystemServiceProvider,
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::workflow_task::{
    WorkflowTaskCreateCommand, WORKFLOW_TASK_SERVICE_ID,
};
use macaca_proto::{CalendarCreateEventCommand, CalendarEvent, COMMUNICATION_CALENDAR_SERVICE_ID};
use macaca_proto::{CitationsResolveIdentifierCommand, KNOWLEDGE_CITATIONS_SERVICE_ID};
use macaca_proto::{DocumentParsingStartParseJobCommand, KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID};
use macaca_proto::{EmailDraftRef, EmailSendCommand, COMMUNICATION_EMAIL_SERVICE_ID};
use macaca_proto::{GraphQueryCommand, KNOWLEDGE_GRAPH_SERVICE_ID};
use macaca_proto::{InboxFetchBodyCommand, COMMUNICATION_INBOX_SERVICE_ID};
use macaca_proto::{
    MessagingContent, MessagingConversationKind, MessagingConversationRef,
    MessagingSendMessageCommand, MessagingSenderRef, COMMUNICATION_MESSAGING_SERVICE_ID,
};
use macaca_proto::{NotificationPublishCommand, COMMUNICATION_NOTIFICATION_SERVICE_ID};
use macaca_proto::{RetrievalRetrieveCommand, KNOWLEDGE_RETRIEVAL_SERVICE_ID};
use macaca_proto::{SearchSearchCommand, KNOWLEDGE_SEARCH_SERVICE_ID};
use macaca_proto::{SummarizationSummarizeCommand, KNOWLEDGE_SUMMARIZATION_SERVICE_ID};

use super::support::{host_import_command, register_mock_service, traced_request};
#[tokio::test]
async fn wasm_host_import_carries_workflow_task_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(WorkflowTaskLifecycleSystemServiceProvider::mock());
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            macaca_proto::TraceContext::new("trace-workflow-task-start"),
        )
        .await
        .unwrap();
    assert_eq!(service_id.as_str(), WORKFLOW_TASK_SERVICE_ID);

    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-workflow-task-abi-provider"))
        .await
        .unwrap();
    let task_command = WorkflowTaskCreateCommand::default();
    let command = host_import_command(
        "trace-workflow-task-abi-command",
        &service_id,
        "workflow_task.create",
        serde_json::to_value(task_command).unwrap(),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(WORKFLOW_TASK_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("workflow_task.create")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_notification_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(NotificationSystemServiceProvider::mock());
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            macaca_proto::TraceContext::new("trace-notification-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-notification-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-notification-abi-command",
        &service_id,
        "notification.publish",
        serde_json::to_value(NotificationPublishCommand {
            message: macaca_proto::NotificationMessage {
                title_ref: "artifact:title".into(),
                body_ref: "artifact:body".into(),
                locale: None,
                sensitivity: "private".into(),
                category_id: None,
                collapse_key: None,
            },
            target: macaca_proto::NotificationTarget {
                target_id: "target:one".into(),
                target_kind: "user".into(),
                subscription: None,
                redaction_label: "recipient".into(),
            },
            channel: macaca_proto::NotificationDeliveryChannel::InApp,
            client_request_id: "request:one".into(),
        })
        .unwrap(),
        "service.call",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(COMMUNICATION_NOTIFICATION_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("notification.publish")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_calendar_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(CalendarSystemServiceProvider::mock());
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            macaca_proto::TraceContext::new("trace-calendar-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-calendar-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-calendar-abi-command",
        &service_id,
        "calendar.create_event",
        serde_json::to_value(CalendarCreateEventCommand {
            event: CalendarEvent {
                event_id: "event:reference".into(),
                source_id: "source:reference".into(),
                title_ref: "artifact:title".into(),
                description_ref: None,
                start_epoch_ms: 1,
                end_epoch_ms: 2,
                timezone_id: "UTC".into(),
                recurrence: None,
                attendees: Vec::new(),
            },
            idempotency_key: "request:reference".into(),
        })
        .unwrap(),
        "service.call",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(COMMUNICATION_CALENDAR_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("calendar.create_event")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_inbox_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(InboxSystemServiceProvider::mock());
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                provider.descriptor(),
                provider,
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            macaca_proto::TraceContext::new("trace-inbox-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-inbox-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-inbox-abi-command",
        &service_id,
        "inbox.fetch_body",
        serde_json::to_value(InboxFetchBodyCommand {
            item_id: "item:reference".into(),
            body_part: "body:reference".into(),
            max_bytes: 1024,
        })
        .unwrap(),
        "service.call",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(COMMUNICATION_INBOX_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("inbox.fetch_body")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_messaging_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(MessagingSystemServiceProvider::mock());
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                provider.descriptor(),
                provider,
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            macaca_proto::TraceContext::new("trace-messaging-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-messaging-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-messaging-abi-command",
        &service_id,
        "messaging.send_message",
        serde_json::to_value(MessagingSendMessageCommand {
            sender: MessagingSenderRef {
                sender_id: "sender:reference".into(),
                verified: true,
                provider_class: "mock".into(),
                secret_ref: Some("secret:reference".into()),
            },
            conversation: MessagingConversationRef {
                conversation_id: "conversation:reference".into(),
                provider_class: "mock".into(),
                kind: MessagingConversationKind::Channel,
                tenant_scope: "tenant:reference".into(),
                visibility: "private".into(),
            },
            content: MessagingContent {
                fallback_text_ref: "artifact:fallback".into(),
                content_ref: Some("artifact:content".into()),
                format: "reference".into(),
                formatting_policy: "redacted".into(),
            },
            attachments: Vec::new(),
            approval_ref: Some("approval:reference".into()),
            idempotency_key: "request:reference".into(),
        })
        .unwrap(),
        "service.call",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(COMMUNICATION_MESSAGING_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("messaging.send_message")
    );
}
