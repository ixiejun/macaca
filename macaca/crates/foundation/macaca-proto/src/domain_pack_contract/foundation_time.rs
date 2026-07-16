use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Stable pack id for the provider-neutral foundation time capability.
pub const FOUNDATION_TIME_PACK_ID: &str = "pack.foundation.time.v1";
/// Stable service id used by future time providers once a provider is installed.
pub const FOUNDATION_TIME_SERVICE_ID: &str = "service.foundation.time";

/// Canonical command names described by `pack.foundation.time.v1`.
///
/// The command list is descriptor data only. The pack stays preview-unavailable until a
/// serviceized time provider registers through the runtime composition root.
pub const FOUNDATION_TIME_COMMANDS: &[&str] = &[
    "time.now",
    "time.monotonic_now",
    "time.clock_health",
    "time.duration_between",
    "time.add_duration",
    "time.convert_timezone",
    "time.resolve_timezone",
    "time.calendar_convert",
    "time.format",
    "time.parse",
    "time.create_timer",
    "time.cancel_timer",
    "time.inspect_timer",
    "time.evaluate_deadline",
];

/// Build the descriptor-only catalog entry for `pack.foundation.time.v1`.
///
/// The descriptor exposes command/result schemas, policy defaults, SDK metadata,
/// provider-class descriptors, health metadata, and unavailable diagnostics without binding any
/// concrete host clock, timezone database, locale formatter, or timer runtime.
pub fn foundation_time_pack_definition() -> DomainPackDefinition {
    let command_schemas = schema_set(FOUNDATION_TIME_COMMANDS);
    let result_schemas = FOUNDATION_TIME_COMMANDS
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        FOUNDATION_TIME_PACK_ID,
        DomainPackMetadata {
            family_id: "foundation".into(),
            parent_pack_id: Some("pack.foundation.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            service_command_schemas: BTreeMap::from([(
                FOUNDATION_TIME_SERVICE_ID.into(),
                command_schemas,
            )]),
            service_result_schemas: BTreeMap::from([(
                FOUNDATION_TIME_SERVICE_ID.into(),
                result_schemas,
            )]),
            permission_scopes: schema_set(&[
                "time.read",
                "time.monotonic",
                "time.timezone",
                "time.calendar",
                "time.format",
                "time.parse",
                "time.timer",
                "time.deadline",
            ]),
            source_attribution: schema_set(&[
                "openspec:add-developer-pack-industrial-capability-catalog",
                "openspec:add-pack-foundation-time",
            ]),
            migration_notes: vec![
                "The time pack is discoverable as an industrial descriptor and becomes callable only after an approved time system service provider registers.".into(),
                "Provider-native clock, timer, timezone, or formatter handles must not cross the SDK or WASM boundary.".into(),
            ],
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(5_000),
                max_retries: Some(0),
                budget_units: Some(1),
                allow_network: Some(false),
            },
            data_governance: DomainPackDataGovernance {
                classification: "temporal_metadata".into(),
                retention_policy: "bounded_clock_and_timer_metadata_only".into(),
                redaction_policy: "provider_payloads_and_user_content_redacted".into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: "sdk.packs.foundation.time".into(),
                docs_url: "docs://macaca/developer-packs/foundation/time".into(),
                examples: vec![
                    "Declare `pack.foundation.time.v1` as optional until a time provider is installed.".into(),
                    "Use `time.clock_health` diagnostics to explain unavailable clock, timezone, locale, or timer support.".into(),
                ],
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: "time.clock_health".into(),
                unavailable_reason: "time_provider_not_installed".into(),
                replay_schema: "time.pack.replay.v1".into(),
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::from([(
                    FOUNDATION_TIME_SERVICE_ID.into(),
                    "^1".into(),
                )]),
            },
            provider_descriptors: time_provider_descriptors(),
        },
        [FOUNDATION_TIME_SERVICE_ID.to_string()],
    )
}

