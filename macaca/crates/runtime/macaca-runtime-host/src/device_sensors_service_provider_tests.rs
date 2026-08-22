use macaca_kernel::SystemService;
use macaca_proto::device_sensors::DEVICE_SENSORS_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext};

use super::device_sensors_service_provider::DeviceSensorsSystemServiceProvider;

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
