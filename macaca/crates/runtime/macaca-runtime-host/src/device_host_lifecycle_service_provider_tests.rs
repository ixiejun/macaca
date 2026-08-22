//! Conformance tests for the provider-neutral host lifecycle Strategy.

use super::device_host_lifecycle_service_provider::DeviceHostLifecycleSystemServiceProvider;
use macaca_kernel::SystemService;
use macaca_proto::device_foreground_background_host::DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS;
use macaca_proto::{
    HostLifecyclePreflightFacts, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext,
};

#[test]
fn every_lifecycle_command_has_a_specific_audit_event() {
    let expected = [
        (
            "host_lifecycle.inspect_state",
            "host_lifecycle.state_inspected",
        ),
        (
            "host_lifecycle.subscribe_events",
            "host_lifecycle.events_subscribed",
        ),
        (
            "host_lifecycle.open_foreground_session",
            "host_lifecycle.foreground_session_opened",
        ),
        (
            "host_lifecycle.close_foreground_session",
            "host_lifecycle.foreground_session_closed",
        ),
        (
            "host_lifecycle.request_background_lease",
            "host_lifecycle.background_lease_requested",
        ),
        (
            "host_lifecycle.release_background_lease",
            "host_lifecycle.background_lease_released",
        ),
        (
            "host_lifecycle.inspect_policy",
            "host_lifecycle.policy_inspected",
        ),
        (
            "host_lifecycle.revoke",
            "host_lifecycle.session_or_lease_revoked",
        ),
        (
            "host_lifecycle.inspect_host",
            "host_lifecycle.host_inspected",
        ),
    ];
    assert_eq!(
        expected.len(),
        DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS.len()
    );
    for command in DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS {
        assert!(expected.iter().any(|(known, _)| known == command));
    }
}

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
        assert_eq!(
            receive_outcome(&mut events, "completed").await.outcome,
            "completed"
        );
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

#[tokio::test]
async fn lifecycle_replay_reference_is_stable_after_provider_restart() {
    let trace_id = "host-lifecycle-restart-trace";
    let first = DeviceHostLifecycleSystemServiceProvider::mock();
    first
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("host_lifecycle.inspect_host"),
            serde_json::json!({"host_id":"must-not-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    first.cleanup().await.unwrap();

    // Recreate the provider to model a host refresh while retaining only the
    // deterministic trace-derived replay address.
    let restarted = DeviceHostLifecycleSystemServiceProvider::mock();
    let mut events = restarted.subscribe();
    restarted
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("host_lifecycle.inspect_host"),
            serde_json::json!({"lifecycle_log":"must-not-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    let event = events.recv().await.unwrap();
    assert_eq!(event.trace_id, trace_id);
    assert_eq!(
        event.replay_ref,
        format!("replay:host-lifecycle:{trace_id}")
    );
}

#[tokio::test]
async fn lifecycle_emits_stable_pack_admission_and_state_events() {
    let provider = DeviceHostLifecycleSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    for command in [
        "host_lifecycle.open_foreground_session",
        "host_lifecycle.request_background_lease",
        "host_lifecycle.inspect_policy",
        "host_lifecycle.revoke",
    ] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({"host_id":"redacted"}),
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
        "host_lifecycle.pack_declared",
        "host_lifecycle.admission_validated",
        "host_lifecycle.policy_decision",
        "host_lifecycle.state_changed",
        "host_lifecycle.background_lease_granted",
        "host_lifecycle.throttle_changed",
        "host_lifecycle.session_or_lease_revoked",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
    }
}

async fn receive_outcome(
    events: &mut tokio::sync::broadcast::Receiver<
        super::device_host_lifecycle_service_provider::HostLifecycleRuntimeEvent,
    >,
    expected: &str,
) -> super::device_host_lifecycle_service_provider::HostLifecycleRuntimeEvent {
    loop {
        let event = events.recv().await.unwrap();
        if event.outcome == expected {
            return event;
        }
    }
}
