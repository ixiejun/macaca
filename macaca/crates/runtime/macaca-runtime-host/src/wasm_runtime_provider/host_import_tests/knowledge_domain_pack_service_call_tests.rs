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
async fn wasm_host_import_carries_email_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(EmailSystemServiceProvider::mock());
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
            macaca_proto::TraceContext::new("trace-email-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-email-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-email-abi-command",
        &service_id,
        "email.send",
        serde_json::to_value(EmailSendCommand {
            message: None,
            draft: Some(EmailDraftRef {
                draft_id: "draft:reference".into(),
                revision: "revision:reference".into(),
            }),
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
        Some(COMMUNICATION_EMAIL_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("email.send")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_citation_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(CitationSystemServiceProvider::mock());
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
            macaca_proto::TraceContext::new("trace-citations-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-citations-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-citations-abi-command",
        &service_id,
        "citations.resolve_identifier",
        serde_json::to_value(CitationsResolveIdentifierCommand::default()).unwrap(),
        "service.call",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(KNOWLEDGE_CITATIONS_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("citations.resolve_identifier")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_document_parsing_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(DocumentParsingSystemServiceProvider::mock());
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
            macaca_proto::TraceContext::new("trace-document-parsing-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-document-parsing-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-document-parsing-abi-command",
        &service_id,
        "document_parsing.start_parse_job",
        serde_json::to_value(DocumentParsingStartParseJobCommand::default()).unwrap(),
        "service.call",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("document_parsing.start_parse_job")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_retrieval_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(RetrievalSystemServiceProvider::mock());
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
            macaca_proto::TraceContext::new("trace-retrieval-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-retrieval-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-retrieval-abi-command",
        &service_id,
        "retrieval.retrieve",
        serde_json::to_value(RetrievalRetrieveCommand::default()).unwrap(),
        "service.call",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(KNOWLEDGE_RETRIEVAL_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("retrieval.retrieve")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_search_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(SearchSystemServiceProvider::mock());
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
            macaca_proto::TraceContext::new("trace-search-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-search-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-search-abi-command",
        &service_id,
        "search.search",
        serde_json::to_value(SearchSearchCommand::default()).unwrap(),
        "service.call",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(KNOWLEDGE_SEARCH_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("search.search")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_graph_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(GraphSystemServiceProvider::mock());
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
            macaca_proto::TraceContext::new("trace-graph-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-graph-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-graph-abi-command",
        &service_id,
        "graph.query",
        serde_json::to_value(GraphQueryCommand::default()).unwrap(),
        "service.call",
    );
    let result = session.dispatch(command).await.unwrap();
    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(KNOWLEDGE_GRAPH_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("graph.query")
    );
}

#[tokio::test]
async fn wasm_host_import_carries_summarization_dto_through_canonical_service_call() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider = Arc::new(SummarizationSystemServiceProvider::mock());
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
            macaca_proto::TraceContext::new("trace-summarization-start"),
        )
        .await
        .unwrap();
    let bridge = Arc::new(WasmHostImportBridge::new(
        Arc::clone(&runtime),
        WasmHostImportBridgeConfig::default(),
    ));
    let provider = DefaultInProcessWasmRuntimeProvider::default().with_host_import_bridge(bridge);
    let session = provider
        .create_session(traced_request("trace-summarization-abi-provider"))
        .await
        .unwrap();
    let command = host_import_command(
        "trace-summarization-abi-command",
        &service_id,
        "summarization.summarize",
        serde_json::to_value(SummarizationSummarizeCommand::default()).unwrap(),
        "service.call",
    );

    let result = session.dispatch(command).await.unwrap();

    assert!(matches!(result.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        result.metadata.get("service_id").map(String::as_str),
        Some(KNOWLEDGE_SUMMARIZATION_SERVICE_ID)
    );
    assert_eq!(
        result.metadata.get("service.operation").map(String::as_str),
        Some("summarization.summarize")
    );
}
