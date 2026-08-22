use macaca_kernel::SystemService;
use macaca_proto::device_sensors::DEVICE_SENSORS_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext};

use super::device_sensors_service_provider::{
    transition_sensor_lease, DeviceSensorsSystemServiceProvider, SensorLeaseState,
};

#[tokio::test]
async fn sensor_commands_are_reference_only_and_redacted() {
    let provider = DeviceSensorsSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in DEVICE_SENSORS_COMMANDS {
        let marker = "raw-sample-calibration-hardware-marker";
        let result = provider.call(ServiceCommand::with_trace(ServiceCommandName::new(*command), serde_json::json!({"sample_vector":marker,"hardware_id":marker,"calibration":marker}), TraceContext::new(format!("sensor-{command}")))).await.unwrap();
        assert_eq!(result.output["status"], "reference_only");
        assert!(!result.output.to_string().contains(marker));
        assert!(!format!("{:?}", events.recv().await.unwrap()).contains(marker));
    }
}

#[tokio::test]
async fn unavailable_sensor_provider_fails_closed_and_cleanup_releases_state() {
    let unavailable = DeviceSensorsSystemServiceProvider::unavailable("module_absent");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("sensors.read"),
                serde_json::json!({"sample_vector":"must-not-read"}),
                TraceContext::new("unavailable")
            ))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = DeviceSensorsSystemServiceProvider::mock();
    for command in ["sensors.open_stream", "sensors.acquire_lease"] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({}),
                TraceContext::new(command),
            ))
            .await
            .unwrap();
    }
    assert_eq!(provider.snapshot().await["active_stream_count"], "1");
    assert_eq!(provider.snapshot().await["active_lease_count"], "1");
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_stream_count"], "0");
}

#[tokio::test]
async fn sensor_replay_reference_is_stable_after_provider_restart() {
    let trace_id = "sensors-restart-trace";
    let first = DeviceSensorsSystemServiceProvider::mock();
    first
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("sensors.inspect_host"),
            serde_json::json!({"sample_vector":"must-not-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    first.cleanup().await.unwrap();
    let restarted = DeviceSensorsSystemServiceProvider::mock();
    let mut events = restarted.subscribe();
    restarted
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("sensors.inspect_host"),
            serde_json::json!({"hardware_id":"must-not-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    let event = events.recv().await.unwrap();
    assert_eq!(event.trace_id, trace_id);
    assert_eq!(event.replay_ref, format!("replay:sensors:{trace_id}"));
}

#[tokio::test]
async fn sensors_emit_stable_audit_event_taxonomy() {
    let provider = DeviceSensorsSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    for command in [
        "sensors.open_stream",
        "sensors.read_stream",
        "sensors.release_lease",
    ] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({"sample_vector":"redacted"}),
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
        "sensors.pack_declared",
        "sensors.admission_validated",
        "sensors.policy_decision",
        "sensors.entitlement_checked",
        "sensors.resource_reserved",
        "sensors.command_requested",
        "sensors.provider_selected",
        "sensors.stream_opened",
        "sensors.stream_chunk_delivered",
        "sensors.lease_revoked",
        "sensors.command_succeeded",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
    }
}

#[tokio::test]
async fn sensors_admission_denies_policy_facts_before_stream_allocation() {
    let provider = DeviceSensorsSystemServiceProvider::mock();
    for (trace, payload) in [
        ("permission", serde_json::json!({"permission_denied": true})),
        (
            "foreground",
            serde_json::json!({"foreground_required": true}),
        ),
        ("background", serde_json::json!({"background_denied": true})),
        ("sensitive", serde_json::json!({"sensitive_sensor": true})),
        ("frequency", serde_json::json!({"frequency_hz": 101})),
        ("samples", serde_json::json!({"sample_count": 1025})),
    ] {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("sensors.open_stream"),
                payload,
                TraceContext::new(trace),
            ))
            .await;
        assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    }
    assert_eq!(provider.snapshot().await["active_stream_count"], "0");
}

#[tokio::test]
async fn sensors_bound_streams_and_fail_closed_on_cancellation() {
    let provider = DeviceSensorsSystemServiceProvider::mock();
    for index in 0..32 {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("sensors.open_stream"),
                serde_json::json!({}),
                TraceContext::new(format!("stream-{index}")),
            ))
            .await
            .unwrap();
    }
    let bounded = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("sensors.open_stream"),
            serde_json::json!({}),
            TraceContext::new("stream-overflow"),
        ))
        .await;
    assert!(matches!(bounded, Err(ServiceError::DisabledByPolicy(_))));
    let cancelled = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("sensors.read_stream"),
            serde_json::json!({"cancelled": true}),
            TraceContext::new("cancelled"),
        ))
        .await;
    assert!(matches!(cancelled, Err(ServiceError::DisabledByPolicy(_))));
}

#[test]
fn sensor_lease_transitions_fail_closed() {
    assert_eq!(
        transition_sensor_lease(SensorLeaseState::Requested, "open"),
        Some(SensorLeaseState::Active)
    );
    assert_eq!(
        transition_sensor_lease(SensorLeaseState::Active, "drain"),
        Some(SensorLeaseState::Draining)
    );
    assert_eq!(
        transition_sensor_lease(SensorLeaseState::Draining, "close"),
        Some(SensorLeaseState::Closed)
    );
    assert_eq!(
        transition_sensor_lease(SensorLeaseState::Closed, "open"),
        None
    );
}