fn time_provider_descriptors() -> BTreeMap<String, DomainPackProviderDescriptor> {
    [
        provider_descriptor("host-clock", DomainPackProviderCapabilityState::Preview),
        provider_descriptor(
            "frozen-test-clock",
            DomainPackProviderCapabilityState::Preview,
        ),
        provider_descriptor("mock", DomainPackProviderCapabilityState::Preview),
        provider_descriptor(
            "unavailable",
            DomainPackProviderCapabilityState::Unavailable,
        ),
    ]
    .into_iter()
    .map(|descriptor| (descriptor.provider_class.clone(), descriptor))
    .collect()
}

fn provider_descriptor(
    provider_class: &str,
    availability: DomainPackProviderCapabilityState,
) -> DomainPackProviderDescriptor {
    let capability = TimeProviderCapability {
        provider_class: provider_class.into(),
        supported_commands: schema_set(FOUNDATION_TIME_COMMANDS),
        supported_clock_sources: BTreeSet::from([
            TimeClockSource::WallClock,
            TimeClockSource::Monotonic,
            TimeClockSource::FrozenTest,
        ]),
        supports_timezone_database: true,
        supports_locale_formatting: true,
        supports_exact_timers: provider_class != "unavailable",
        supports_mock_clock: provider_class == "frozen-test-clock" || provider_class == "mock",
        max_timer_duration_ms: 86_400_000,
        availability,
    };
    DomainPackProviderDescriptor {
        provider_class: provider_class.into(),
        service_id: FOUNDATION_TIME_SERVICE_ID.into(),
        availability,
        capability_hash: time_stable_hash(&capability),
        compatibility_hash: "foundation-time-provider-v1".into(),
        diagnostics_schema: "time.provider.diagnostics.v1".into(),
        metadata: BTreeMap::from([
            ("max_timer_duration_ms".into(), "86400000".into()),
            (
                "clock_sources".into(),
                "wall_clock,monotonic,frozen_test".into(),
            ),
            (
                "mock_clock".into(),
                capability.supports_mock_clock.to_string(),
            ),
        ]),
    }
}

fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Logical clock source requested by time commands and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeClockSource {
    WallClock,
    Monotonic,
    FrozenTest,
}

/// Timer exactness requested by scheduler-like time commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeExactnessHint {
    ExactRequired,
    ExactPreferred,
    InexactAllowed,
}

/// Provider-neutral instant value; providers choose the concrete clock adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeInstant {
    pub epoch_millis: i128,
    pub timezone_id: String,
    pub calendar_id: String,
}

/// Provider-neutral monotonic instant value for elapsed-time decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeMonotonicInstant {
    pub ticks: u128,
    pub unit: String,
    pub source_id: String,
}

