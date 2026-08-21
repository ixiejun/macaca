//! Canonical runtime and redaction tests for foundation session state.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, FOUNDATION_SESSION_STATE_COMMANDS,
};

use super::foundation_session_state_service_provider::FoundationSessionStateSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn session_state_commands_are_traceable_without_raw_state_echo() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(FoundationSessionStateSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("session-state-start"))
        .await
        .unwrap();
    for command in FOUNDATION_SESSION_STATE_COMMANDS {
        let trace_id = format!("session-state-{command}");
        let reply = runtime
            .call(
                &service_id,
                ServiceBusSource::new("session-state-conformance"),
                ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({
                        "raw_state": "secret-marker",
                        "raw_secret": "secret-reference-marker",
                        "prompt": "private-prompt-marker",
                        "manifest": "manifest-bytes-marker",
                        "package_bytes": "package-bytes-marker",
                        "credentials": "credential-marker",
                        "private_key": "private-key-marker",
                        "provider_payload": "provider-payload-marker",
                        "unbounded_output": "unbounded-output-marker".repeat(2_048)
                    }),
                    TraceContext::new(trace_id.clone()),
                ),
            )
            .await
            .unwrap();
        assert_eq!(reply.status, "ok");
        assert!(!reply.output.unwrap().to_string().contains("marker"));
        assert!(events.events().unwrap().iter().any(|event| {
            event.trace_id.as_deref() == Some(trace_id.as_str())
                && event.operation == "service_runtime.call.completed"
        }));
    }
    let observable = format!("{:?}", events.events().unwrap());
    for marker in [
        "secret-marker",
        "secret-reference-marker",
        "private-prompt-marker",
        "manifest-bytes-marker",
        "package-bytes-marker",
        "credential-marker",
        "private-key-marker",
        "provider-payload-marker",
        "unbounded-output-marker",
    ] {
        assert!(
            !observable.contains(marker),
            "observability leaked {marker}"
        );
    }
}

#[tokio::test]
async fn session_state_provider_reports_snapshot_and_unavailable_state() {
    let provider = FoundationSessionStateSystemServiceProvider::mock();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("session_state.put"),
            serde_json::json!({"value":"secret-marker"}),
            TraceContext::new("session-state-reference"),
        ))
        .await
        .unwrap();
    let snapshot = provider.snapshot().await;
    assert_eq!(snapshot.provider_class, "mock");
    assert_eq!(snapshot.revision_hashes.len(), 1);
    assert_eq!(snapshot.redaction_summary.redacted_value_count, 1);
    provider.shutdown().await.unwrap();
    assert!(provider.snapshot().await.revision_hashes.is_empty());

    let unavailable = FoundationSessionStateSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("session_state.get"),
                serde_json::json!({}),
                TraceContext::new("session-state-unavailable"),
            ))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
}
