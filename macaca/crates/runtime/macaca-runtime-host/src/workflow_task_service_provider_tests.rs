use std::sync::Arc;

use macaca_proto::domain_pack_contract::workflow_task::WORKFLOW_TASK_COMMANDS;
use macaca_proto::domain_pack_contract::workflow_task_lifecycle_event::WorkflowTaskLifecycleEventKind;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext,
};

use super::workflow_task_service_provider::WorkflowTaskLifecycleSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};
use macaca_kernel::SystemService;

#[tokio::test]
async fn workflow_task_provider_emits_sanitized_events_for_every_descriptor_command() {
    let provider = WorkflowTaskLifecycleSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let mut published = Vec::new();
    for command in WORKFLOW_TASK_COMMANDS {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"raw_secret": "must-not-leak"}),
                TraceContext::new(format!("trace-{command}")),
            ))
            .await
            .unwrap();
        for expected_kind in expected_kinds(command) {
            let event = events.recv().await.unwrap();
            assert_eq!(event.kind, expected_kind);
            published.push(event);
        }
    }
    for event in &published {
        assert!(event.is_trace_safe());
        assert!(!serde_json::to_string(event)
            .unwrap()
            .contains("must-not-leak"));
    }
}

fn expected_kinds(command: &str) -> Vec<WorkflowTaskLifecycleEventKind> {
    use WorkflowTaskLifecycleEventKind as Kind;
    match command {
        "workflow_task.create" => vec![Kind::Created],
        "workflow_task.enqueue" => vec![Kind::Enqueued],
        "workflow_task.claim" => vec![Kind::Claimed],
        "workflow_task.heartbeat" => vec![Kind::HeartbeatRecorded],
        "workflow_task.release" => vec![Kind::LeaseRevoked],
        "workflow_task.record_progress" => vec![Kind::ProgressRecorded],
        "workflow_task.record_checkpoint" => vec![Kind::CheckpointRecorded],
        "workflow_task.attach_artifact" => vec![Kind::ArtifactAttached],
        "workflow_task.complete" => vec![Kind::Completed],
        "workflow_task.fail" => vec![Kind::Failed, Kind::RetryScheduled],
        "workflow_task.cancel" => vec![Kind::Cancelled],
        "workflow_task.skip" => vec![Kind::Skipped],
        "workflow_task.snapshot" => vec![Kind::SnapshotRecorded],
        "workflow_task.inspect_provider" => vec![Kind::PackDeclared],
        "workflow_task.update"
        | "workflow_task.patch_metadata"
        | "workflow_task.get"
        | "workflow_task.list"
        | "workflow_task.get_history" => vec![Kind::AdmissionValidated],
        _ => panic!("workflow task descriptor declared an unknown command: {command}"),
    }
}

#[tokio::test]
async fn workflow_task_provider_redacts_every_sensitive_payload_class() {
    let provider = WorkflowTaskLifecycleSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let markers = [
        "raw-prompt-marker",
        "input-payload-marker",
        "artifact-marker",
        "provider-payload-marker",
        "worker-diagnostic-marker",
        "history-event-marker",
        "snapshot-marker",
        "log-marker",
    ];
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("workflow_task.fail"),
            serde_json::json!({
                "prompt": markers[0],
                "input": markers[1],
                "artifact": markers[2],
                "provider_payload": markers[3],
                "worker_diagnostic": markers[4],
                "history": markers[5],
                "snapshot": markers[6],
                "log": markers[7],
            }),
            TraceContext::new("trace-redaction"),
        ))
        .await
        .unwrap();
    let emitted = [events.recv().await.unwrap(), events.recv().await.unwrap()];
    let observable = format!(
        "{}{}",
        result.output,
        serde_json::to_string(&emitted).unwrap()
    );
    for marker in markers {
        assert!(!observable.contains(marker));
    }
}

