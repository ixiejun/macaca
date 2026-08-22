//! Conformance tests for the metadata-only transcription service Strategy.

use macaca_kernel::SystemService;
use macaca_proto::media_transcription::MEDIA_TRANSCRIPTION_COMMANDS;
use macaca_proto::{
    ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext,
    TranscriptionPreflightFacts,
};

use super::media_transcription_service_provider::MediaTranscriptionSystemServiceProvider;

#[test]
fn every_transcription_command_has_a_specific_audit_event() {
    let expected = [
        (
            "transcription.inspect_provider",
            "transcription.provider_inspected",
        ),
        (
            "transcription.import_source_request",
            "transcription.source_import_requested",
        ),
        ("transcription.open_source", "transcription.source_opened"),
        (
            "transcription.inspect_media",
            "transcription.media_inspected",
        ),
        ("transcription.plan_batch", "transcription.batch_planned"),
        (
            "transcription.batch_request",
            "transcription.batch_requested",
        ),
        ("transcription.plan_stream", "transcription.stream_planned"),
        ("transcription.start_stream", "transcription.stream_started"),
        (
            "transcription.append_stream_chunk",
            "transcription.stream_chunk_appended",
        ),
        (
            "transcription.finish_stream",
            "transcription.stream_finished",
        ),
        (
            "transcription.cancel_stream",
            "transcription.stream_cancelled",
        ),
        (
            "transcription.plan_diarization",
            "transcription.diarization_planned",
        ),
        (
            "transcription.diarization_request",
            "transcription.diarization_requested",
        ),
        (
            "transcription.align_timestamps",
            "transcription.timestamps_aligned",
        ),
        (
            "transcription.normalize_transcript",
            "transcription.transcript_normalized",
        ),
        (
            "transcription.plan_redaction",
            "transcription.redaction_planned",
        ),
        (
            "transcription.redaction_request",
            "transcription.redaction_requested",
        ),
        (
            "transcription.plan_subtitle_export",
            "transcription.subtitle_export_planned",
        ),
        (
            "transcription.subtitle_export_request",
            "transcription.subtitle_export_requested",
        ),
        (
            "transcription.plan_translation_handoff",
            "transcription.translation_handoff_planned",
        ),
        (
            "transcription.translation_handoff_request",
            "transcription.translation_handoff_requested",
        ),
        ("transcription.inspect_job", "transcription.job_inspected"),
        (
            "transcription.get_artifact_handle",
            "transcription.artifact_handle_created",
        ),
    ];
    assert_eq!(expected.len(), MEDIA_TRANSCRIPTION_COMMANDS.len());
    for command in MEDIA_TRANSCRIPTION_COMMANDS {
        assert!(expected.iter().any(|(known, _)| known == command));
    }
}

#[tokio::test]
async fn declared_transcription_commands_are_traceable_and_redacted() {
    let provider = MediaTranscriptionSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in MEDIA_TRANSCRIPTION_COMMANDS {
        let trace_id = format!("transcription-{command}");
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({
                    "raw_audio": "private-audio-marker",
                    "raw_transcript": "private-transcript-marker",
                    "credentials": "credential-marker"
                }),
                TraceContext::new(trace_id.clone()),
            ))
            .await
            .unwrap();
        assert_eq!(result.output["status"], "metadata_only");
        assert!(!result.output.to_string().contains("marker"));
        let event = receive_outcome(&mut events, "completed").await;
        assert_eq!(event.trace_id, trace_id);
        assert_eq!(event.outcome, "completed");
    }
}

#[tokio::test]
async fn unavailable_provider_fails_closed_for_every_declared_command() {
    let provider = MediaTranscriptionSystemServiceProvider::unavailable("module_absent");
    let mut events = provider.subscribe();
    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    for command in MEDIA_TRANSCRIPTION_COMMANDS {
        let raw_marker = "must-not-process-or-observe";
        let error = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"raw_audio":raw_marker,"raw_transcript":raw_marker}),
                TraceContext::new(format!("unavailable-{command}")),
            ))
            .await
            .unwrap_err();
        assert!(matches!(error, ServiceError::ServiceUnavailable(_)));
        let event = receive_outcome(&mut events, "unavailable").await;
        assert_eq!(event.outcome, "unavailable");
        assert!(!event.command.contains(raw_marker));
        assert!(!event.replay_ref.contains(raw_marker));
    }
    assert_eq!(provider.capability().provider_class, "unavailable");
}

