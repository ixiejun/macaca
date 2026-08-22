//! Conformance tests for the metadata-only audio service Strategy.

use macaca_kernel::SystemService;
use macaca_proto::media_audio::MEDIA_AUDIO_COMMANDS;
use macaca_proto::{
    AudioPreflightFacts, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext,
};

use super::media_audio_service_provider::MediaAudioSystemServiceProvider;

#[tokio::test]
async fn declared_audio_commands_are_traceable_and_redacted() {
    let provider = MediaAudioSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in MEDIA_AUDIO_COMMANDS {
        let marker = "private-audio-prompt-marker";
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"raw_audio":marker,"prompt":marker,"credentials":marker}),
                TraceContext::new(format!("audio-{command}")),
            ))
            .await
            .unwrap();
        assert_eq!(result.output["status"], "metadata_only");
        assert!(!result.output.to_string().contains(marker));
        assert_eq!(
            receive_outcome(&mut events, "completed").await.outcome,
            "completed"
        );
    }
}

#[tokio::test]
async fn unavailable_audio_provider_fails_closed_for_every_command() {
    let provider = MediaAudioSystemServiceProvider::unavailable("module_absent");
    let mut events = provider.subscribe();
    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    for command in MEDIA_AUDIO_COMMANDS {
        assert!(matches!(
            provider
                .call(ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({"raw_audio":"must-not-process"}),
                    TraceContext::new(format!("unavailable-{command}"))
                ))
                .await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
        assert_eq!(
            receive_outcome(&mut events, "unavailable").await.outcome,
            "unavailable"
        );
    }
}

async fn receive_outcome(
    events: &mut tokio::sync::broadcast::Receiver<
        super::media_audio_service_provider::AudioRuntimeEvent,
    >,
    expected: &str,
) -> super::media_audio_service_provider::AudioRuntimeEvent {
    loop {
        let event = events.recv().await.unwrap();
        if event.outcome == expected {
            return event;
        }
    }
}

#[tokio::test]
async fn preflight_and_replay_are_bounded() {
    let provider =
        MediaAudioSystemServiceProvider::mock().with_admission_facts(AudioPreflightFacts {
            permission_granted: false,
            ..AudioPreflightFacts::permissive()
        });
    let mut events = provider.subscribe();
    let denied = ServiceCommand::with_trace(
        ServiceCommandName::new("audio.export_request"),
        serde_json::json!({}),
        TraceContext::new("audio-denied"),
    );
    assert!(matches!(
        provider.call(denied).await,
        Err(ServiceError::DisabledByPolicy(_))
    ));
    assert_eq!(events.recv().await.unwrap().outcome, "preflight_rejected");
    let snapshot = provider.snapshot().await;
    assert_eq!(snapshot["snapshot_schema"], "media.audio.replay.v1");
    assert_eq!(
        snapshot["command_count"],
        MEDIA_AUDIO_COMMANDS.len().to_string()
    );
    assert_eq!(events.recv().await.unwrap().outcome, "snapshot_recorded");
}

#[tokio::test]
async fn host_issued_rejections_never_complete_or_observe_command_payloads() {
    let rejected_facts = [
        AudioPreflightFacts {
            scope_granted: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            policy_granted: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            entitlement_granted: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            schema_valid: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            format_supported: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            codec_supported: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            metadata_allowed: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            voice_allowed: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            prompt_allowed: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            synthesis_allowed: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            export_allowed: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            write_allowed: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            artifact_allowed: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            requested_units: 2,
            reserved_units: 1,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            approval_required: true,
            approval_granted: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            within_timeout: false,
            ..AudioPreflightFacts::permissive()
        },
        AudioPreflightFacts {
            cancellation_requested: true,
            ..AudioPreflightFacts::permissive()
        },
    ];
    for (index, facts) in rejected_facts.into_iter().enumerate() {
        let provider = MediaAudioSystemServiceProvider::mock().with_admission_facts(facts);
        let mut events = provider.subscribe();
        let marker = "must-not-enter-audio-provider";
        assert!(provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("audio.export_request"),
                serde_json::json!({"raw_audio":marker,"raw_prompt":marker}),
                TraceContext::new(format!("audio-rejected-{index}")),
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
async fn side_effecting_requests_are_idempotent_and_stale_versions_do_not_complete() {
    let provider = MediaAudioSystemServiceProvider::mock();
    let mut first = ServiceCommand::with_trace(
        ServiceCommandName::new("audio.export_request"),
        serde_json::json!({}),
        TraceContext::new("audio-first"),
    );
    first
        .metadata
        .insert("idempotency_key".into(), "once".into());
    let first_ref = provider.call(first).await.unwrap().output["artifact_ref"].clone();
    let mut replay = ServiceCommand::with_trace(
        ServiceCommandName::new("audio.export_request"),
        serde_json::json!({}),
        TraceContext::new("audio-replay"),
    );
    replay
        .metadata
        .insert("idempotency_key".into(), "once".into());
    assert_eq!(
        provider.call(replay).await.unwrap().output["artifact_ref"],
        first_ref
    );

    let mut events = provider.subscribe();
    let mut stale = ServiceCommand::with_trace(
        ServiceCommandName::new("audio.transcode_request"),
        serde_json::json!({}),
        TraceContext::new("audio-stale"),
    );
    stale.metadata.insert("audio_version".into(), "v1".into());
    stale
        .metadata
        .insert("current_audio_version".into(), "v2".into());
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
async fn payload_policy_rejections_fail_closed_before_audio_side_effects() {
    let provider = MediaAudioSystemServiceProvider::mock();
    for (trace, payload) in [
        ("scope", serde_json::json!({"scope_denied": true})),
        ("format", serde_json::json!({"format_unsupported": true})),
        ("voice", serde_json::json!({"voice_denied": true})),
        ("prompt", serde_json::json!({"prompt_denied": true})),
        ("export", serde_json::json!({"export_denied": true})),
        ("approval", serde_json::json!({"approval_required": true})),
        ("timeout", serde_json::json!({"timeout": true})),
    ] {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("audio.export_request"),
                payload,
                TraceContext::new(trace),
            ))
            .await;
        assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    }
    assert_eq!(
        provider.snapshot().await["idempotency_completion_count"],
        "0"
    );
}

#[tokio::test]
async fn audio_results_events_and_snapshots_redact_all_sensitive_media_data() {
    let provider = MediaAudioSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let sensitive = serde_json::json!({
        "credential": "credential-marker",
        "raw_prompt": "prompt-marker",
        "private_recording": "recording-marker",
        "speaker_biometric": "biometric-marker",
        "generated_audio": "generated-audio-marker",
        "raw_export": "export-marker",
        "provider_payload": "provider-marker",
        "manifest": "manifest-marker",
        "package_bytes": "package-marker",
        "private_key": "private-key-marker",
        "signature": "signature-marker",
        "pcm_samples": "pcm-marker"
    });
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("audio.synthesis_request"),
            sensitive,
            TraceContext::new("audio-redaction"),
        ))
        .await
        .unwrap();
    let event = events.recv().await.unwrap();
    let snapshot = provider.snapshot().await;
    let snapshot_event = events.recv().await.unwrap();
    let observable = format!("{result:?}{event:?}{snapshot:?}{snapshot_event:?}");
    for marker in [
        "credential-marker",
        "prompt-marker",
        "recording-marker",
        "biometric-marker",
        "generated-audio-marker",
        "export-marker",
        "provider-marker",
        "manifest-marker",
        "package-marker",
        "private-key-marker",
        "signature-marker",
        "pcm-marker",
    ] {
        assert!(
            !observable.contains(marker),
            "sensitive marker leaked: {marker}"
        );
    }
}

#[tokio::test]
async fn audio_emits_stable_audit_taxonomy() {
    let provider = MediaAudioSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    provider.health().await.unwrap();
    for command in [
        "audio.inspect_provider",
        "audio.import_audio_request",
        "audio.inspect_metadata",
        "audio.inspect_waveform",
        "audio.plan_transcode",
        "audio.plan_segment",
        "audio.plan_filter",
        "audio.plan_mix",
        "audio.plan_synthesis",
        "audio.plan_export",
        "audio.get_artifact_handle",
    ] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({"pcm":"redacted","prompt":"redacted"}),
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
        "audio.pack_declared",
        "audio.health",
        "audio.admission",
        "audio.policy",
        "audio.entitlement",
        "audio.resource",
        "audio.approval",
        "audio.provider_inspection",
        "audio.import_open",
        "audio.metadata_inspection",
        "audio.waveform_inspection",
        "audio.transcode",
        "audio.segment",
        "audio.filter",
        "audio.mix",
        "audio.synthesis",
        "audio.export",
        "audio.artifact_handle",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
    }
}