/// Signed duration value shared by arithmetic, timers, and deadlines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeDuration {
    pub millis: i128,
    pub nanos_adjustment: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeZoneReference {
    pub zone_id: String,
    pub data_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeCalendarReference {
    pub calendar_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeLocaleReference {
    pub locale_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeFormatSpec {
    pub pattern_ref: String,
    pub locale: TimeLocaleReference,
    pub timezone: TimeZoneReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeTimerReference {
    pub timer_id: String,
    pub session_binding: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeDeadlineSpec {
    pub deadline: TimeInstant,
    pub now_ref: Option<TimeInstant>,
    pub exactness: TimeExactnessHint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeNowCommand {
    pub timezone: Option<TimeZoneReference>,
    pub calendar: Option<TimeCalendarReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeMonotonicNowCommand {
    pub source: TimeClockSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeClockHealthCommand {
    pub include_timer_limits: bool,
    pub include_timezone_data: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeDurationBetweenCommand {
    pub start: TimeInstant,
    pub end: TimeInstant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeAddDurationCommand {
    pub instant: TimeInstant,
    pub duration: TimeDuration,
    pub overflow_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeConvertTimezoneCommand {
    pub instant: TimeInstant,
    pub target_timezone: TimeZoneReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeResolveTimezoneCommand {
    pub zone_query: String,
    pub region_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeCalendarConvertCommand {
    pub instant: TimeInstant,
    pub target_calendar: TimeCalendarReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeFormatCommand {
    pub instant: TimeInstant,
    pub format: TimeFormatSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeParseCommand {
    pub input_ref: String,
    pub format: TimeFormatSpec,
    pub strict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeCreateTimerCommand {
    pub duration: TimeDuration,
    pub exactness: TimeExactnessHint,
    pub session_binding: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeCancelTimerCommand {
    pub timer: TimeTimerReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeInspectTimerCommand {
    pub timer: TimeTimerReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeEvaluateDeadlineCommand {
    pub deadline: TimeDeadlineSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeClockHealth {
    pub provider_class: String,
    pub wall_clock_available: bool,
    pub monotonic_available: bool,
    pub timezone_data_version: Option<String>,
    pub locale_data_available: bool,
    pub max_timer_duration_ms: u64,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supported_clock_sources: BTreeSet<TimeClockSource>,
    pub supports_timezone_database: bool,
    pub supports_locale_formatting: bool,
    pub supports_exact_timers: bool,
    pub supports_mock_clock: bool,
    pub max_timer_duration_ms: u64,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub health: TimeClockHealth,
    pub timer_state_hashes: BTreeMap<String, String>,
}

/// Normalized result status used by every time command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeResultStatus {
    Success,
    Denied,
    InvalidTime,
    InvalidTimezone,
    InvalidCalendar,
    InvalidLocale,
    ParseFailed,
    Overflow,
    Unsupported,
    TimerNotFound,
    QuotaExceeded,
    Unavailable,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeError {
    pub code: TimeResultStatus,
    pub message: String,
    pub retryable: bool,
}

/// Generic result envelope shared by time command outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeResultEnvelope<T> {
    pub status: TimeResultStatus,
    pub data: Option<T>,
    pub error: Option<TimeError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub health_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

/// Return deterministic hashes for the current time contract schema surface.
pub fn foundation_time_descriptor_hashes() -> TimeDescriptorHashes {
    let health = TimeClockHealth {
        provider_class: "unavailable".into(),
        wall_clock_available: false,
        monotonic_available: false,
        timezone_data_version: None,
        locale_data_available: false,
        max_timer_duration_ms: 0,
        unavailable_reason: Some("time_provider_not_installed".into()),
    };
    TimeDescriptorHashes {
        command_schema_hash: time_stable_hash(&FOUNDATION_TIME_COMMANDS),
        result_schema_hash: time_stable_hash(&TimeResultStatus::Success),
        health_schema_hash: time_stable_hash(&health),
        snapshot_schema_hash: time_stable_hash(&TimeProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            health: health.clone(),
            timer_state_hashes: BTreeMap::new(),
        }),
        provider_capability_schema_hash: time_stable_hash(&TimeProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: schema_set(FOUNDATION_TIME_COMMANDS),
            supported_clock_sources: BTreeSet::from([TimeClockSource::WallClock]),
            supports_timezone_database: false,
            supports_locale_formatting: false,
            supports_exact_timers: false,
            supports_mock_clock: false,
            max_timer_duration_ms: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
        }),
        unavailable_schema_hash: time_stable_hash(&TimeError {
            code: TimeResultStatus::Unavailable,
            message: "time provider is not installed".into(),
            retryable: false,
        }),
    }
}

/// Compute a deterministic, non-secret hash for descriptor and DTO compatibility tests.
///
/// This is not a security primitive. It is intentionally small and stable so tests, SDK
/// discovery, and audit metadata can compare schema DTOs without logging clock provider payloads,
/// timer internals, user content, credentials, or application data.
pub fn time_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}