#[tokio::test]
async fn unknown_command_is_structured_unsupported() {
    let provider = MediaTranscriptionSystemServiceProvider::mock().with_admission_facts(
        TranscriptionPreflightFacts {
            permission_granted: false,
            ..TranscriptionPreflightFacts::permissive()
        },
    );
    let error = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("transcription.provider_native"),
            serde_json::json!({}),
            TraceContext::new("transcription-unsupported"),
        ))
        .await
        .unwrap_err();
    assert!(matches!(error, ServiceError::UnsupportedCommand(_)));
}

#[tokio::test]
async fn preflight_rejections_do_not_complete_provider_work() {
    let provider = MediaTranscriptionSystemServiceProvider::mock().with_admission_facts(
        TranscriptionPreflightFacts {
            permission_granted: false,
            ..TranscriptionPreflightFacts::permissive()
        },
    );
    let mut events = provider.subscribe();
    let command = ServiceCommand::with_trace(
        ServiceCommandName::new("transcription.batch_request"),
        serde_json::json!({"raw_audio":"must-not-process"}),
        TraceContext::new("transcription-preflight"),
    );
    let error = provider.call(command).await.unwrap_err();
    assert!(matches!(error, ServiceError::DisabledByPolicy(_)));
    let event = receive_outcome(&mut events, "preflight_rejected").await;
    assert_eq!(event.outcome, "preflight_rejected");
}

#[tokio::test]
async fn host_issued_rejections_never_complete_or_observe_transcription_payloads() {
    let rejected = [
        TranscriptionPreflightFacts {
            scope_granted: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            policy_granted: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            entitlement_granted: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            schema_valid: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            language_supported: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            diarization_supported: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            redaction_allowed: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            translation_allowed: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            requested_units: 2,
            reserved_units: 1,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            approval_required: true,
            approval_granted: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            within_timeout: false,
            ..TranscriptionPreflightFacts::permissive()
        },
        TranscriptionPreflightFacts {
            cancellation_requested: true,
            ..TranscriptionPreflightFacts::permissive()
        },
    ];
    for (index, facts) in rejected.into_iter().enumerate() {
        let provider = MediaTranscriptionSystemServiceProvider::mock().with_admission_facts(facts);
        let mut events = provider.subscribe();
        let marker = "must-not-enter-transcription-provider";
        assert!(provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("transcription.batch_request"),
                serde_json::json!({"raw_audio":marker,"raw_transcript":marker}),
                TraceContext::new(format!("transcription-rejected-{index}"))
            ))
            .await
            .is_err());
        let event = receive_outcome(&mut events, "preflight_rejected").await;
        assert_eq!(event.outcome, "preflight_rejected");
        assert!(!event.command.contains(marker));
        assert!(!event.replay_ref.contains(marker));
    }
}

#[tokio::test]
async fn side_effecting_requests_are_idempotent_and_stale_sources_do_not_complete() {
    let provider = MediaTranscriptionSystemServiceProvider::mock();
    let mut first = ServiceCommand::with_trace(
        ServiceCommandName::new("transcription.batch_request"),
        serde_json::json!({}),
        TraceContext::new("transcription-first"),
    );
    first
        .metadata
        .insert("idempotency_key".into(), "request-once".into());
    let first_ref = provider.call(first).await.unwrap().output["artifact_ref"].clone();
    let mut replay = ServiceCommand::with_trace(
        ServiceCommandName::new("transcription.batch_request"),
        serde_json::json!({}),
        TraceContext::new("transcription-replay"),
    );
    replay
        .metadata
        .insert("idempotency_key".into(), "request-once".into());
    assert_eq!(
        provider.call(replay).await.unwrap().output["artifact_ref"],
        first_ref
    );

    let mut events = provider.subscribe();
    let mut stale = ServiceCommand::with_trace(
        ServiceCommandName::new("transcription.batch_request"),
        serde_json::json!({}),
        TraceContext::new("transcription-stale"),
    );
    stale.metadata.insert("source_version".into(), "v1".into());
    stale
        .metadata
        .insert("current_source_version".into(), "v2".into());
    assert!(matches!(
        provider.call(stale).await,
        Err(ServiceError::AdapterFailure(_))
    ));
    assert_eq!(
        receive_outcome(&mut events, "precondition_rejected")
            .await
            .outcome,
        "precondition_rejected"
    );
}