#[tokio::test]
async fn workflow_task_provider_reports_unsupported_and_unavailable_without_side_effects() {
    let provider = WorkflowTaskLifecycleSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let unsupported = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("workflow_task.not_declared"),
            serde_json::json!({}),
            TraceContext::new("trace-unsupported"),
        ))
        .await
        .unwrap_err();
    assert!(matches!(unsupported, ServiceError::UnsupportedCommand(_)));
    assert!(events.try_recv().is_err());

    let unavailable = WorkflowTaskLifecycleSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    let error = unavailable
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("workflow_task.create"),
            serde_json::json!({}),
            TraceContext::new("trace-unavailable"),
        ))
        .await
        .unwrap_err();
    assert!(matches!(error, ServiceError::ServiceUnavailable(_)));
}

#[tokio::test]
async fn workflow_task_provider_lifecycle_and_output_remain_bounded() {
    let provider = WorkflowTaskLifecycleSystemServiceProvider::mock();
    provider.start().await.unwrap();
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("workflow_task.snapshot"),
            serde_json::json!({"unbounded": "x".repeat(20_000)}),
            TraceContext::new("trace-bounded-output"),
        ))
        .await
        .unwrap();
    assert!(result.output.to_string().len() < 512);
    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Healthy
    ));
    provider.stop().await.unwrap();
    provider.cleanup().await.unwrap();
}

#[tokio::test]
async fn workflow_task_terminal_events_remain_trace_addressable_after_runtime_restart() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    }));
    let provider = Arc::new(WorkflowTaskLifecycleSystemServiceProvider::mock());
    let mut lifecycle_events = provider.subscribe();
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-workflow-task-start"))
        .await
        .unwrap();

    dispatch_runtime_command(&runtime, &service_id, "workflow_task.claim", "trace-claim").await;
    assert_eq!(
        lifecycle_events.recv().await.unwrap().kind,
        WorkflowTaskLifecycleEventKind::Claimed
    );
    runtime
        .stop(&service_id, TraceContext::new("trace-workflow-task-stop"))
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            TraceContext::new("trace-workflow-task-restart"),
        )
        .await
        .unwrap();

    for (command, trace_id, expected) in [
        (
            "workflow_task.complete",
            "trace-complete",
            WorkflowTaskLifecycleEventKind::Completed,
        ),
        (
            "workflow_task.fail",
            "trace-fail",
            WorkflowTaskLifecycleEventKind::Failed,
        ),
        (
            "workflow_task.cancel",
            "trace-cancel",
            WorkflowTaskLifecycleEventKind::Cancelled,
        ),
        (
            "workflow_task.skip",
            "trace-skip",
            WorkflowTaskLifecycleEventKind::Skipped,
        ),
    ] {
        dispatch_runtime_command(&runtime, &service_id, command, trace_id).await;
        assert_eq!(lifecycle_events.recv().await.unwrap().kind, expected);
        if command == "workflow_task.fail" {
            assert_eq!(
                lifecycle_events.recv().await.unwrap().kind,
                WorkflowTaskLifecycleEventKind::RetryScheduled
            );
        }
    }

    let runtime_events = events.events().unwrap();
    for trace_id in [
        "trace-claim",
        "trace-complete",
        "trace-fail",
        "trace-cancel",
        "trace-skip",
    ] {
        assert!(runtime_events.iter().any(|event| {
            event.trace_id.as_deref() == Some(trace_id)
                && event.operation == "service_runtime.call.dispatched"
        }));
        assert!(runtime_events.iter().any(|event| {
            event.trace_id.as_deref() == Some(trace_id)
                && event.operation == "service_runtime.call.completed"
        }));
    }
}

async fn dispatch_runtime_command(
    runtime: &ServiceRuntime,
    service_id: &macaca_proto::KernelServiceId,
    command: &str,
    trace_id: &str,
) {
    runtime
        .call(
            service_id,
            ServiceBusSource::new("workflow-task-replay-test"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({}),
                TraceContext::new(trace_id),
            ),
        )
        .await
        .unwrap();
}
