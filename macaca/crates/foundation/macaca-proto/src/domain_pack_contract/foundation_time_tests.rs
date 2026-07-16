use std::collections::{BTreeMap, BTreeSet};

use super::*;

// The time pack tests validate only descriptor and DTO contracts. They do not
// call host clocks, create timers, inspect timezone databases, or make the pack
// callable before a serviceized provider is installed.

#[test]
fn foundation_time_descriptor_is_discoverable_and_not_callable() {
    let definition = foundation_time_pack_definition();

    assert_eq!(definition.pack_id, FOUNDATION_TIME_PACK_ID);
    assert!(!definition.is_callable());
    assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
    assert_eq!(
        definition.metadata.parent_pack_id.as_deref(),
        Some("pack.foundation.v1")
    );
    assert_eq!(
        definition.metadata.diagnostics.unavailable_reason,
        "time_provider_not_installed"
    );
    assert!(definition
        .metadata
        .sdk
        .docs_url
        .contains("developer-packs/foundation/time"));

    let commands = definition
        .metadata
        .service_command_schemas
        .get(FOUNDATION_TIME_SERVICE_ID)
        .expect("time descriptor exposes command schemas");
    for command in FOUNDATION_TIME_COMMANDS {
        assert!(commands.contains(*command), "missing command {command}");
    }

    for scope in [
        "time.read",
        "time.monotonic",
        "time.timezone",
        "time.calendar",
        "time.format",
        "time.parse",
        "time.timer",
        "time.deadline",
    ] {
        assert!(
            definition.metadata.permission_scopes.contains(scope),
            "missing permission scope {scope}"
        );
    }

    for provider_class in ["host-clock", "frozen-test-clock", "mock", "unavailable"] {
        assert!(
            definition
                .metadata
                .provider_descriptors
                .contains_key(provider_class),
            "missing provider descriptor {provider_class}"
        );
    }
}

