//! Contract tests for host and frozen time providers.

use crate::{FrozenTimeProvider, HostTimeProvider, TimeService};
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new("trace-time"),
    )
}

#[tokio::test]
async fn frozen_clock_is_deterministic_and_timer_lifecycle_is_bounded() {
    let provider = FrozenTimeProvider::new(1_000);
    assert_eq!(
        provider
            .call(command("time.now", serde_json::json!({})))
            .await
            .unwrap()
            .output["epoch_millis"],
        1_000
    );
    let created = provider
        .call(command(
            "time.create_timer",
            serde_json::json!({"duration":{"millis":10},"exactness":"inexact_allowed"}),
        ))
        .await
        .unwrap();
    let id = created.output["timer_id"].clone();
    assert_eq!(
        provider
            .call(command(
                "time.cancel_timer",
                serde_json::json!({"timer":{"timer_id":id}})
            ))
            .await
            .unwrap()
            .output["state"],
        "cancelled"
    );
}

#[tokio::test]
async fn host_provider_rejects_invalid_timer_without_fallback() {
    let error = HostTimeProvider::default()
        .call(command(
            "time.create_timer",
            serde_json::json!({"duration":{"millis":0}}),
        ))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("duration"));
}

#[tokio::test]
async fn host_provider_supports_bounded_format_parse_and_fixed_timezone_commands() {
    let provider = HostTimeProvider::default();
    let parsed = provider
        .call(command(
            "time.parse",
            serde_json::json!({"input_ref":"2024-01-01T00:00:00Z"}),
        ))
        .await
        .unwrap();
    assert_eq!(parsed.output["status"], "success");
    let formatted = provider.call(command("time.format", serde_json::json!({"instant":{"epoch_millis":0},"format":{"pattern_ref":"format:rfc3339"}}))).await.unwrap();
    assert!(formatted.output["formatted"]
        .as_str()
        .unwrap()
        .contains("1970"));
    let zone = provider
        .call(command(
            "time.resolve_timezone",
            serde_json::json!({"zone_query":"UTC+08:00"}),
        ))
        .await
        .unwrap();
    assert_eq!(zone.output["offset_seconds"], 28_800);
}

#[tokio::test]
async fn snapshots_hash_timer_ids_and_shutdown_releases_timer_state() {
    let provider = FrozenTimeProvider::new(1_000);
    provider
        .call(command(
            "time.create_timer",
            serde_json::json!({"duration":{"millis":10}}),
        ))
        .await
        .unwrap();
    let snapshot = provider.snapshot();
    assert_eq!(snapshot.timer_state_hashes.len(), 1);
    assert!(snapshot
        .timer_state_hashes
        .keys()
        .all(|key| !key.contains("timer-1")));
    provider.shutdown().await.unwrap();
    assert!(provider.snapshot().timer_state_hashes.is_empty());
}

#[tokio::test]
async fn fired_timer_releases_its_reservation_from_the_ledger() {
    let provider = FrozenTimeProvider::new(1_000);
    let created = provider
        .call(command(
            "time.create_timer",
            serde_json::json!({"duration":{"millis":10}}),
        ))
        .await
        .unwrap();
    let timer_id = created.output["timer_id"].clone();
    assert_eq!(provider.snapshot().timer_state_hashes.len(), 1);
    provider.advance_millis(10).unwrap();
    let inspected = provider
        .call(command(
            "time.inspect_timer",
            serde_json::json!({"timer":{"timer_id":timer_id}}),
        ))
        .await
        .unwrap();
    assert_eq!(inspected.output["state"], "fired");
    assert!(provider.snapshot().timer_state_hashes.is_empty());
}

#[tokio::test]
async fn every_declared_command_is_trace_addressable_and_never_echoes_parse_input() {
    let provider = FrozenTimeProvider::new(1_000);
    let cases = [
        ("time.now", serde_json::json!({})),
        ("time.monotonic_now", serde_json::json!({})),
        ("time.clock_health", serde_json::json!({})),
        (
            "time.duration_between",
            serde_json::json!({"start":{"epoch_millis":1},"end":{"epoch_millis":2}}),
        ),
        (
            "time.add_duration",
            serde_json::json!({"instant":{"epoch_millis":1},"duration":{"millis":2}}),
        ),
        (
            "time.convert_timezone",
            serde_json::json!({"instant":{"epoch_millis":1},"target_timezone":{"zone_id":"UTC"}}),
        ),
        (
            "time.resolve_timezone",
            serde_json::json!({"zone_query":"UTC"}),
        ),
        (
            "time.calendar_convert",
            serde_json::json!({"target_calendar":{"calendar_id":"iso8601"}}),
        ),
        (
            "time.format",
            serde_json::json!({"instant":{"epoch_millis":0},"format":{"pattern_ref":"format:rfc3339"}}),
        ),
        (
            "time.parse",
            serde_json::json!({"input_ref":"2024-01-01T00:00:00Z"}),
        ),
        (
            "time.create_timer",
            serde_json::json!({"duration":{"millis":10}}),
        ),
        (
            "time.cancel_timer",
            serde_json::json!({"timer":{"timer_id":"timer-unknown"}}),
        ),
        (
            "time.inspect_timer",
            serde_json::json!({"timer":{"timer_id":"timer-unknown"}}),
        ),
        (
            "time.evaluate_deadline",
            serde_json::json!({"deadline":{"deadline":{"epoch_millis":1}}}),
        ),
    ];
    for (name, payload) in cases {
        let trace_id = format!("trace-replay-{name}");
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(name),
                payload,
                TraceContext::new(trace_id.clone()),
            ))
            .await
            .unwrap();
        assert_eq!(result.trace.trace_id.as_str(), trace_id);
        assert!(!result.output.to_string().contains("2024-01-01T00:00:00Z"));
    }
}
