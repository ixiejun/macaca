//! Conformance tests for the metadata-only audio service Strategy.

use macaca_kernel::SystemService;
use macaca_proto::media_audio::MEDIA_AUDIO_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext};

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
        assert_eq!(events.recv().await.unwrap().outcome, "completed");
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
        assert_eq!(events.recv().await.unwrap().outcome, "unavailable");
    }
}

#[tokio::test]
async fn preflight_and_replay_are_bounded() {
    let provider = MediaAudioSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let mut denied = ServiceCommand::with_trace(
        ServiceCommandName::new("audio.export_request"),
        serde_json::json!({}),
        TraceContext::new("audio-denied"),
    );
    denied
        .metadata
        .insert("permission_granted".into(), "false".into());
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