#[test]
fn industrial_catalog_uses_foundation_time_contract_descriptor() {
    let definition = industrial_reference_domain_pack_definitions()
        .into_iter()
        .find(|definition| definition.pack_id == FOUNDATION_TIME_PACK_ID)
        .expect("industrial catalog includes foundation time");

    assert!(!definition.is_callable());
    assert!(definition
        .metadata
        .service_command_schemas
        .contains_key(FOUNDATION_TIME_SERVICE_ID));
    assert_eq!(
        definition
            .metadata
            .provider_descriptors
            .get("frozen-test-clock")
            .and_then(|descriptor| descriptor.metadata.get("mock_clock"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn foundation_time_command_dtos_are_serde_compatible() {
    let timezone = TimeZoneReference {
        zone_id: "UTC".into(),
        data_version: "tzdb-2026a".into(),
    };
    let calendar = TimeCalendarReference {
        calendar_id: "iso8601".into(),
    };
    let locale = TimeLocaleReference {
        locale_id: "en-US".into(),
    };
    let instant = TimeInstant {
        epoch_millis: 1_800_000_000_000,
        timezone_id: "UTC".into(),
        calendar_id: "iso8601".into(),
    };
    let duration = TimeDuration {
        millis: 5_000,
        nanos_adjustment: 0,
    };
    let format = TimeFormatSpec {
        pattern_ref: "rfc3339".into(),
        locale,
        timezone: timezone.clone(),
    };
    let timer = TimeTimerReference {
        timer_id: "timer-ref".into(),
        session_binding: "session-ref".into(),
    };

    let commands = vec![
        serde_json::to_value(TimeNowCommand {
            timezone: Some(timezone.clone()),
            calendar: Some(calendar.clone()),
        })
        .unwrap(),
        serde_json::to_value(TimeMonotonicNowCommand {
            source: TimeClockSource::Monotonic,
        })
        .unwrap(),
        serde_json::to_value(TimeClockHealthCommand {
            include_timer_limits: true,
            include_timezone_data: true,
        })
        .unwrap(),
        serde_json::to_value(TimeDurationBetweenCommand {
            start: instant.clone(),
            end: instant.clone(),
        })
        .unwrap(),
        serde_json::to_value(TimeAddDurationCommand {
            instant: instant.clone(),
            duration: duration.clone(),
            overflow_policy: "deny".into(),
        })
        .unwrap(),
        serde_json::to_value(TimeConvertTimezoneCommand {
            instant: instant.clone(),
            target_timezone: timezone.clone(),
        })
        .unwrap(),
        serde_json::to_value(TimeResolveTimezoneCommand {
            zone_query: "UTC".into(),
            region_hint: Some("001".into()),
        })
        .unwrap(),
        serde_json::to_value(TimeCalendarConvertCommand {
            instant: instant.clone(),
            target_calendar: calendar,
        })
        .unwrap(),
        serde_json::to_value(TimeFormatCommand {
            instant: instant.clone(),
            format: format.clone(),
        })
        .unwrap(),
        serde_json::to_value(TimeParseCommand {
            input_ref: "artifact:timestamp".into(),
            format,
            strict: true,
        })
        .unwrap(),
        serde_json::to_value(TimeCreateTimerCommand {
            duration,
            exactness: TimeExactnessHint::InexactAllowed,
            session_binding: "session-ref".into(),
        })
        .unwrap(),
        serde_json::to_value(TimeCancelTimerCommand {
            timer: timer.clone(),
        })
        .unwrap(),
        serde_json::to_value(TimeInspectTimerCommand {
            timer: timer.clone(),
        })
        .unwrap(),
        serde_json::to_value(TimeEvaluateDeadlineCommand {
            deadline: TimeDeadlineSpec {
                deadline: instant,
                now_ref: None,
                exactness: TimeExactnessHint::ExactPreferred,
            },
        })
        .unwrap(),
    ];

    assert_eq!(commands.len(), FOUNDATION_TIME_COMMANDS.len());
    assert!(commands.iter().all(|value| value.is_object()));
}

#[test]
fn foundation_time_hashes_change_with_contract_content() {
    let request = TimeMonotonicNowCommand {
        source: TimeClockSource::Monotonic,
    };
    let changed = TimeMonotonicNowCommand {
        source: TimeClockSource::FrozenTest,
    };

    assert_eq!(time_stable_hash(&request), time_stable_hash(&request));
    assert_ne!(time_stable_hash(&request), time_stable_hash(&changed));

    let hashes = foundation_time_descriptor_hashes();
    let unique = BTreeSet::from([
        hashes.command_schema_hash,
        hashes.result_schema_hash,
        hashes.health_schema_hash,
        hashes.snapshot_schema_hash,
        hashes.provider_capability_schema_hash,
        hashes.unavailable_schema_hash,
    ]);
    assert_eq!(unique.len(), 6);
    assert!(unique.iter().all(|hash| !hash.is_empty()));
}

#[test]
fn foundation_time_result_and_snapshot_dtos_are_bounded() {
    let health = TimeClockHealth {
        provider_class: "unavailable".into(),
        wall_clock_available: false,
        monotonic_available: false,
        timezone_data_version: None,
        locale_data_available: false,
        max_timer_duration_ms: 0,
        unavailable_reason: Some("time_provider_not_installed".into()),
    };
    let snapshot = TimeProviderSnapshot {
        descriptor_hash: "descriptor-hash".into(),
        provider_class: "unavailable".into(),
        health: health.clone(),
        timer_state_hashes: BTreeMap::from([("timer-ref".into(), "timer-state-hash".into())]),
    };
    let unavailable: TimeResultEnvelope<String> = TimeResultEnvelope {
        status: TimeResultStatus::Unavailable,
        data: None,
        error: Some(TimeError {
            code: TimeResultStatus::Unavailable,
            message: "time provider is not installed".into(),
            retryable: false,
        }),
        trace_id: "trace-time-unavailable".into(),
        descriptor_hash: time_stable_hash(&snapshot),
    };

    let serialized = serde_json::to_string(&unavailable).unwrap();
    assert!(serialized.contains("trace-time-unavailable"));
    assert!(!serialized.contains("provider-private-timezone-payload"));
    assert!(!serialized.contains("raw-user-content"));
    assert_eq!(snapshot.health, health);
}
