//! Conformance tests for the metadata-only transcription service Strategy.

use macaca_kernel::SystemService;
use macaca_proto::media_transcription::MEDIA_TRANSCRIPTION_COMMANDS;
use macaca_proto::{
    ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext,
    TranscriptionPreflightFacts,
};

use super::media_transcription_service_provider::MediaTranscriptionSystemServiceProvider;

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
        let event = events.recv().await.unwrap();
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
        let event = events.recv().await.unwrap();
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
    let event = events.recv().await.unwrap();
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
        let event = events.recv().await.unwrap();
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
        events.recv().await.unwrap().outcome,
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
            events.recv().await.unwrap().replay_ref,
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
    assert_eq!(events.recv().await.unwrap().outcome, "snapshot_recorded");
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
