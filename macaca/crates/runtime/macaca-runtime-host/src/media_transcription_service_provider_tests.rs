//! Conformance tests for the metadata-only transcription service Strategy.

use macaca_kernel::SystemService;
use macaca_proto::media_transcription::MEDIA_TRANSCRIPTION_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext};

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
async fn unavailable_provider_fails_closed_without_audio_processing() {
    let provider = MediaTranscriptionSystemServiceProvider::unavailable("module_absent");
    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    let error = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("transcription.batch_request"),
            serde_json::json!({"raw_audio": "must-not-process"}),
            TraceContext::new("transcription-unavailable"),
        ))
        .await
        .unwrap_err();
    assert!(matches!(error, ServiceError::ServiceUnavailable(_)));
    assert_eq!(provider.capability().provider_class, "unavailable");
}

#[tokio::test]
async fn unknown_command_is_structured_unsupported() {
    let provider = MediaTranscriptionSystemServiceProvider::mock();
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
