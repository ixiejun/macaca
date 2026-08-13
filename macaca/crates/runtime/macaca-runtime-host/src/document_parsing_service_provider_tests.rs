//! Canonical runtime and redaction tests for the document parsing adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, KNOWLEDGE_DOCUMENT_PARSING_COMMANDS,
};

use super::document_parsing_service_provider::DocumentParsingSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn document_parsing_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
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
        .start(&service_id, TraceContext::new("document-parsing-start"))
        .await
        .unwrap();
    for command in KNOWLEDGE_DOCUMENT_PARSING_COMMANDS {
        let trace_id = format!("document-parsing-{command}");
        let reply = runtime.call(&service_id, ServiceBusSource::new("document-parsing-conformance"), ServiceCommand::with_trace(ServiceCommandName::new(*command), serde_json::json!({"credential":"secret-marker", "document":"raw-marker", "ocr_image":"private-marker"}), TraceContext::new(trace_id.clone()))).await.unwrap();
        assert_eq!(reply.status, "ok");
        assert!(!reply.output.unwrap().to_string().contains("marker"));
        assert!(events
            .events()
            .unwrap()
            .iter()
            .any(|event| event.trace_id.as_deref() == Some(trace_id.as_str())
                && event.operation == "service_runtime.call.completed"));
    }
    let observable = format!("{:?}", events.events().unwrap());
    for marker in ["secret-marker", "raw-marker", "private-marker"] {
        assert!(!observable.contains(marker));
    }
}

#[tokio::test]
async fn document_parsing_provider_fails_closed_and_cleans_bounded_snapshot_state() {
    let unavailable = DocumentParsingSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("document_parsing.parse_document", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = DocumentParsingSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("document_parsing.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("document_parsing.start_parse_job", "one"))
        .await
        .unwrap();
    let snapshot = provider.snapshot().await;
    assert_eq!(snapshot["active_job_count"], "1");
    let observable = format!("{snapshot:?}");
    for marker in [
        "secret-marker",
        "raw-marker",
        "private-marker",
        "raw-ocr-image",
    ] {
        assert!(!observable.contains(marker));
    }
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_job_count"], "0");
}

#[test]
fn document_parsing_mock_capability_reports_only_bounded_generic_facts() {
    let capability = DocumentParsingSystemServiceProvider::mock().capability();
    for feature in ["ocr", "layout", "tables", "forms", "async_jobs"] {
        assert!(capability.supported_features.contains(feature));
    }
    assert_eq!(capability.max_bytes, 65_536);
    assert_eq!(capability.max_pages, 100);
    assert_eq!(capability.max_output_bytes, 65_536);
    assert!(capability.supports_health);
    assert!(!capability.rate_limit_bucket.contains("credential"));
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
