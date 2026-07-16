//! Canonical runtime, replay, and redaction tests for the summarization adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, KNOWLEDGE_SUMMARIZATION_COMMANDS,
};

use super::summarization_service_provider::{
    SummarizationRuntimeEventKind, SummarizationSystemServiceProvider,
};
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn summarization_commands_are_traceable_replayable_and_redacted_through_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(SummarizationSystemServiceProvider::mock());
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                provider.descriptor(),
                provider.clone(),
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("summarization-start"))
        .await
        .unwrap();

    let mut provider_events = provider.subscribe();
    for command in KNOWLEDGE_SUMMARIZATION_COMMANDS {
        let trace_id = format!("summarization-{command}");
        let reply = runtime
            .call(
                &service_id,
                ServiceBusSource::new("summarization-conformance"),
                ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({
                        "credential":"secret-marker",
                        "source":"private-marker",
                        "prompt":"raw-marker"
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

        let mut replayable = false;
        let mut command_phase_observed = false;
        while let Ok(event) = provider_events.try_recv() {
            replayable |= event.trace_id == trace_id
                && event.command == *command
                && event.replay_ref == format!("replay:{trace_id}");
            command_phase_observed |= event.trace_id == trace_id
                && event.kind == expected_event_kind(command)
                && event.command == *command;
        }
        assert!(
            replayable,
            "summarization command must emit a replay reference"
        );
        assert!(
            command_phase_observed,
            "{command} must emit its lifecycle phase"
        );
    }

    let observable = format!("{:?}", events.events().unwrap());
    for marker in ["secret-marker", "private-marker", "raw-marker"] {
        assert!(!observable.contains(marker));
    }
}

#[tokio::test]
async fn summarization_provider_emits_bounded_lifecycle_events() {
    let provider = SummarizationSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    provider.health().await.unwrap();
    provider.snapshot().await;
    let kinds = [
        events.try_recv().unwrap().kind,
        events.try_recv().unwrap().kind,
        events.try_recv().unwrap().kind,
    ];
    assert_eq!(
        kinds,
        [
            SummarizationRuntimeEventKind::PackDeclared,
            SummarizationRuntimeEventKind::HealthReported,
            SummarizationRuntimeEventKind::SnapshotRecorded,
        ]
    );

    let unavailable = SummarizationSystemServiceProvider::unavailable("not_installed");
    let mut unavailable_events = unavailable.subscribe();
    assert!(unavailable
        .call(command("summarization.summarize", "unavailable-event"))
        .await
        .is_err());
    assert_eq!(
        unavailable_events.try_recv().unwrap().kind,
        SummarizationRuntimeEventKind::Unavailable
    );

    let failed = SummarizationSystemServiceProvider::mock();
    let mut failed_events = failed.subscribe();
    assert!(failed
        .call(command("summarization.unsupported", "failure-event"))
        .await
        .is_err());
    assert_eq!(
        failed_events.try_recv().unwrap().kind,
        SummarizationRuntimeEventKind::Failure
    );
}

#[test]
fn summarization_capability_reports_all_provider_neutral_states() {
    let provider = SummarizationSystemServiceProvider::mock();
    for state in [
        macaca_proto::DomainPackProviderCapabilityState::Available,
        macaca_proto::DomainPackProviderCapabilityState::Degraded,
        macaca_proto::DomainPackProviderCapabilityState::Preview,
        macaca_proto::DomainPackProviderCapabilityState::Unavailable,
        macaca_proto::DomainPackProviderCapabilityState::Unsupported,
        macaca_proto::DomainPackProviderCapabilityState::Retired,
    ] {
        let capability = provider.capability_for_state(state);
        assert_eq!(capability.state, state);
        assert!(capability.modes.contains("hybrid"));
        assert!(capability.source_kinds.contains("document"));
        assert!(capability.languages.contains("und"));
    }
    let quota_limited = provider.capability_with_diagnostics(
        macaca_proto::DomainPackProviderCapabilityState::Degraded,
        true,
        Some("provider_quota_limited".into()),
    );
    assert!(quota_limited.quota_limited);
    assert_eq!(
        quota_limited.diagnostic_code.as_deref(),
        Some("provider_quota_limited")
    );
    assert_eq!(
        SummarizationSystemServiceProvider::unavailable("not-installed")
            .capability()
            .state,
        macaca_proto::DomainPackProviderCapabilityState::Unavailable
    );
}

#[tokio::test]
async fn summarization_provider_is_fail_closed_and_uses_bounded_mementos() {
    let unavailable = SummarizationSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("summarization.summarize", "none"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = SummarizationSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("summarization.unsupported", "bad"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("summarization.summarize", "one"))
        .await
        .unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "1");
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn summarization_plan_returns_resumable_bounded_map_reduce_memento() {
    let provider = SummarizationSystemServiceProvider::mock();
    let reply = provider
        .call(command("summarization.plan", "long-document"))
        .await
        .unwrap();
    assert_eq!(reply.output["strategy"], "long_document_synthesis");
    assert_eq!(
        reply.output["long_document_plan"]["partial_failure_policy"],
        "retain_successful_maps_and_resume_failed_maps"
    );
    assert!(reply.output["long_document_plan"]["checkpoint_ref"]
        .as_str()
        .unwrap()
        .contains("long_document_synthesis"));
}

#[tokio::test]
async fn summarization_provider_projects_bounded_partial_page_and_stream_results() {
    let provider = SummarizationSystemServiceProvider::mock();
    for (state, required_field) in [
        ("partial", "partial_result_ref"),
        ("paged", "next_cursor_ref"),
        ("streaming", "stream_frames"),
    ] {
        let reply = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("summarization.summarize"),
                serde_json::json!({"result_state":state}),
                TraceContext::new(format!("result-{state}")),
            ))
            .await
            .unwrap();
        assert_eq!(reply.status, state);
        assert!(!reply.output[required_field].is_null());
    }
}

#[tokio::test]
async fn cited_summary_fails_closed_when_declared_evidence_service_is_unavailable() {
    let provider = SummarizationSystemServiceProvider::mock();
    let command = ServiceCommand::with_trace(
        ServiceCommandName::new("summarization.summarize_with_citations"),
        serde_json::json!({"citation_service_available":false}),
        TraceContext::new("citation-unavailable"),
    );
    assert!(matches!(
        provider.call(command).await,
        Err(ServiceError::ServiceUnavailable(reason)) if reason == "summarization_declared_dependency_unavailable"
    ));
    let evidence_command = ServiceCommand::with_trace(
        ServiceCommandName::new("summarization.inspect_summary_evidence"),
        serde_json::json!({"evidence_service_available":false}),
        TraceContext::new("evidence-unavailable"),
    );
    assert!(matches!(
        provider.call(evidence_command).await,
        Err(ServiceError::ServiceUnavailable(reason)) if reason == "summarization_declared_dependency_unavailable"
    ));
}

fn command(name: &str, trace: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace),
    )
}

fn expected_event_kind(command: &str) -> SummarizationRuntimeEventKind {
    match command {
        "summarization.plan" => SummarizationRuntimeEventKind::PlanningCompleted,
        "summarization.validate_request" => SummarizationRuntimeEventKind::RequestValidated,
        "summarization.summarize"
        | "summarization.summarize_with_citations"
        | "summarization.summarize_many" => SummarizationRuntimeEventKind::SummaryGenerated,
        "summarization.summarize_conversation" => {
            SummarizationRuntimeEventKind::ConversationSummarized
        }
        "summarization.compress_context" => SummarizationRuntimeEventKind::ContextCompressed,
        "summarization.refine_summary" => SummarizationRuntimeEventKind::SummaryRefined,
        "summarization.compare_summaries" => SummarizationRuntimeEventKind::SummariesCompared,
        "summarization.evaluate_summary" => SummarizationRuntimeEventKind::SummaryEvaluated,
        "summarization.inspect_summary_evidence" => {
            SummarizationRuntimeEventKind::EvidenceInspected
        }
        _ => SummarizationRuntimeEventKind::ProviderInspected,
    }
}