#[tokio::test]
async fn replay_snapshot_is_bounded_and_every_declared_command_has_trace_evidence() {
    let provider = MediaTranscriptionSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in MEDIA_TRANSCRIPTION_COMMANDS {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"raw_transcript":"must-not-enter-replay"}),
                TraceContext::new(format!("replay-{command}")),
            ))
            .await
            .unwrap();
        assert_eq!(
            receive_outcome(&mut events, "completed").await.replay_ref,
            format!("replay:transcription:replay-{command}")
        );
    }
    let snapshot = provider.snapshot().await;
    assert_eq!(snapshot["snapshot_schema"], "media.transcription.replay.v1");
    assert_eq!(
        snapshot["command_count"],
        MEDIA_TRANSCRIPTION_COMMANDS.len().to_string()
    );
    assert!(snapshot
        .values()
        .all(|value| !value.contains("must-not-enter-replay")));
    assert_eq!(
        receive_outcome(&mut events, "snapshot_recorded")
            .await
            .outcome,
        "snapshot_recorded"
    );
}

#[tokio::test]
async fn replay_references_remain_trace_addressable_after_provider_restart() {
    let trace_id = "transcription-restart-trace";
    let first = MediaTranscriptionSystemServiceProvider::mock();
    let _first_result = first
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("transcription.inspect_provider"),
            serde_json::json!({"raw_audio":"must-not-enter-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    let first_ref = format!("replay:transcription:{trace_id}");
    first.cleanup().await.unwrap();

    // A new provider instance represents a host refresh/restart. The canonical
    // trace-derived replay reference remains deterministic and payload-free.
    let restarted = MediaTranscriptionSystemServiceProvider::mock();
    let mut events = restarted.subscribe();
    let restarted_result = restarted
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("transcription.inspect_provider"),
            serde_json::json!({"raw_transcript":"must-not-enter-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    let event = events.recv().await.unwrap();
    assert_eq!(restarted_result.trace.trace_id, trace_id);
    assert_eq!(event.replay_ref, first_ref);
    assert_eq!(event.trace_id, trace_id);
    assert!(!event.replay_ref.contains("must-not-enter-replay"));
}

#[tokio::test]
async fn transcription_emits_stable_audit_taxonomy() {
    let provider = MediaTranscriptionSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    provider.health().await.unwrap();
    for command in [
        "transcription.inspect_provider",
        "transcription.import_source_request",
        "transcription.inspect_media",
        "transcription.plan_batch",
        "transcription.plan_stream",
        "transcription.start_stream",
        "transcription.plan_diarization",
        "transcription.align_timestamps",
        "transcription.normalize_transcript",
        "transcription.plan_redaction",
        "transcription.plan_subtitle_export",
        "transcription.plan_translation_handoff",
        "transcription.inspect_job",
        "transcription.get_artifact_handle",
    ] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({"audio":"redacted","transcript":"redacted"}),
                TraceContext::new(command),
            ))
            .await
            .unwrap();
    }
    let mut names = Vec::new();
    while let Ok(event) = events.try_recv() {
        names.push(event.event_name);
    }
    for expected in [
        "transcription.pack_declared",
        "transcription.health",
        "transcription.admission",
        "transcription.provider_inspection",
        "transcription.policy",
        "transcription.entitlement",
        "transcription.resource",
        "transcription.approval",
        "transcription.source_import_requested",
        "transcription.media_inspected",
        "transcription.batch_planned",
        "transcription.stream_planned",
        "transcription.stream_started",
        "transcription.diarization_planned",
        "transcription.timestamps_aligned",
        "transcription.transcript_normalized",
        "transcription.redaction_planned",
        "transcription.subtitle_export_planned",
        "transcription.translation_handoff_planned",
        "transcription.job_inspected",
        "transcription.artifact_handle_created",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
    }
}

async fn receive_outcome(
    events: &mut tokio::sync::broadcast::Receiver<
        super::media_transcription_service_provider::TranscriptionRuntimeEvent,
    >,
    expected: &str,
) -> super::media_transcription_service_provider::TranscriptionRuntimeEvent {
    loop {
        let event = events.recv().await.unwrap();
        if event.outcome == expected {
            return event;
        }
    }
}
