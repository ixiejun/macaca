//! Conformance tests for the provider-neutral host lifecycle Strategy.

use super::device_host_lifecycle_service_provider::DeviceHostLifecycleSystemServiceProvider;
use macaca_kernel::SystemService;
use macaca_proto::device_foreground_background_host::DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS;
use macaca_proto::{
    HostLifecyclePreflightFacts, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext,
};

#[tokio::test]
async fn lifecycle_commands_are_traceable_and_redacted() {
    let provider = DeviceHostLifecycleSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS {
        let marker = "private-host-presentation-marker";
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"host_id":marker,"lifecycle_log":marker,"credentials":marker}),
                TraceContext::new(format!("lifecycle-{command}")),
            ))
            .await
            .unwrap();
        assert_eq!(result.output["status"], "reference_only");
        assert!(!result.output.to_string().contains(marker));
        assert_eq!(events.recv().await.unwrap().outcome, "completed");
    }
}

#[tokio::test]
async fn unavailable_lifecycle_provider_fails_closed_and_snapshot_is_bounded() {
    let provider = DeviceHostLifecycleSystemServiceProvider::unavailable("module_absent");
    let mut events = provider.subscribe();
    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    for command in DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS {
        assert!(matches!(
            provider
                .call(ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({"host_id":"must-not-open"}),
                    TraceContext::new(format!("unavailable-{command}"))
                ))
                .await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
        assert_eq!(events.recv().await.unwrap().outcome, "unavailable");
    }
    assert_eq!(
        provider.snapshot().await["snapshot_schema"],
        "device.host_lifecycle.replay.v1"
    );
    assert_eq!(events.recv().await.unwrap().outcome, "snapshot_recorded");
}

#[tokio::test]
async fn host_rejections_and_cleanup_do_not_leak_lifecycle_resources() {
    let provider = DeviceHostLifecycleSystemServiceProvider::mock().with_admission_facts(
        HostLifecyclePreflightFacts {
            throttled: true,
            ..HostLifecyclePreflightFacts::permissive()
        },
    );
    assert!(provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("host_lifecycle.request_background_lease"),
            serde_json::json!({"lifecycle_log":"must-not-observe"}),
            TraceContext::new("lifecycle-rejected")
        ))
        .await
        .is_err());
    let provider = DeviceHostLifecycleSystemServiceProvider::mock();
    for command in [
        "host_lifecycle.open_foreground_session",
        "host_lifecycle.request_background_lease",
    ] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({}),
                TraceContext::new(command),
            ))
            .await
            .unwrap();
    }
    let active = provider.snapshot().await;
    assert_eq!(active["active_foreground_session_count"], "1");
    assert_eq!(active["active_background_lease_count"], "1");
    provider.cleanup().await.unwrap();
    let released = provider.snapshot().await;
    assert_eq!(released["active_foreground_session_count"], "0");
    assert_eq!(released["active_background_lease_count"], "0");
}

#[tokio::test]
async fn lifecycle_results_events_and_snapshots_redact_sensitive_host_data() {
    let provider = DeviceHostLifecycleSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let sensitive = serde_json::json!({
        "provider_payload": "provider-marker",
        "host_id": "host-marker",
        "presentation_metadata": "presentation-marker",
        "lifecycle_log": "lifecycle-log-marker",
        "credential": "credential-marker",
        "session_id": "session-marker",
        "lease_id": "lease-marker",
        "snapshot": "snapshot-marker",
        "diagnostics": "diagnostics-marker"
    });
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("host_lifecycle.open_foreground_session"),
            sensitive,
            TraceContext::new("host-lifecycle-redaction"),
        ))
        .await
        .unwrap();
    let event = events.recv().await.unwrap();
    let snapshot = provider.snapshot().await;
    let snapshot_event = events.recv().await.unwrap();

    let observable = format!("{result:?}{event:?}{snapshot:?}{snapshot_event:?}");
    for marker in [
        "provider-marker",
        "host-marker",
        "presentation-marker",
        "lifecycle-log-marker",
        "credential-marker",
        "session-marker",
        "lease-marker",
        "snapshot-marker",
        "diagnostics-marker",
    ] {
        assert!(
            !observable.contains(marker),
            "sensitive marker leaked: {marker}"
        );
    }
}
