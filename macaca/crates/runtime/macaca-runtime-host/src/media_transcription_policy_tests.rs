use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

use super::media_transcription_service_provider::MediaTranscriptionSystemServiceProvider;

#[tokio::test]
async fn transcription_policy_and_resource_bounds_fail_before_side_effects() {
    let provider = MediaTranscriptionSystemServiceProvider::mock();
    for (command, payload, reason) in [
        (
            "transcription.batch_request",
            serde_json::json!({"duration_seconds": 86_401}),
            "transcription_duration_limit",
        ),
        (
            "transcription.start_stream",
            serde_json::json!({"external_delivery": true}),
            "transcription_external_delivery_approval",
        ),
        (
            "transcription.append_stream_chunk",
            serde_json::json!({"chunk_bytes": 1_048_577}),
            "transcription_chunk_size_limit",
        ),
    ] {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                payload,
                TraceContext::new(reason),
            ))
            .await;
        assert!(result.unwrap_err().to_string().contains(reason));
    }
    assert_eq!(
        provider.snapshot().await["idempotency_completion_count"],
        "0"
    );
}
